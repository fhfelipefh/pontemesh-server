import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const projectName = "ponte-mesh-e2e-origin-replica";
const composeFile = "docker/docker-compose.e2e-origin-replica.yml";
const originUrl = "http://127.0.0.1:18080";
const originS3Url = "http://127.0.0.1:19000";
const replicaUrl = "http://127.0.0.1:18081";
const replicaS3Url = "http://127.0.0.1:19001";
const bucket = "tcc-e2e";
const objectKey = "hello-origin-replica.txt";
const objectContent = "Ponte Mesh Origin Replica E2E";
const adminPassword = "TccE2eAdmin123!";
const s3AccessKeyId = "PMKORIGINE2EACCESSKEY";
const s3SecretAccessKey = "pm-origin-e2e-secret-material-123";
const region = "us-east-1";
const summaryPath = path.resolve("test-results/origin-replica-flow-summary.json");

test.describe.configure({ mode: "serial", timeout: 180_000 });

test("validates real Origin + Replica/Edge hybrid flow", async ({ page, request }) => {
  await fs.promises.mkdir(path.dirname(summaryPath), { recursive: true });

  const originToken = readInitialAdminToken("origin-server");
  await completeInitialSetup(page, originUrl, originToken, {
    instanceName: "Ponte Mesh Origin E2E",
    role: "origin",
    username: "origin-admin",
    password: adminPassword
  });
  await login(page, originUrl, "origin-admin", adminPassword);
  await expectDashboardRole(page, "origin");

  const originAdmin = await loginRequest(request, originUrl, "origin-admin", adminPassword);

  await createBucketViaS3(request, bucket);
  await putObjectViaS3(request, bucket, objectKey, objectContent, "text/plain");
  await updateBucketPolicy(originAdmin);
  await expectPolicyPersisted(originAdmin);

  const replicaCredential = await createReplicaCredentialViaUi(page);
  expect(replicaCredential.token).toBeTruthy();
  await expectReplicaSecretNotListed(page, replicaCredential.token);

  const replicaToken = readInitialAdminToken("replica-server");
  await completeInitialSetup(page, replicaUrl, replicaToken, {
    instanceName: "Ponte Mesh Replica E2E",
    role: "replica-edge",
    username: "replica-admin",
    password: adminPassword,
    originBaseUrl: "http://origin-server:8080",
    replicaPublicEndpoint: replicaUrl,
    replicaId: replicaCredential.replicaId,
    replicaToken: replicaCredential.token
  });
  await restartService("replica-server");
  await waitForSetupStatus(request, replicaUrl, false);
  await login(page, replicaUrl, "replica-admin", adminPassword);
  await expectDashboardRole(page, "replica-edge");
  const replicaAdmin = await loginRequest(request, replicaUrl, "replica-admin", adminPassword);

  await expectReplicaConnected(originAdmin, replicaCredential.replicaId);

  const appCredential = await createApplicationCredential(originAdmin);
  const appAuth = { Authorization: `Bearer ${appCredential.token}` };

  const manifest = await apiJson<ManifestResponse>(request, `${originUrl}/pontemesh/objects/${bucket}/manifest/${objectKey}`, {
    headers: appAuth
  });
  expect(manifest.fragments.length).toBeGreaterThanOrEqual(1);

  await expect.poll(async () => {
    const sources = await apiJson<SourcesResponse>(request, `${originUrl}/pontemesh/objects/${bucket}/sources/${objectKey}`, {
      headers: appAuth
    });
    return sources.authorizedSources.some((source: AuthorizedSource) => source.sourceType === "REPLICA_EDGE");
  }, { timeout: 60_000 }).toBe(true);

  const packageBody = await apiJson<AccessPackageResponse>(request, `${originUrl}/pontemesh/access-packages`, {
    method: "POST",
    headers: { ...appAuth, "content-type": "application/json" },
    data: { bucket, key: objectKey, ttlSeconds: 120 }
  });
  const sources = await apiJson<SourcesResponse>(request, `${originUrl}/pontemesh/objects/${bucket}/sources/${objectKey}`, {
    headers: appAuth
  });
  const availability = await apiJson<AvailabilityResponse>(request, `${originUrl}/pontemesh/objects/${bucket}/availability/${objectKey}`, {
    headers: appAuth
  });
  const objectPolicy = await apiJson<ObjectPolicyResponse>(request, `${originUrl}/pontemesh/objects/${bucket}/policies/${objectKey}`, {
    headers: appAuth
  });

  const originSource = packageBody.authorizedSources.find((source: AuthorizedSource) => source.sourceType === "ORIGIN");
  const replicaSource = packageBody.authorizedSources.find((source: AuthorizedSource) => source.sourceType === "REPLICA_EDGE");
  expect(originSource, "origin source").toBeTruthy();
  expect(replicaSource, "replica source").toBeTruthy();
  const originEndpoint = required(originSource).endpoint;
  const replicaEndpoint = required(replicaSource).endpoint;
  expect(originEndpoint).toContain("127.0.0.1:19000");
  expect(replicaEndpoint).toContain("127.0.0.1:18081");
  expect(replicaEndpoint).toContain(`/pontemesh/replica/access-packages/${packageBody.id}/objects/${bucket}/${objectKey}`);
  expect(sources.authorizedSources.some((source: AuthorizedSource) => source.sourceType === "ORIGIN")).toBe(true);
  expect(sources.authorizedSources.some((source: AuthorizedSource) => source.sourceType === "REPLICA_EDGE")).toBe(true);
  expect(availability.replicaSources).toBeGreaterThanOrEqual(1);
  expect(availability.fragments[0].availableSourceTypes).toContain("REPLICA_EDGE");
  expect(objectPolicy.fallbackSupportsRange).toBe(true);
  expect(objectPolicy.preserveValidatedFragments).toBe(true);

  const originEndpointReachable = await endpointReachable(request, originEndpoint);
  const replicaEndpointReachable = await endpointReachable(request, replicaS3Url);
  expect(originEndpointReachable).toBe(true);
  expect(replicaEndpointReachable).toBe(true);

  const fullObject = await apiText(request, `${originUrl}/pontemesh/access-packages/${packageBody.id}/objects/${bucket}/${objectKey}`, {
    headers: { Authorization: `Bearer ${packageBody.packageToken}` }
  });
  expect(fullObject).toBe(objectContent);
  const rangeObject = await apiText(request, `${originUrl}/pontemesh/access-packages/${packageBody.id}/objects/${bucket}/${objectKey}`, {
    headers: { Authorization: `Bearer ${packageBody.packageToken}`, Range: "bytes=0-9" },
    expectedStatus: 206
  });
  expect(rangeObject).toBe(objectContent.slice(0, 10));

  const replicaFull = await request.get(replicaEndpoint, {
    headers: { Authorization: `Bearer ${packageBody.packageToken}` }
  });
  expect(replicaFull.status()).toBe(200);
  expect(replicaFull.headers()["x-pontemesh-source"]).toBe("replica-edge");
  expect(replicaFull.headers()["x-pontemesh-package-id"]).toBe(packageBody.id);
  const replicaFullText = await replicaFull.text();
  expect(replicaFullText).toBe(objectContent);

  const replicaRange = await request.get(replicaEndpoint, {
    headers: { Authorization: `Bearer ${packageBody.packageToken}`, Range: "bytes=0-9" }
  });
  expect(replicaRange.status()).toBe(206);
  expect(replicaRange.headers()["content-range"]).toBe(`bytes 0-9/${objectContent.length}`);
  const replicaRangeText = await replicaRange.text();
  expect(replicaRangeText).toBe(objectContent.slice(0, 10));

  const replicaMissingToken = await request.get(replicaEndpoint);
  expect(replicaMissingToken.status()).toBe(401);
  const replicaInvalidToken = await request.get(replicaEndpoint, {
    headers: { Authorization: "Bearer invalid-package-token" }
  });
  expect([401, 403]).toContain(replicaInvalidToken.status());

  await postSdkFragmentEvent(request, packageBody, manifest);
  await revokeAccessPackage(originAdmin, packageBody.id);
  const replicaRevokedPackage = await request.get(replicaEndpoint, {
    headers: { Authorization: `Bearer ${packageBody.packageToken}` }
  });
  expect(replicaRevokedPackage.status()).toBe(403);
  await expectAuditEvents(originAdmin);
  await expectReplicaServingAuditEvents(replicaAdmin);

  const summary = {
    originUrl,
    originS3Url,
    replicaUrl,
    replicaS3Url,
    bucket,
    objectKey,
    originSourceFound: Boolean(originSource),
    replicaSourceFound: Boolean(replicaSource),
    manifestFragments: manifest.fragments.length,
    originEndpointReachable,
    replicaEndpointReachable,
    fallbackSupportsRange: packageBody.fallback.supportsRange,
    preserveValidatedFragments: packageBody.fallback.preserveValidatedFragments,
    replicaDirectObjectServing: "ready",
    replicaGetStatus: replicaFull.status(),
    replicaRangeStatus: replicaRange.status(),
    replicaReturnedExpectedContent: replicaFullText === objectContent,
    replicaReturnedExpectedRange: replicaRangeText === objectContent.slice(0, 10),
    replicaDeniedMissingToken: replicaMissingToken.status() === 401,
    replicaDeniedInvalidToken: [401, 403].includes(replicaInvalidToken.status()),
    replicaDeniedRevokedPackage: replicaRevokedPackage.status() === 403
  };
  await fs.promises.writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, "utf8");
});

