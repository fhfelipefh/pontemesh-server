import { ensureOk } from "./http";

export type BucketSummary = {
  name: string;
  objectCount: number;
  totalBytes: number;
  createdAt: string;
};

export type ObjectSummary = {
  key: string;
  sizeBytes: number;
  contentType: string;
  sha256: string;
  createdAt: string;
  state: "AVAILABLE" | "DELETED" | string;
};

export async function listBuckets(): Promise<BucketSummary[]> {
  const response = await fetch("/api/admin/buckets", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<BucketSummary[]>;
}

export async function createBucket(name: string): Promise<BucketSummary> {
  const response = await fetch("/api/admin/buckets", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify({ name })
  });
  await ensureOk(response);
  return response.json() as Promise<BucketSummary>;
}

export async function deleteBucket(bucketName: string): Promise<void> {
  const response = await fetch(`/api/admin/buckets/${encodeURIComponent(bucketName)}`, {
    method: "DELETE"
  });
  await ensureOk(response);
}

export async function listObjects(bucketName: string): Promise<ObjectSummary[]> {
  const response = await fetch(`/api/admin/buckets/${encodeURIComponent(bucketName)}/objects`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ObjectSummary[]>;
}

export async function uploadObject(bucketName: string, file: File, key?: string): Promise<ObjectSummary> {
  const body = new FormData();
  if (key?.trim()) {
    body.append("key", key.trim());
  }
  body.append("file", file);

  const response = await fetch(`/api/admin/buckets/${encodeURIComponent(bucketName)}/objects`, {
    method: "POST",
    body
  });
  await ensureOk(response);
  return response.json() as Promise<ObjectSummary>;
}

export async function deleteObject(bucketName: string, objectKey: string): Promise<void> {
  const response = await fetch(
    `/api/admin/buckets/${encodeURIComponent(bucketName)}/objects/${encodePathKey(objectKey)}`,
    {
      method: "DELETE"
    }
  );
  await ensureOk(response);
}

function encodePathKey(key: string): string {
  return key.split("/").map(encodeURIComponent).join("/");
}
