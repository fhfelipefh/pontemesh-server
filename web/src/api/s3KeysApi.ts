import { ensureOk } from "./http";

export type S3AccessKeySummary = {
  id: string;
  accessKeyId: string;
  userId: string | null;
  isActive: boolean;
  createdAt: string;
  revokedAt: string | null;
  lastUsedAt: string | null;
};

export type CreatedS3AccessKey = {
  key: S3AccessKeySummary;
  secretAccessKey: string;
};

export async function listS3AccessKeys(): Promise<S3AccessKeySummary[]> {
  const response = await fetch("/api/admin/s3-access-keys", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<S3AccessKeySummary[]>;
}

export async function createS3AccessKey(): Promise<CreatedS3AccessKey> {
  const response = await fetch("/api/admin/s3-access-keys", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: "{}"
  });
  await ensureOk(response);
  return response.json() as Promise<CreatedS3AccessKey>;
}

export async function revokeS3AccessKey(accessKeyId: string): Promise<void> {
  const response = await fetch(`/api/admin/s3-access-keys/${encodeURIComponent(accessKeyId)}/revoke`, {
    method: "POST"
  });
  await ensureOk(response);
}
