import { afterEach, describe, expect, it, vi } from "vitest";
import {
  bulkUpdateBucketPolicies,
  createBucket,
  deleteBucket,
  getBucketPolicyDefaults,
  getObjectDownloadUrl,
  listObjects,
  updateBucketPolicyDefaults
} from "./bucketsApi";
import { HttpError } from "./http";

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" }
  });
}

const baseDefaults = {
  accessPackageTtlSeconds: 300,
  fragmentSizeBytes: 1048576,
  allowReplicaEdge: true,
  allowPeerSharing: false,
  sourceSelectionStrategy: "ORIGIN_REPLICA_EDGE",
  fragmentPriorityStrategy: "MANIFEST_ORDER",
  failureThreshold: 3,
  fallbackMode: "ORIGIN_RANGE"
};

describe("bucketsApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("createBucket", () => {
    it("posts to /api/admin/buckets with the bucket name", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ name: "media", objectCount: 0, totalBytes: 0, createdAt: "2026-01-01T00:00:00Z" }, 201)
      );

      const bucket = await createBucket("media");

      expect(bucket.name).toBe("media");
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/buckets", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name: "media" })
      });
    });

    it("surfaces error responses as HttpError", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ error: "bucket already exists" }, 400)
      );

      await expect(createBucket("media")).rejects.toMatchObject({
        name: "HttpError",
        status: 400,
        message: "bucket already exists"
      } satisfies Partial<HttpError>);
    });
  });

  describe("deleteBucket", () => {
    it("sends DELETE to the correct bucket URL with encoded name", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(null, { status: 204 })
      );

      await deleteBucket("my/bucket");

      expect(fetchMock).toHaveBeenCalledWith("/api/admin/buckets/my%2Fbucket", {
        method: "DELETE"
      });
    });

    it("surfaces 409 when bucket is not empty", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ error: "bucket must be empty before it can be deleted" }, 409)
      );

      await expect(deleteBucket("media")).rejects.toMatchObject({
        status: 409
      } satisfies Partial<HttpError>);
    });
  });

  describe("getBucketPolicyDefaults", () => {
    it("fetches from /api/admin/bucket-policy-defaults", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ ...baseDefaults, updatedAt: "2026-01-01T00:00:00Z" })
      );

      const defaults = await getBucketPolicyDefaults();

      expect(defaults.fallbackMode).toBe("ORIGIN_RANGE");
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/bucket-policy-defaults", {
        headers: { accept: "application/json" }
      });
    });
  });

  describe("updateBucketPolicyDefaults", () => {
    it("sends PUT with the full defaults payload", async () => {
      const updated = { ...baseDefaults, updatedAt: "2026-07-01T00:00:00Z" };
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse(updated)
      );

      const result = await updateBucketPolicyDefaults({ ...baseDefaults, allowPeerSharing: true });

      expect(result.allowPeerSharing).toBe(false);
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/bucket-policy-defaults", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ ...baseDefaults, allowPeerSharing: true })
      });
    });
  });

  describe("bulkUpdateBucketPolicies", () => {
    it("sends PUT to /api/admin/buckets/bulk-policy with target and policy", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ updatedBuckets: ["media", "docs"], updatedCount: 2 })
      );

      const result = await bulkUpdateBucketPolicies(
        { allBuckets: false, bucketNames: ["media", "docs"] },
        baseDefaults
      );

      expect(result.updatedCount).toBe(2);
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/buckets/bulk-policy", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          allBuckets: false,
          bucketNames: ["media", "docs"],
          policy: baseDefaults
        })
      });
    });

    it("supports allBuckets flag to update every bucket at once", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ updatedBuckets: ["a", "b", "c"], updatedCount: 3 })
      );

      await bulkUpdateBucketPolicies({ allBuckets: true, bucketNames: [] }, baseDefaults);

      expect(fetchMock).toHaveBeenCalledWith("/api/admin/buckets/bulk-policy", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ allBuckets: true, bucketNames: [], policy: baseDefaults })
      });
    });
  });

  describe("listObjects with prefix", () => {
    it("includes prefix in the query string when provided", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ items: [], commonPrefixes: [], page: 1, pageSize: 20, totalItems: 0, totalPages: 1 })
      );

      await listObjects("assets", { page: 1, pageSize: 20, prefix: "images/" });

      expect(fetchMock).toHaveBeenCalledWith(
        "/api/admin/buckets/assets/objects?prefix=images%2F&page=1&pageSize=20",
        { headers: { accept: "application/json" } }
      );
    });

    it("omits prefix from query string when not provided", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ items: [], commonPrefixes: [], page: 1, pageSize: 20, totalItems: 0, totalPages: 1 })
      );

      await listObjects("assets", { page: 1, pageSize: 20 });

      const calledUrl = (fetchMock.mock.calls[0][0] as string);
      expect(calledUrl).not.toContain("prefix");
    });

    it("combines prefix and query in the query string", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ items: [], commonPrefixes: [], page: 1, pageSize: 10, totalItems: 0, totalPages: 1 })
      );

      await listObjects("assets", { page: 1, pageSize: 10, prefix: "docs/", query: "readme" });

      expect(fetchMock).toHaveBeenCalledWith(
        "/api/admin/buckets/assets/objects?query=readme&prefix=docs%2F&page=1&pageSize=10",
        { headers: { accept: "application/json" } }
      );
    });

    it("returns commonPrefixes from the response", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({
          items: [{ key: "images/logo.png", sizeBytes: 512, contentType: "image/png", sha256: "x", createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z", state: "AVAILABLE" }],
          commonPrefixes: ["images/icons/", "images/banners/"],
          page: 1,
          pageSize: 20,
          totalItems: 1,
          totalPages: 1
        })
      );

      const page = await listObjects("assets", { page: 1, pageSize: 20, prefix: "images/" });

      expect(page.commonPrefixes).toEqual(["images/icons/", "images/banners/"]);
      expect(page.items).toHaveLength(1);
    });
  });

  describe("getObjectDownloadUrl", () => {
    it("encodes each path segment independently preserving slashes", () => {
      expect(getObjectDownloadUrl("my bucket", "path/to/my file.txt")).toBe(
        "/api/admin/buckets/my%20bucket/objects/path/to/my%20file.txt"
      );
    });

    it("encodes bucket name with slashes", () => {
      expect(getObjectDownloadUrl("org/repo", "data.bin")).toBe(
        "/api/admin/buckets/org%2Frepo/objects/data.bin"
      );
    });

    it("handles keys with special characters in each segment", () => {
      expect(getObjectDownloadUrl("bucket", "folder/file (1).txt")).toBe(
        "/api/admin/buckets/bucket/objects/folder/file%20(1).txt"
      );
    });
  });
});
