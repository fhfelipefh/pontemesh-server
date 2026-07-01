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
  mcp: {
    enabled: boolean;
    endpoint: string;
    authRequired: boolean;
    readToolsEnabled: boolean;
    writeToolsEnabled: boolean;
    resourcesEnabled: boolean;
    promptsEnabled: boolean;
    lastActivityAt: string | null;
    activeSessionsCount: number;
    recentCallsCount: number;
  };
};

export type InstanceSummary = {
  name: string;
  role: "origin" | "replica-edge";
  environment: "native" | "container" | "unknown";
  version: string;
  uptimeSeconds: number;
};

export type OriginTrafficMetrics = {
  totalRequests: number;
  fullObjectRequests: number;
  rangeRequests: number;
  totalBytesServed: number;
};

export type ReplicaTrafficMetrics = {
  totalReplicas: number;
  activeReplicas: number;
  totalBytesSynced: number;
  totalBytesServed: number;
  totalFragmentsSynced: number;
  totalFragmentsServed: number;
  syncFailures: number;
  authFailures: number;
};

export type BucketTrafficMetric = {
  bucket: string;
  originBytesServed: number;
  originRequests: number;
  replicaBytesSynced: number;
  peerBytesServed: number;
  fragmentEvents: number;
  fallbackEvents: number;
  integrityFailures: number;
  originOffloadBytes: number;
};

export type ObjectTrafficMetric = {
  bucket: string;
  key: string;
  originBytesServed: number;
  originRequests: number;
  replicaBytesSynced: number;
  peerBytesServed: number;
  fragmentEvents: number;
  fallbackEvents: number;
  integrityFailures: number;
  originOffloadBytes: number;
};

export type ReplicaDetailMetric = {
  replicaId: string;
  replicaName: string;
  bytesSynced: number;
  bytesServed: number;
  fragmentsSynced: number;
  fragmentsServed: number;
  syncFailures: number;
  authFailures: number;
  fragmentEvents: number;
};

export type ApplicationLogEntry = {
  timestamp: string;
  level: "info" | "warn" | "error" | string;
  target: string;
  message: string;
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

export async function getApplicationLogs(limit = 80): Promise<ApplicationLogEntry[]> {
  const response = await fetch(`/api/admin/logs/application?limit=${encodeURIComponent(String(limit))}`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ApplicationLogEntry[]>;
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

export async function getOriginTrafficMetrics(): Promise<OriginTrafficMetrics> {
  const response = await fetch("/api/admin/metrics/origin-traffic", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<OriginTrafficMetrics>;
}

export async function getReplicaTrafficMetrics(): Promise<ReplicaTrafficMetrics> {
  const response = await fetch("/api/admin/metrics/replica-traffic", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ReplicaTrafficMetrics>;
}

export async function getBucketTrafficMetrics(): Promise<BucketTrafficMetric[]> {
  const response = await fetch("/api/admin/metrics/buckets", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<BucketTrafficMetric[]>;
}

export async function getObjectTrafficMetrics(): Promise<ObjectTrafficMetric[]> {
  const response = await fetch("/api/admin/metrics/objects", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ObjectTrafficMetric[]>;
}

export async function getReplicaDetailMetrics(replicaId: string): Promise<ReplicaDetailMetric> {
  const response = await fetch(`/api/admin/metrics/replicas/${encodeURIComponent(replicaId)}`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ReplicaDetailMetric>;
}
