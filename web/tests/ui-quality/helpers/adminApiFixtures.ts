import { Page, Route } from "@playwright/test";

const now = "2026-06-30T12:00:00.000Z";

const dashboardSummary = {
  instance: {
    name: "Ponte Mesh QA",
    role: "origin",
    environment: "container",
    version: "0.1.0",
    uptimeSeconds: 7200
  },
  storage: {
    path: "/var/lib/pontemesh/storage",
    exists: true,
    totalBytes: 10_737_418_240,
    availableBytes: 8_589_934_592,
    usedBytes: 2_147_483_648,
    usedPercent: 20,
    writable: true,
    warnings: []
  },
  objects: {
    totalBuckets: 2,
    totalObjects: 3,
    totalObjectBytes: 4096
  },
  resources: {
    cpuUsagePercent: 12,
    memoryUsedBytes: 268_435_456,
    memoryTotalBytes: 1_073_741_824,
    memoryUsagePercent: 25,
    processMemoryBytes: 134_217_728,
    source: "sysinfo",
    warnings: []
  },
  health: {
    databaseConnected: true,
    storageWritable: true,
    setupCompleted: true,
    authenticated: true,
    lastCheckedAt: now
  },
  mcp: {
    enabled: false,
    endpoint: "/mcp",
    authRequired: true,
    readToolsEnabled: true,
    writeToolsEnabled: false,
    adminToolsEnabled: false,
    resourcesEnabled: true,
    promptsEnabled: true,
    lastActivityAt: null,
    activeSessionsCount: 0,
    recentCallsCount: 0
  }
};

const buckets = [
  {
    name: "assets",
    objectCount: 2,
    totalBytes: 3072,
    createdAt: now
  },
  {
    name: "documents",
    objectCount: 1,
    totalBytes: 1024,
    createdAt: now
  }
];

const objects = [
  {
    key: "images/logo.png",
    sizeBytes: 2048,
    contentType: "image/png",
    sha256: "8f14e45fceea167a5a36dedd4bea2543",
    createdAt: now,
    updatedAt: now,
    state: "AVAILABLE"
  },
  {
    key: "docs/readme.txt",
    sizeBytes: 1024,
    contentType: "text/plain",
    sha256: "45c48cce2e2d7fbdea1afc51c7c6ad26",
    createdAt: now,
    updatedAt: now,
    state: "AVAILABLE"
  }
];

const bucketPolicy = {
  accessPackageTtlSeconds: 900,
  fragmentSizeBytes: 4_194_304,
  sourceSelectionStrategy: "ORIGIN_REPLICA_EDGE",
  fragmentPriorityStrategy: "INITIAL_FIRST",
  failureThreshold: 3,
  fallbackMode: "ORIGIN_RANGE",
  allowReplicaEdge: false,
  allowPeerSharing: false,
  s3ListDefaultMaxKeys: 1000,
  s3ListMaxKeysLimit: 10000,
  s3ListAllowDelimiter: true,
  s3VersioningEnabled: false,
  s3ObjectTaggingEnabled: true,
  s3ChecksumAlgorithm: "SHA256",
  s3MultipartAbortDays: 7,
  s3DefaultEncryptionAlgorithm: "NONE",
  s3DefaultEncryptionKeyId: null,
  s3ObjectLockEnabled: false,
  s3ObjectLockDefaultMode: null,
  s3ObjectLockDefaultRetainDays: null,
  s3LifecycleRules: [],
  s3ResourcePolicy: { Version: "2012-10-17", Statement: [] },
  s3EventNotifications: { EventBridgeEnabled: false, Rules: [] }
};

type AdminFixtureOptions = {
  buckets?: typeof buckets;
  objectsByBucket?: Record<string, typeof objects>;
};

