import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createApplicationCredential,
  listApplicationCredentials,
  revokeApplicationCredential
} from "./applicationCredentialsApi";
import { listAuditEvents } from "./auditApi";
import {
  deleteObject,
  getBucketPolicy,
  getObjectDownloadUrl,
  listBuckets,
  listObjects,
  updateBucketPolicy,
  uploadObject
} from "./bucketsApi";
import {
  getBucketTrafficMetrics,
  getDashboardSummary,
  getObjectTrafficMetrics,
  getOriginTrafficMetrics,
  getReplicaDetailMetrics,
  getReplicaTrafficMetrics
} from "./dashboardApi";
import { HttpError } from "./http";
import { createReplicaCredential, listReplicas, revokeReplica } from "./replicasApi";
import { getStorageStatus } from "./storageApi";

describe("admin API clients", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches the dashboard summary from the protected admin endpoint", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse(dashboardSummary()));

    const summary = await getDashboardSummary();

    expect(summary.health.authenticated).toBe(true);
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/dashboard/summary", {
      headers: {
        accept: "application/json"
      }
    });
  });

  it("surfaces dashboard 401 responses so callers can redirect to login", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({ error: "authentication required" }, 401)
    );

    await expect(getDashboardSummary()).rejects.toMatchObject({
      name: "HttpError",
      status: 401,
      message: "authentication required"
    } satisfies Partial<HttpError>);
  });

  it("uses protected admin routes for storage and buckets", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse({
        path: "/tmp/storage",
        exists: true,
        writable: true,
        totalBytes: null,
        availableBytes: null,
        usedBytes: null,
        usedPercent: null,
        warnings: []
      }))
      .mockResolvedValueOnce(jsonResponse({
        items: [],
        page: 1,
        pageSize: 20,
        totalItems: 0,
        totalPages: 1
      }));

    await getStorageStatus();
    await listBuckets({ page: 1, pageSize: 20 });

    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/storage/status", {
      headers: {
        accept: "application/json"
      }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/buckets?page=1&pageSize=20", {
      headers: {
        accept: "application/json"
      }
    });
  });

  it("manages bucket objects through protected admin routes", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse({
        items: [{
          key: "folder/hello world.txt",
          sizeBytes: 5,
          contentType: "text/plain",
          sha256: "abc",
          createdAt: "2026-06-30T12:00:00Z",
          updatedAt: "2026-06-30T12:00:00Z",
          state: "AVAILABLE"
        }],
        page: 1,
        pageSize: 20,
        totalItems: 1,
        totalPages: 1
      }))
      .mockResolvedValueOnce(jsonResponse({
        key: "folder/hello world.txt",
        sizeBytes: 5,
        contentType: "text/plain",
        sha256: "abc",
        createdAt: "2026-06-30T12:00:00Z",
        updatedAt: "2026-06-30T12:00:00Z",
        state: "AVAILABLE"
      }, 201))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));

    await listObjects("bucket/name", { query: "hello", page: 1, pageSize: 20 });
    await uploadObject("bucket/name", new File(["hello"], "hello.txt", { type: "text/plain" }), "folder/hello world.txt");
    await deleteObject("bucket/name", "folder/hello world.txt");

    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/buckets/bucket%2Fname/objects?query=hello&page=1&pageSize=20", {
      headers: {
        accept: "application/json"
      }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/buckets/bucket%2Fname/objects", {
      method: "POST",
      body: expect.any(FormData)
    });
    expect(fetchMock).toHaveBeenNthCalledWith(3, "/api/admin/buckets/bucket%2Fname/objects/folder/hello%20world.txt", {
      method: "DELETE"
    });
    expect(getObjectDownloadUrl("bucket/name", "folder/hello world.txt")).toBe(
      "/api/admin/buckets/bucket%2Fname/objects/folder/hello%20world.txt"
    );
  });

  it("manages hybrid bucket policy through protected admin routes", async () => {
    const policy = {
      bucketName: "media",
      accessPackageTtlSeconds: 300,
      fragmentSizeBytes: 1048576,
      allowReplicaEdge: true,
      allowPeerSharing: false,
      sourceSelectionStrategy: "ORIGIN_REPLICA_EDGE",
      fragmentPriorityStrategy: "MANIFEST_ORDER",
      failureThreshold: 3,
      fallbackMode: "ORIGIN_RANGE",
      updatedAt: "2026-07-01T12:00:00Z"
    };
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse(policy))
      .mockResolvedValueOnce(jsonResponse({ ...policy, allowPeerSharing: true, sourceSelectionStrategy: "PEER_FIRST" }));

    await getBucketPolicy("media");
    await updateBucketPolicy("media", {
      accessPackageTtlSeconds: 300,
      fragmentSizeBytes: 1048576,
      allowReplicaEdge: true,
      allowPeerSharing: true,
      sourceSelectionStrategy: "PEER_FIRST",
      fragmentPriorityStrategy: "MANIFEST_ORDER",
      failureThreshold: 3,
      fallbackMode: "ORIGIN_RANGE"
    });

    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/buckets/media/policy", {
      headers: { accept: "application/json" }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/buckets/media/policy", {
      method: "PUT",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify({
        accessPackageTtlSeconds: 300,
        fragmentSizeBytes: 1048576,
        allowReplicaEdge: true,
        allowPeerSharing: true,
        sourceSelectionStrategy: "PEER_FIRST",
        fragmentPriorityStrategy: "MANIFEST_ORDER",
        failureThreshold: 3,
        fallbackMode: "ORIGIN_RANGE"
      })
    });
  });

  it("surfaces bucket object 401 responses", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({ error: "authentication required" }, 401)
    );

    await expect(listObjects("assets", { page: 1, pageSize: 20 })).rejects.toMatchObject({
      name: "HttpError",
      status: 401,
      message: "authentication required"
    } satisfies Partial<HttpError>);
  });

  it("fetches Origin traffic metrics from the protected admin endpoint", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({
        totalRequests: 2,
        fullObjectRequests: 1,
        rangeRequests: 1,
        totalBytesServed: 42
      })
    );

    const metrics = await getOriginTrafficMetrics();

    expect(metrics.totalBytesServed).toBe(42);
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/metrics/origin-traffic", {
      headers: {
        accept: "application/json"
      }
    });
  });

  it("fetches replica traffic metrics from the protected admin endpoint", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({
        totalReplicas: 1,
        activeReplicas: 1,
        totalBytesSynced: 5,
        totalBytesServed: 7,
        totalFragmentsSynced: 1,
        totalFragmentsServed: 2,
        syncFailures: 0,
        authFailures: 0
      })
    );

    const metrics = await getReplicaTrafficMetrics();

    expect(metrics.totalBytesSynced).toBe(5);
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/metrics/replica-traffic", {
      headers: {
        accept: "application/json"
      }
    });
  });

  it("fetches detailed traffic metrics from protected admin endpoints", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse([{ bucket: "media", originBytesServed: 1, originRequests: 1, replicaBytesSynced: 2, peerBytesServed: 3, fragmentEvents: 1, fallbackEvents: 1, integrityFailures: 0, originOffloadBytes: 5 }]))
      .mockResolvedValueOnce(jsonResponse([{ bucket: "media", key: "a.txt", originBytesServed: 1, originRequests: 1, replicaBytesSynced: 2, peerBytesServed: 3, fragmentEvents: 1, fallbackEvents: 1, integrityFailures: 0, originOffloadBytes: 5 }]))
      .mockResolvedValueOnce(jsonResponse({
        replicaId: "replica-1",
        replicaName: "edge-1",
        bytesSynced: 2,
        bytesServed: 0,
        fragmentsSynced: 1,
        fragmentsServed: 0,
        syncFailures: 0,
        authFailures: 0,
        fragmentEvents: 1
      }));

    await getBucketTrafficMetrics();
    await getObjectTrafficMetrics();
    await getReplicaDetailMetrics("replica/1");

    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/metrics/buckets", {
      headers: { accept: "application/json" }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/metrics/objects", {
      headers: { accept: "application/json" }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(3, "/api/admin/metrics/replicas/replica%2F1", {
      headers: { accept: "application/json" }
    });
  });

  it("fetches filtered audit events from the protected admin endpoint", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([{ id: "audit-1", event: "access_package_revoked", principal: "admin", outcome: "success", detail: "package_id=1", createdAt: "2026-06-29T12:00:00Z" }])
    );

    const events = await listAuditEvents({ event: "access_package_revoked", outcome: "success", limit: 25 });

    expect(events[0].event).toBe("access_package_revoked");
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/audit-events?event=access_package_revoked&outcome=success&limit=25", {
      headers: {
        accept: "application/json"
      }
    });
  });

  it("manages Replica credentials through protected admin routes", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse([]))
      .mockResolvedValueOnce(jsonResponse({
        replica: {
          id: "replica-1",
          name: "edge-1",
          allowedBuckets: ["media"],
          createdAt: "2026-06-29T12:00:00Z",
          revoked: false,
          availableObjects: 0,
          lastSeenAt: null,
          healthStatus: null,
          healthReportedAt: null
        },
        token: "pm_rep_secret"
      }, 201))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));

    await listReplicas();
    await createReplicaCredential("edge-1", ["media"]);
    await revokeReplica("replica/1");

    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/replicas", {
      headers: {
        accept: "application/json"
      }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/replicas", {
      method: "POST",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify({ name: "edge-1", allowedBuckets: ["media"] })
    });
    expect(fetchMock).toHaveBeenNthCalledWith(3, "/api/admin/replicas/replica%2F1/revoke", {
      method: "POST"
    });
  });

  it("manages application credentials through protected admin routes", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse([]))
      .mockResolvedValueOnce(jsonResponse({
        credential: {
          id: "app-1",
          name: "default-sdk",
          scopes: ["pontemesh:manifest:read"],
          createdAt: "2026-06-29T12:00:00Z",
          revoked: false
        },
        token: "pm_app_secret"
      }, 201))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));

    await listApplicationCredentials();
    await createApplicationCredential("default-sdk", ["pontemesh:manifest:read"]);
    await revokeApplicationCredential("app/1");

    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/application-credentials", {
      headers: {
        accept: "application/json"
      }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/application-credentials", {
      method: "POST",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify({ name: "default-sdk", scopes: ["pontemesh:manifest:read"] })
    });
    expect(fetchMock).toHaveBeenNthCalledWith(3, "/api/admin/application-credentials/app%2F1/revoke", {
      method: "POST"
    });
  });
});

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json"
    }
  });
}

function dashboardSummary() {
  return {
    instance: {
      name: "Test Origin",
      role: "origin",
      environment: "native",
      version: "0.1.0",
      uptimeSeconds: 12
    },
    storage: {
      path: "/tmp/storage",
      exists: true,
      totalBytes: null,
      availableBytes: null,
      usedBytes: null,
      usedPercent: null,
      writable: true,
      warnings: []
    },
    objects: {
      totalBuckets: 0,
      totalObjects: 0,
      totalObjectBytes: 0
    },
    resources: {
      cpuUsagePercent: null,
      memoryUsedBytes: null,
      memoryTotalBytes: null,
      memoryUsagePercent: null,
      processMemoryBytes: null,
      source: "unavailable",
      warnings: []
    },
    health: {
      databaseConnected: true,
      storageWritable: true,
      setupCompleted: true,
      authenticated: true,
      lastCheckedAt: "2026-06-29T00:00:00Z"
    },
    mcp: {
      enabled: false,
      endpoint: "/mcp",
      authRequired: true,
      readToolsEnabled: true,
      writeToolsEnabled: false,
      resourcesEnabled: true,
      promptsEnabled: true,
      lastActivityAt: null,
      activeSessionsCount: 0,
      recentCallsCount: 0
    }
  };
}
