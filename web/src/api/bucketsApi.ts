import { HttpError, ensureOk } from "./http";

export type BucketSummary = {
  name: string;
  ownerId: string;
  ownerUsername: string;
  objectCount: number;
  totalBytes: number;
  createdAt: string;
};

export type BucketPolicy = {
  bucketName: string;
  accessPackageTtlSeconds: number;
  fragmentSizeBytes: number;
  allowReplicaEdge: boolean;
  allowPeerSharing: boolean;
  sourceSelectionStrategy: "ORIGIN_REPLICA_EDGE" | "ORIGIN_ONLY" | "REPLICA_EDGE_FIRST" | "PEER_FIRST" | string;
  fragmentPriorityStrategy: "MANIFEST_ORDER" | "INITIAL_FIRST" | "RAREST_FIRST" | string;
  failureThreshold: number;
  fallbackMode: "ORIGIN_RANGE" | "ORIGIN_FULL_OBJECT" | "DISABLED" | string;
  s3ListDefaultMaxKeys: number;
  s3ListMaxKeysLimit: number;
  s3ListAllowDelimiter: boolean;
  s3VersioningEnabled: boolean;
  s3ObjectTaggingEnabled: boolean;
  s3ChecksumAlgorithm: "SHA256" | "ETAG_MD5_COMPATIBLE" | "NONE" | string;
  s3MultipartAbortDays: number;
  s3DefaultEncryptionAlgorithm: "NONE" | "AES256" | "aws:kms" | string;
  s3DefaultEncryptionKeyId: string | null;
  s3ObjectLockEnabled: boolean;
  s3ObjectLockDefaultMode: "GOVERNANCE" | "COMPLIANCE" | string | null;
  s3ObjectLockDefaultRetainDays: number | null;
  s3LifecycleRules: unknown;
  s3ResourcePolicy: unknown;
  s3EventNotifications: unknown;
  updatedAt: string;
};

export type UpdateBucketPolicyInput = {
  accessPackageTtlSeconds: number;
  fragmentSizeBytes: number;
  allowReplicaEdge: boolean;
  allowPeerSharing: boolean;
  sourceSelectionStrategy: string;
  fragmentPriorityStrategy: string;
  failureThreshold: number;
  fallbackMode: string;
  s3ListDefaultMaxKeys: number;
  s3ListMaxKeysLimit: number;
  s3ListAllowDelimiter: boolean;
  s3VersioningEnabled: boolean;
  s3ObjectTaggingEnabled: boolean;
  s3ChecksumAlgorithm: string;
  s3MultipartAbortDays: number;
  s3DefaultEncryptionAlgorithm: string;
  s3DefaultEncryptionKeyId: string | null;
  s3ObjectLockEnabled: boolean;
  s3ObjectLockDefaultMode: string | null;
  s3ObjectLockDefaultRetainDays: number | null;
  s3LifecycleRules: unknown;
  s3ResourcePolicy: unknown;
  s3EventNotifications: unknown;
};

export type BucketPolicyDefaultsInput = Pick<
  UpdateBucketPolicyInput,
  | "accessPackageTtlSeconds"
  | "fragmentSizeBytes"
  | "allowReplicaEdge"
  | "allowPeerSharing"
  | "sourceSelectionStrategy"
  | "fragmentPriorityStrategy"
  | "failureThreshold"
  | "fallbackMode"
>;

export type BucketPolicyDefaults = BucketPolicyDefaultsInput & {
  updatedAt: string;
};

export type BulkBucketPolicyResult = {
  updatedBuckets: string[];
  updatedCount: number;
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

export type ObjectsPage = PaginatedResponse<ObjectSummary> & {
  commonPrefixes: string[];
};

export type PageParams = {
  query?: string;
  page: number;
  pageSize: number;
};

export type ObjectPageParams = PageParams & {
  prefix?: string;
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

export async function getBucketPolicy(bucketName: string): Promise<BucketPolicy> {
  const response = await fetch(`/api/admin/buckets/${encodeURIComponent(bucketName)}/policy`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<BucketPolicy>;
}

export async function updateBucketPolicy(bucketName: string, policy: UpdateBucketPolicyInput): Promise<BucketPolicy> {
  const response = await fetch(`/api/admin/buckets/${encodeURIComponent(bucketName)}/policy`, {
    method: "PUT",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(policy)
  });
  await ensureOk(response);
  return response.json() as Promise<BucketPolicy>;
}

export async function getBucketPolicyDefaults(): Promise<BucketPolicyDefaults> {
  const response = await fetch("/api/admin/bucket-policy-defaults", {
    headers: { accept: "application/json" }
  });
  await ensureOk(response);
  return response.json() as Promise<BucketPolicyDefaults>;
}

export async function updateBucketPolicyDefaults(policy: BucketPolicyDefaultsInput): Promise<BucketPolicyDefaults> {
  const response = await fetch("/api/admin/bucket-policy-defaults", {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(policy)
  });
  await ensureOk(response);
  return response.json() as Promise<BucketPolicyDefaults>;
}

export async function bulkUpdateBucketPolicies(
  target: { allBuckets: boolean; bucketNames: string[] },
  policy: BucketPolicyDefaultsInput
): Promise<BulkBucketPolicyResult> {
  const response = await fetch("/api/admin/buckets/bulk-policy", {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ ...target, policy })
  });
  await ensureOk(response);
  return response.json() as Promise<BulkBucketPolicyResult>;
}

export async function listObjects(bucketName: string, params: ObjectPageParams): Promise<ObjectsPage> {
  const response = await fetch(`/api/admin/buckets/${encodeURIComponent(bucketName)}/objects${objectPaginationQuery(params)}`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ObjectsPage>;
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

function objectPaginationQuery({ query, page, pageSize, prefix }: ObjectPageParams): string {
  const params = new URLSearchParams();
  const trimmedQuery = query?.trim();
  if (trimmedQuery) {
    params.set("query", trimmedQuery);
  }
  if (prefix) {
    params.set("prefix", prefix);
  }
  params.set("page", String(page));
  params.set("pageSize", String(pageSize));
  return `?${params.toString()}`;
}