type SetupOptions = {
  instanceName: string;
  role: "origin" | "replica-edge";
  username: string;
  password: string;
  originBaseUrl?: string;
  replicaPublicEndpoint?: string;
  replicaId?: string;
  replicaToken?: string;
};

type AuthorizedSource = {
  id: string;
  sourceType: string;
  endpoint: string;
};

type ManifestFragment = {
  index: number;
  sha256: string;
  sizeBytes: number;
};

type ManifestResponse = {
  fragments: ManifestFragment[];
};

type AccessPackageResponse = {
  id: string;
  packageToken: string;
  authorizedSources: AuthorizedSource[];
  fallback: {
    supportsRange: boolean;
    preserveValidatedFragments: boolean;
  };
};

type SourcesResponse = {
  authorizedSources: AuthorizedSource[];
};

type AvailabilityResponse = {
  replicaSources: number;
  fragments: Array<{
    availableSourceTypes: string[];
  }>;
};

type ObjectPolicyResponse = {
  fallbackSupportsRange: boolean;
  preserveValidatedFragments: boolean;
};

type ReplicaSummary = {
  id: string;
  healthStatus: string | null;
  availableObjects: number;
};

type ApplicationCredentialResponse = {
  token: string;
};

type AuditEvent = {
  event: string;
};

