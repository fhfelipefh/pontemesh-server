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

export async function getStorageStatus(): Promise<StorageStatus> {
  const response = await fetch("/api/admin/storage/status", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<StorageStatus>;
}