export async function installAdminApiFixtures(page: Page, options: AdminFixtureOptions = {}) {
  const fixtureBuckets = options.buckets ?? buckets;
  let fixtureInstanceName = dashboardSummary.instance.name;
  const fixtureObjectsByBucket = options.objectsByBucket ?? {
    assets: objects,
    documents: objects.slice(1)
  };
  let diskGuardSettings = {
    enabled: true,
    level: "OK",
    usedPercent: 20,
    availableBytes: 8_589_934_592,
    totalBytes: 10_737_418_240,
    warningPercent: 80,
    degradedPercent: 90,
    blockPercent: 95
  };
  let operationalWebhook = {
    enabled: false,
    url: "",
    cron: "*/15 * * * *",
    payloadPreview: {
      schemaVersion: 1,
      event: "pontemesh.operational_status",
      generatedAt: now,
      instance: {
        name: dashboardSummary.instance.name,
        role: dashboardSummary.instance.role,
        version: "0.3.6"
      },
      storage: {
        ...dashboardSummary.storage,
        guard: diskGuardSettings
      }
    }
  };
  let adminUsers: Array<{ id: string; username: string; createdAt: string; lastLoginAt: string | null }> = [
    {
      id: "admin-qa",
      username: "admin",
      createdAt: now,
      lastLoginAt: now
    }
  ];

  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;

    if (!path.startsWith("/api/")) {
      return route.continue();
    }

    if (path === "/api/setup/status") {
      return json(route, {
        setupRequired: false,
        serverVersion: "0.2.2",
        internalWebPort: 8080,
        internalS3Port: 9000,
        publicWebUrl: null,
        publicS3Url: null
      });
    }

    if (path === "/api/auth/me") {
      return json(route, { authenticated: true, username: "admin" });
    }

    if (path === "/api/auth/logout") {
      return json(route, {});
    }

    if (path === "/api/admin/instance") {
      if (request.method() === "PUT") {
        const body = request.postDataJSON() as { name: string };
        fixtureInstanceName = body.name.trim();
      }
      return json(route, { ...dashboardSummary.instance, name: fixtureInstanceName });
    }

    if (path === "/api/admin/system/update") {
      if (request.method() === "POST") {
        return json(route, { restartRequired: true }, 202);
      }
      return json(route, {
        currentVersion: "0.3.3",
        latestVersion: "0.3.4",
        releaseUrl: "https://github.com/fhfelipefh/pontemesh-server/releases/tag/v0.3.4",
        updateAvailable: true,
        automaticUpdateEnabled: false
      });
    }

    if (path === "/api/admin/dashboard/summary") {
      return json(route, dashboardSummary);
    }

    if (path === "/api/admin/storage/disk-guard") {
      if (request.method() === "PUT") {
        diskGuardSettings = { ...diskGuardSettings, ...request.postDataJSON() };
      }
      return json(route, diskGuardSettings);
    }

    if (path === "/api/admin/operational-webhook") {
      if (request.method() === "PUT") {
        operationalWebhook = {
          ...operationalWebhook,
          ...(request.postDataJSON() as Pick<typeof operationalWebhook, "enabled" | "url" | "cron">)
        };
      }
      return json(route, operationalWebhook);
    }

    if (path === "/api/admin/users") {
      if (request.method() === "POST") {
        const body = request.postDataJSON() as { username: string };
        adminUsers = [
          ...adminUsers,
          {
            id: `admin-${adminUsers.length + 1}`,
            username: body.username,
            createdAt: now,
            lastLoginAt: null
          }
        ];
        return json(route, adminUsers.at(-1), 201);
      }
      return json(route, adminUsers);
    }

    if (path === "/api/admin/logs/application") {
      return json(route, []);
    }

    if (path === "/api/admin/buckets") {
      if (request.method() === "POST") {
        return json(route, fixtureBuckets[0]);
      }
      return json(route, pageResponse(fixtureBuckets));
    }

    if (path === "/api/admin/bucket-policy-defaults") {
      return json(route, {
        accessPackageTtlSeconds: bucketPolicy.accessPackageTtlSeconds,
        fragmentSizeBytes: bucketPolicy.fragmentSizeBytes,
        allowReplicaEdge: bucketPolicy.allowReplicaEdge,
        allowPeerSharing: bucketPolicy.allowPeerSharing,
        sourceSelectionStrategy: bucketPolicy.sourceSelectionStrategy,
        fragmentPriorityStrategy: bucketPolicy.fragmentPriorityStrategy,
        failureThreshold: bucketPolicy.failureThreshold,
        fallbackMode: bucketPolicy.fallbackMode,
        updatedAt: now
      });
    }

    if (path === "/api/admin/buckets/bulk-policy") {
      const body = request.postDataJSON() as { allBuckets: boolean; bucketNames: string[] };
      const updatedBuckets = body.allBuckets ? fixtureBuckets.map((bucket) => bucket.name) : body.bucketNames;
      return json(route, { updatedBuckets, updatedCount: updatedBuckets.length });
    }

    const objectListMatch = path.match(/^\/api\/admin\/buckets\/([^/]+)\/objects$/);
    if (objectListMatch) {
      const bucketName = decodeURIComponent(objectListMatch[1]);
      if (request.method() === "POST") {
        return json(route, objects[0]);
      }
      const query = url.searchParams.get("query")?.toLowerCase() ?? "";
      const prefix = url.searchParams.get("prefix") ?? "";
      const bucketObjects = fixtureObjectsByBucket[bucketName] ?? [];
      const filteredObjects = query
        ? bucketObjects.filter((object) => object.key.toLowerCase().includes(query))
        : bucketObjects;
      return json(route, objectsPageResponse(filteredObjects, prefix));
    }

    if (path.match(/^\/api\/admin\/buckets\/[^/]+\/policy$/)) {
      return json(route, bucketPolicy);
    }

    if (path.match(/^\/api\/admin\/buckets\/[^/]+(\/objects\/.+)?$/)) {
      return json(route, {});
    }

    if (path === "/api/admin/metrics/origin-traffic") {
      return json(route, {
        totalRequests: 24,
        fullObjectRequests: 18,
        rangeRequests: 6,
        totalBytesServed: 4096
      });
    }

    if (path === "/api/admin/metrics/replica-traffic") {
      return json(route, {
        totalReplicas: 1,
        activeReplicas: 1,
        totalBytesSynced: 2048,
        totalBytesServed: 1024,
        totalFragmentsSynced: 8,
        totalFragmentsServed: 4,
        syncFailures: 0,
        authFailures: 0
      });
    }

    if (path === "/api/admin/replicas") {
      return json(route, [
        {
          id: "replica-qa",
          name: "Replica QA",
          allowedBuckets: ["assets"],
          createdAt: now,
          revoked: false,
          availableObjects: 2,
          lastSeenAt: now,
          healthStatus: "healthy",
          healthReportedAt: now
        }
      ]);
    }

    if (path === "/api/admin/s3/access-keys") {
      return json(route, {
        items: [
          {
            id: "key-qa",
            name: "default-admin-key",
            accessKeyId: "PMKQAACCESSKEY",
            userId: null,
            isActive: true,
            createdAt: now,
            revokedAt: null,
            lastUsedAt: null
          }
        ],
        page: 1,
        pageSize: 10,
        total: 1,
        totalPages: 1
      });
    }

    if (path === "/api/admin/application-credentials") {
      return json(route, [
        {
          id: "app-qa",
          name: "default-sdk",
          scopes: ["objects:read"],
          createdAt: now,
          revoked: false
        }
      ]);
    }

    if (path === "/api/admin/mcp/settings") {
      return json(route, {
        enabled: true,
        endpointPath: "/mcp",
        bindHost: null,
        requireAuth: true,
        readToolsEnabled: true,
        writeToolsEnabled: false,
    adminToolsEnabled: false,
        exposeResources: true,
        exposePrompts: true,
        allowLocalhostOnly: true,
        createdAt: now,
        updatedAt: now
      });
    }

    if (path === "/api/admin/mcp/status") {
      return json(route, dashboardSummary.mcp);
    }

    if (path === "/api/admin/mcp/tokens") {
      return json(route, [
        {
          id: "mcp-token-qa",
          name: "default-mcp-client",
          tokenPrefix: "pmcp_qa12345",
          active: true,
          createdAt: now,
          revokedAt: null,
          lastUsedAt: null
        }
      ]);
    }

    if (path === "/api/admin/mcp/activity") {
      return json(route, []);
    }

    return json(route, { error: `Unhandled UI quality fixture route: ${path}` }, 404);
  });
}

function pageResponse<T>(items: T[]) {
  return {
    items,
    page: 1,
    pageSize: 20,
    totalItems: items.length,
    totalPages: 1
  };
}

function objectsPageResponse(items: typeof objects, prefix: string) {
  const normalizedPrefix = prefix && !prefix.endsWith("/") ? `${prefix}/` : prefix;
  const commonPrefixes = new Set<string>();
  const files = items.filter((object) => {
    if (!object.key.startsWith(normalizedPrefix)) {
      return false;
    }
    const remainder = object.key.slice(normalizedPrefix.length);
    const separatorIndex = remainder.indexOf("/");
    if (separatorIndex >= 0) {
      commonPrefixes.add(`${normalizedPrefix}${remainder.slice(0, separatorIndex + 1)}`);
      return false;
    }
    return true;
  });
  return {
    ...pageResponse(files),
    commonPrefixes: [...commonPrefixes]
  };
}

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(body)
  });
}