async function completeInitialSetup(page: Page, baseUrl: string, token: string, options: SetupOptions) {
  await page.goto(baseUrl);
  await page.locator("#token").fill(token);
  await page.getByRole("button", { name: /continue|continuar/i }).click();
  await page.waitForURL("**/setup/configure");
  await page.locator("#instanceName").fill(options.instanceName);
  await page.locator("#role").selectOption(options.role);
  await page.locator("#adminUsername").fill(options.username);
  await page.locator("#adminPassword").fill(options.password);
  await page.locator("#httpPort").fill("8080");
  if (options.role === "replica-edge") {
    await page.locator("#originBaseUrl").fill(required(options.originBaseUrl));
    await page.locator("#replicaPublicEndpoint").fill(required(options.replicaPublicEndpoint));
    await page.locator("#replicaId").fill(required(options.replicaId));
    await page.locator("#replicaToken").fill(required(options.replicaToken));
  }
  await page.getByRole("button", { name: /finish|concluir|complete|finalizar/i }).click();
  await page.getByRole("button", { name: /login|entrar/i }).click();
  await page.waitForURL("**/login");
}

async function login(page: Page, baseUrl: string, username: string, password: string) {
  await page.goto(`${baseUrl}/login`);
  await page.locator("#loginUsername").fill(username);
  await page.locator("#loginPassword").fill(password);
  await page.getByRole("button", { name: /sign in|entrar/i }).click();
  await page.waitForURL("**/dashboard");
}

