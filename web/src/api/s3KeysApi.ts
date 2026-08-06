import { ensureOk } from "./http";

export type S3AccessKeySummary = {
  id: string;
  name: string | null;
  accessKeyId: string;
  userId: string | null;
  isActive: boolean;
  createdAt: string;
  revokedAt: string | null;
  lastUsedAt: string | null;
};

export type CreatedS3AccessKey = {
  id: string;
  name: string | null;
  accessKeyId: string;
  secretAccessKey: string;
  createdAt: string;
};

export type S3AccessKeysPage = {
  items: S3AccessKeySummary[];
  page: number;
  pageSize: number;
  total: number;
  totalPages: number;
};

export async function listS3AccessKeys(page = 1, pageSize = 10): Promise<S3AccessKeysPage> {
  const params = new URLSearchParams({
    page: String(page),
    pageSize: String(pageSize)
  });
  const response = await fetch(`/api/admin/s3/access-keys?${params.toString()}`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<S3AccessKeysPage>;
}

export async function createS3AccessKey(name?: string): Promise<CreatedS3AccessKey> {
  const response = await fetch("/api/admin/s3/access-keys", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify({ name: name?.trim() || null })
  });
  await ensureOk(response);
  return response.json() as Promise<CreatedS3AccessKey>;
}

export async function revokeS3AccessKey(id: string): Promise<void> {
  const response = await fetch(`/api/admin/s3/access-keys/${encodeURIComponent(id)}`, {
    method: "DELETE"
  });
  await ensureOk(response);
}
