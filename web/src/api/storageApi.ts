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