async function expectDashboardRole(page: Page, role: "origin" | "replica-edge") {
  const summary = await page.evaluate(async () => {
    const response = await fetch("/api/admin/dashboard/summary", { headers: { accept: "application/json" } });
    if (!response.ok) {
      throw new Error(`dashboard summary failed: ${response.status}`);
    }
    return response.json();
  });
  expect(summary.instance.role).toBe(role);
}

async function createReplicaCredentialViaUi(page: Page): Promise<{ replicaId: string; token: string }> {
  await page.goto(`${originUrl}/replicas`);
  await page.getByLabel(/replica name|nome da replica|nome da réplica/i).fill("edge-tcc-e2e");
  await page.getByLabel(/allowed buckets|buckets permitidos/i).fill(bucket);
  await page.getByRole("button", { name: /create|criar/i }).click();
  const panel = page.locator(".secret-panel");
  await expect(panel).toBeVisible();
  const replicaId = (await panel.locator("dd code").nth(0).innerText()).trim();
  const token = (await panel.locator("dd code").nth(1).innerText()).trim();
  expect(replicaId).toBeTruthy();
  expect(token).toBeTruthy();
  return { replicaId, token };
}

async function expectReplicaSecretNotListed(page: Page, token: string) {
  await page.reload();
  await expect(page.locator("table")).toContainText("edge-tcc-e2e");
  await expect(page.locator("table")).not.toContainText(token);
}

async function loginRequest(request: APIRequestContext, baseUrl: string, username: string, password: string) {
  const response = await request.post(`${baseUrl}/api/auth/login`, {
    data: { username, password }
  });
  expect(response.ok()).toBe(true);
  const cookie = response.headers()["set-cookie"]?.split(";")[0];
  expect(cookie).toBeTruthy();
  return {
    baseUrl,
    cookie: required(cookie)
  };
}

async function createBucketViaS3(request: APIRequestContext, name: string) {
  const response = await signedS3Fetch(request, "PUT", `/${name}`);
  expect(response.ok() || response.status() === 400).toBe(true);
}

async function putObjectViaS3(
  request: APIRequestContext,
  bucketName: string,
  key: string,
  body: string,
  contentType: string
) {
  const response = await signedS3Fetch(request, "PUT", `/${bucketName}/${key}`, body, {
    "content-type": contentType
  });
  expect(response.ok()).toBe(true);
}

async function updateBucketPolicy(admin: { baseUrl: string; cookie: string }) {
  const policy = {
    accessPackageTtlSeconds: 300,
    fragmentSizeBytes: 1024,
    allowReplicaEdge: true,
    allowPeerSharing: false,
    sourceSelectionStrategy: "ORIGIN_REPLICA_EDGE",
    fragmentPriorityStrategy: "INITIAL_FIRST",
    failureThreshold: 3,
    fallbackMode: "ORIGIN_RANGE",
    s3ListDefaultMaxKeys: 1000,
    s3ListMaxKeysLimit: 10000,
    s3ListAllowDelimiter: true,
    s3VersioningEnabled: false,
    s3ObjectTaggingEnabled: true,
    s3ChecksumAlgorithm: "SHA256",
    s3MultipartAbortDays: 7
  };
  const response = await fetchJson(admin, `/api/admin/buckets/${bucket}/policy`, {
    method: "PUT",
    data: policy
  });
  expect(response.allowReplicaEdge).toBe(true);
}

async function expectPolicyPersisted(admin: { baseUrl: string; cookie: string }) {
  const policy = await fetchJson(admin, `/api/admin/buckets/${bucket}/policy`);
  expect(policy.allowReplicaEdge).toBe(true);
  expect(policy.sourceSelectionStrategy).toBe("ORIGIN_REPLICA_EDGE");
  expect(policy.fragmentPriorityStrategy).toBe("INITIAL_FIRST");
  expect(policy.fallbackMode).toBe("ORIGIN_RANGE");
}

async function createApplicationCredential(admin: { baseUrl: string; cookie: string }): Promise<{ token: string }> {
  const body = await fetchJson<ApplicationCredentialResponse>(admin, "/api/admin/application-credentials", {
    method: "POST",
    data: { name: "tcc-e2e-sdk" }
  });
  expect(body.token).toBeTruthy();
  return { token: body.token };
}

