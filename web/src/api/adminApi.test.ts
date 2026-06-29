import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createApplicationCredential,
  listApplicationCredentials,
  revokeApplicationCredential
} from "./applicationCredentialsApi";
import { listAuditEvents } from "./auditApi";
import { listBuckets } from "./bucketsApi";
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
      .mockResolvedValueOnce(jsonResponse([]));

    await getStorageStatus();
    await listBuckets();

    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/storage/status", {
      headers: {
        accept: "application/json"
      }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/buckets", {
      headers: {
        accept: "application/json"
      }
    });
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
      .mockResolvedValueOnce(jsonResponse([{ bucket: "media", originBytesServed: 1, originRequests: 1, replicaBytesSynced: 2, fragmentEvents: 1 }]))
      .mockResolvedValueOnce(jsonResponse([{ bucket: "media", key: "a.txt", originBytesServed: 1, originRequests: 1, replicaBytesSynced: 2, fragmentEvents: 1 }]))
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
    }
  };
}
