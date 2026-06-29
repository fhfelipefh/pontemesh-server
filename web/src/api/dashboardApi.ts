import { ensureOk } from "./http";

export type DashboardSummary = {
  instance: InstanceSummary;
  storage: {
    path: string;
    exists: boolean;
    totalBytes: number | null;
    availableBytes: number | null;
    usedBytes: number | null;
    usedPercent: number | null;
    writable: boolean;
    warnings: string[];
  };
  objects: {
    totalBuckets: number;
    totalObjects: number;
    totalObjectBytes: number;
  };
  resources: {
    cpuUsagePercent: number | null;
    memoryUsedBytes: number | null;
    memoryTotalBytes: number | null;
    memoryUsagePercent: number | null;
    processMemoryBytes: number | null;
    source: "sysinfo" | "cgroup" | "unavailable" | string;
    warnings: string[];
  };
  health: {
    databaseConnected: boolean;
    storageWritable: boolean;
    setupCompleted: boolean;
    authenticated: boolean;
    lastCheckedAt: string;
  };
};

export type InstanceSummary = {
  name: string;
  role: "origin" | "replica-edge";
  environment: "native" | "container" | "unknown";
  version: string;
  uptimeSeconds: number;
};

export async function getDashboardSummary(): Promise<DashboardSummary> {
  const response = await fetch("/api/admin/dashboard/summary", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<DashboardSummary>;
}

export async function getInstanceSummary(): Promise<InstanceSummary> {
  const response = await fetch("/api/admin/instance", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<InstanceSummary>;
}