async function revokeAccessPackage(admin: { baseUrl: string; cookie: string }, packageId: string): Promise<void> {
  await fetchJson(admin, `/api/admin/access-packages/${packageId}/revoke`, {
    method: "POST"
  });
}

async function expectReplicaConnected(admin: { baseUrl: string; cookie: string }, replicaId: string) {
  await expect.poll(async () => {
    const replicas = await fetchJson<ReplicaSummary[]>(admin, "/api/admin/replicas");
    const replica = replicas.find((item) => item.id === replicaId);
    return replica?.healthStatus ?? null;
  }, { timeout: 90_000 }).toBe("OK");
  const replicas = await fetchJson<ReplicaSummary[]>(admin, "/api/admin/replicas");
  const replica = replicas.find((item) => item.id === replicaId);
  expect(replica, "replica summary after health").toBeTruthy();
  expect(required(replica).availableObjects).toBeGreaterThanOrEqual(1);
}

async function postSdkFragmentEvent(
  request: APIRequestContext,
  packageBody: AccessPackageResponse,
  manifest: ManifestResponse
) {
  const fragment = manifest.fragments[0];
  const response = await request.post(`${originUrl}/pontemesh/access-packages/${packageBody.id}/events/${bucket}/${objectKey}`, {
    headers: {
      Authorization: `Bearer ${packageBody.packageToken}`,
      "content-type": "application/json"
    },
    data: {
      sourceType: "REPLICA_EDGE",
      fragmentIndex: fragment.index,
      fragmentHash: fragment.sha256,
      eventType: "FRAGMENT_VALIDATED",
      bytesTransferred: fragment.sizeBytes,
      outcome: "SUCCESS",
      latencyMs: 5,
      detail: { sessionId: "origin-replica-e2e" }
    }
  });
  expect(response.status()).toBe(201);
}

async function expectAuditEvents(admin: { baseUrl: string; cookie: string }) {
  const events = await fetchJson<AuditEvent[]>(admin, "/api/admin/audit-events?limit=300");
  const names = new Set(events.map((event) => event.event));
  for (const expected of [
    "s3_bucket_created",
    "s3_object_put",
    "bucket_policy_updated",
    "replica_credential_created",
    "replica_sync_plan_issued",
    "replica_availability_announced",
    "access_package_created",
    "manifest_issued",
    "sources_issued",
    "availability_issued",
    "access_package_object_served",
    "sdk_fragment_event_recorded"
  ]) {
    expect(names.has(expected), `missing audit event ${expected}`).toBe(true);
  }
  expect(
    names.has("replica_object_synced") || names.has("replica_fragment_synced"),
    "missing replica object or fragment sync audit event"
  ).toBe(true);
}

async function expectReplicaServingAuditEvents(admin: { baseUrl: string; cookie: string }) {
  const events = await fetchJson<AuditEvent[]>(admin, "/api/admin/audit-events?limit=100");
  const names = new Set(events.map((event) => event.event));
  for (const expected of [
    "replica_object_served",
    "replica_range_served",
    "replica_access_denied"
  ]) {
    expect(names.has(expected), `missing replica serving audit event ${expected}`).toBe(true);
  }
}

async function fetchJson<T = Record<string, unknown>>(
  admin: { baseUrl: string; cookie: string },
  route: string,
  options: { method?: string; data?: unknown } = {}
): Promise<T> {
  const response = await fetch(`${admin.baseUrl}${route}`, {
    method: options.method ?? "GET",
    headers: {
      accept: "application/json",
      cookie: admin.cookie,
      ...(options.data ? { "content-type": "application/json" } : {})
    },
    body: options.data ? JSON.stringify(options.data) : undefined
  });
  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(`${route} failed with ${response.status}: ${body}`);
  }
  const text = await response.text();
  return (text ? JSON.parse(text) : {}) as T;
}

async function apiJson<T = Record<string, unknown>>(
  request: APIRequestContext,
  url: string,
  options: {
    method?: "POST";
    headers?: Record<string, string>;
    data?: unknown;
  } = {}
): Promise<T> {
  const response = options.method === "POST"
    ? await request.post(url, options)
    : await request.get(url, options);
  if (!response.ok()) {
    const body = await response.text().catch(() => "");
    throw new Error(`${url} failed with ${response.status()}: ${body}`);
  }
  return response.json() as Promise<T>;
}

