import { ensureOk } from "./http";

export type StorageStatus = {
  path: string;
  exists: boolean;
  writable: boolean;
  totalBytes: number | null;
  availableBytes: number | null;
  usedBytes: number | null;
  usedPercent: number | null;
  warnings: string[];
};

export type DiskGuardSettings = {
  enabled: boolean;
  level: "OK" | "WARNING" | "DEGRADED" | "BLOCKED" | string;
  usedPercent: number | null;
  availableBytes: number | null;
  totalBytes: number | null;
  warningPercent: number;
  degradedPercent: number;
  blockPercent: number;
};

export type UpdateDiskGuardSettings = Pick<
  DiskGuardSettings,
  "enabled" | "warningPercent" | "degradedPercent" | "blockPercent"
>;

export async function getStorageStatus(): Promise<StorageStatus> {
  const response = await fetch("/api/admin/storage/status", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<StorageStatus>;
}

export async function getDiskGuardSettings(): Promise<DiskGuardSettings> {
  const response = await fetch("/api/admin/storage/disk-guard", {
    headers: { accept: "application/json" }
  });
  await ensureOk(response);
  return response.json() as Promise<DiskGuardSettings>;
}

export async function updateDiskGuardSettings(settings: UpdateDiskGuardSettings): Promise<DiskGuardSettings> {
  const response = await fetch("/api/admin/storage/disk-guard", {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(settings)
  });
  await ensureOk(response);
  return response.json() as Promise<DiskGuardSettings>;
}

export type StorageDriveInfo = {
  path: string;
  isPrimary: boolean;
  exists: boolean;
  writable: boolean;
  totalBytes: number | null;
  availableBytes: number | null;
  usedBytes: number | null;
  usedPercent: number | null;
  level: "OK" | "WARNING" | "DEGRADED" | "BLOCKED" | string;
  warnings: string[];
};

export type StoragePoolStatus = {
  allocationStrategy: "MOST_AVAILABLE_FREE_SPACE" | "ROUND_ROBIN" | string;
  drives: StorageDriveInfo[];
  totalBytes: number | null;
  availableBytes: number | null;
  usedBytes: number | null;
  usedPercent: number | null;
};

export type StorageDrainResult = {
  sourcePath: string;
  targetPath: string;
  objectsMigrated: number;
  bytesMigrated: number;
};

export async function getStoragePoolStatus(): Promise<StoragePoolStatus> {
  const response = await fetch("/api/admin/storage/drives", {
    headers: { accept: "application/json" }
  });
  await ensureOk(response);
  return response.json() as Promise<StoragePoolStatus>;
}

export async function addStorageDrive(path: string): Promise<StoragePoolStatus> {
  const response = await fetch("/api/admin/storage/drives", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ path })
  });
  await ensureOk(response);
  return response.json() as Promise<StoragePoolStatus>;
}

export async function drainStorageDrive(
  path: string,
  targetPath?: string,
  removeFromConfig: boolean = true
): Promise<StorageDrainResult> {
  const response = await fetch("/api/admin/storage/drives/drain", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      path,
      targetPath: targetPath || undefined,
      removeFromConfig
    })
  });
  await ensureOk(response);
  return response.json() as Promise<StorageDrainResult>;
}

export async function updateStorageAllocationStrategy(strategy: string): Promise<StoragePoolStatus> {
  const response = await fetch("/api/admin/storage/drives/allocation", {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ strategy })
  });
  await ensureOk(response);
  return response.json() as Promise<StoragePoolStatus>;
}
