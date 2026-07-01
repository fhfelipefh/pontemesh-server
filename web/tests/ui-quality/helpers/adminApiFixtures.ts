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

type AdminFixtureOptions = {
  buckets?: typeof buckets;
  objectsByBucket?: Record<string, typeof objects>;
};

export async function installAdminApiFixtures(page: Page, options: AdminFixtureOptions = {}) {
  const fixtureBuckets = options.buckets ?? buckets;
  const fixtureObjectsByBucket = options.objectsByBucket ?? {
    assets: objects,
    documents: objects.slice(1)
  };

  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;

    if (!path.startsWith("/api/")) {
      return route.continue();
    }

    if (path === "/api/setup/status") {
      return json(route, { setupRequired: false });
    }

    if (path === "/api/auth/me") {
      return json(route, { authenticated: true, username: "admin" });
    }

    if (path === "/api/auth/logout") {
      return json(route, {});
    }

    if (path === "/api/admin/instance") {
      return json(route, dashboardSummary.instance);
    }

    if (path === "/api/admin/dashboard/summary") {
      return json(route, dashboardSummary);
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

    const objectListMatch = path.match(/^\/api\/admin\/buckets\/([^/]+)\/objects$/);
    if (objectListMatch) {
      const bucketName = decodeURIComponent(objectListMatch[1]);
      if (request.method() === "POST") {
        return json(route, objects[0]);
      }
      const query = url.searchParams.get("query")?.toLowerCase() ?? "";
      const bucketObjects = fixtureObjectsByBucket[bucketName] ?? [];
      const filteredObjects = query
        ? bucketObjects.filter((object) => object.key.toLowerCase().includes(query))
        : bucketObjects;
      return json(route, pageResponse(filteredObjects));
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

async function json(route: Route, body: unknown, status = 200) {
  await route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(body)
  });
}