async function apiText(
  request: APIRequestContext,
  url: string,
  options: {
    headers?: Record<string, string>;
    expectedStatus?: number;
  } = {}
) {
  const response = await request.get(url, options);
  expect(response.status()).toBe(options.expectedStatus ?? 200);
  return response.text();
}

async function endpointReachable(request: APIRequestContext, url: string) {
  const response = await request.get(url, { timeout: 10_000 });
  return response.status() < 500;
}

async function waitForSetupStatus(request: APIRequestContext, baseUrl: string, setupRequired: boolean) {
  await expect.poll(async () => {
    const response = await request.get(`${baseUrl}/api/setup/status`);
    if (!response.ok()) {
      return null;
    }
    const body = await response.json();
    return body.setupRequired;
  }, { timeout: 45_000 }).toBe(setupRequired);
}

async function restartService(service: string) {
  execFileSync("docker", ["compose", "-p", projectName, "-f", composeFile, "restart", service], {
    cwd: path.resolve(".."),
    stdio: "ignore"
  });
}

function readInitialAdminToken(service: string) {
  return execFileSync("docker", [
    "compose",
    "-p",
    projectName,
    "-f",
    composeFile,
    "exec",
    "-T",
    service,
    "cat",
    "/var/pontemesh_home/secrets/initialAdminToken"
  ], {
    cwd: path.resolve(".."),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"]
  }).trim();
}

async function signedS3Fetch(
  request: APIRequestContext,
  method: "PUT" | "GET",
  s3Path: string,
  body = "",
  extraHeaders: Record<string, string> = {}
) {
  const url = new URL(s3Path, originS3Url);
  const headers = signedS3Headers(method, url, body, extraHeaders);
  return method === "PUT"
    ? request.put(url.toString(), { headers, data: body })
    : request.get(url.toString(), { headers });
}

function signedS3Headers(method: string, url: URL, body: string, extraHeaders: Record<string, string>) {
  const now = new Date();
  const amzDate = now.toISOString().replace(/[:-]|\.\d{3}/g, "");
  const date = amzDate.slice(0, 8);
  const payloadHash = sha256Hex(body);
  const headers: Record<string, string> = {
    host: url.host,
    "x-amz-content-sha256": payloadHash,
    "x-amz-date": amzDate,
    ...extraHeaders
  };
  const signedHeaderNames = Object.keys(headers).map((name) => name.toLowerCase()).sort();
  const canonicalHeaders = signedHeaderNames.map((name) => `${name}:${headers[name].trim()}\n`).join("");
  const signedHeaders = signedHeaderNames.join(";");
  const canonicalRequest = [
    method,
    url.pathname,
    url.search.slice(1),
    canonicalHeaders,
    signedHeaders,
    payloadHash
  ].join("\n");
  const credentialScope = `${date}/${region}/s3/aws4_request`;
  const stringToSign = [
    "AWS4-HMAC-SHA256",
    amzDate,
    credentialScope,
    sha256Hex(canonicalRequest)
  ].join("\n");
  const signature = hmacHex(signingKey(s3SecretAccessKey, date), stringToSign);
  return {
    ...headers,
    Authorization: `AWS4-HMAC-SHA256 Credential=${s3AccessKeyId}/${credentialScope}, SignedHeaders=${signedHeaders}, Signature=${signature}`
  };
}

function signingKey(secret: string, date: string) {
  const kDate = hmacBuffer(`AWS4${secret}`, date);
  const kRegion = hmacBuffer(kDate, region);
  const kService = hmacBuffer(kRegion, "s3");
  return hmacBuffer(kService, "aws4_request");
}

function sha256Hex(value: string) {
  return crypto.createHash("sha256").update(value).digest("hex");
}

function hmacHex(key: string | Buffer, value: string) {
  return crypto.createHmac("sha256", key).update(value).digest("hex");
}

function hmacBuffer(key: string | Buffer, value: string) {
  return crypto.createHmac("sha256", key).update(value).digest();
}

function required<T>(value: T | null | undefined): T {
  if (value === null || value === undefined || value === "") {
    throw new Error("required value was not provided");
  }
  return value;
}
