import { HttpError, ensureOk } from "./http";

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
  updatedAt: string;
  state: "AVAILABLE" | "DELETED" | string;
};

export type PaginatedResponse<T> = {
  items: T[];
  page: number;
  pageSize: number;
  totalItems: number;
  totalPages: number;
};

export type PageParams = {
  query?: string;
  page: number;
  pageSize: number;
};

export async function listBuckets(params: PageParams): Promise<PaginatedResponse<BucketSummary>> {
  const response = await fetch(`/api/admin/buckets${paginationQuery(params)}`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<PaginatedResponse<BucketSummary>>;
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

export async function listObjects(bucketName: string, params: PageParams): Promise<PaginatedResponse<ObjectSummary>> {
  const response = await fetch(`/api/admin/buckets/${encodeURIComponent(bucketName)}/objects${paginationQuery(params)}`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<PaginatedResponse<ObjectSummary>>;
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

export function uploadObjectWithProgress(
  bucketName: string,
  file: File,
  key: string | undefined,
  onProgress: (progress: { loadedBytes: number; totalBytes: number | null; percent: number | null }) => void
): Promise<ObjectSummary> {
  const body = new FormData();
  if (key?.trim()) {
    body.append("key", key.trim());
  }
  body.append("file", file);

  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open("POST", `/api/admin/buckets/${encodeURIComponent(bucketName)}/objects`);
    request.responseType = "json";

    request.upload.onprogress = (event) => {
      const totalBytes = event.lengthComputable ? event.total : null;
      onProgress({
        loadedBytes: event.loaded,
        totalBytes,
        percent: totalBytes ? Math.min(100, Math.round((event.loaded / totalBytes) * 100)) : null
      });
    };

    request.onload = () => {
      const body = request.response ?? parseJson(request.responseText);
      if (request.status >= 200 && request.status < 300) {
        resolve(body as ObjectSummary);
        return;
      }
      const errorBody = body as { error?: string } | null;
      reject(new HttpError(request.status, errorBody?.error ?? "Request failed"));
    };

    request.onerror = () => reject(new HttpError(0, "Request failed"));
    request.send(body);
  });
}

export async function deleteObject(bucketName: string, objectKey: string): Promise<void> {
  const response = await fetch(getObjectDownloadUrl(bucketName, objectKey), {
    method: "DELETE"
  });
  await ensureOk(response);
}

export function getObjectDownloadUrl(bucketName: string, objectKey: string): string {
  return `/api/admin/buckets/${encodeURIComponent(bucketName)}/objects/${encodePathKey(objectKey)}`;
}

function encodePathKey(key: string): string {
  return key.split("/").map(encodeURIComponent).join("/");
}

function parseJson(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return null;
  }
}

function paginationQuery({ query, page, pageSize }: PageParams): string {
  const params = new URLSearchParams();
  const trimmedQuery = query?.trim();
  if (trimmedQuery) {
    params.set("query", trimmedQuery);
  }
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return `?${params.toString()}`;
}
