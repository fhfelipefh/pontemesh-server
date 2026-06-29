import { afterEach, describe, expect, it, vi } from "vitest";
import { listBuckets } from "./bucketsApi";
import { getDashboardSummary } from "./dashboardApi";
import { HttpError } from "./http";
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
