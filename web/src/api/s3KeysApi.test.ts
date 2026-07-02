import { afterEach, describe, expect, it, vi } from "vitest";
import { HttpError } from "./http";
import { createS3AccessKey, listS3AccessKeys, revokeS3AccessKey } from "./s3KeysApi";

describe("s3KeysApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("requests S3 keys with backend pagination parameters", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({
        items: [
          {
            id: "key-1",
            name: "prod",
            accessKeyId: "PMK123",
            userId: "admin",
            isActive: true,
            createdAt: "2026-06-29T12:00:00Z",
            revokedAt: null,
            lastUsedAt: null
          }
        ],
        page: 3,
        pageSize: 10,
        total: 21,
        totalPages: 3
      })
    );

    const page = await listS3AccessKeys(3, 10);

    expect(page.total).toBe(21);
    expect(page.totalPages).toBe(3);
    expect(page.items).toHaveLength(1);
    expect(page.items[0]).not.toHaveProperty("secretAccessKey");
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/s3/access-keys?page=3&pageSize=10", {
      headers: {
        accept: "application/json"
      }
    });
  });

  it("creates S3 keys without exposing secrets in query strings", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({
        id: "key-1",
        name: "deploy",
        accessKeyId: "PMK456",
        secretAccessKey: "secret-material",
        createdAt: "2026-06-29T12:00:00Z"
      }, 201)
    );

    const created = await createS3AccessKey("deploy");

    expect(created.secretAccessKey).toBe("secret-material");
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/s3/access-keys", {
      method: "POST",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify({ name: "deploy" })
    });
  });

  it("revokes S3 keys by id through DELETE", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(null, { status: 204 }));

    await revokeS3AccessKey("key/with spaces");

    expect(fetchMock).toHaveBeenCalledWith("/api/admin/s3/access-keys/key%2Fwith%20spaces", {
      method: "DELETE"
    });
  });

  it("surfaces paginated list failures as HttpError", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse({ error: "session expired" }, 401));

    await expect(listS3AccessKeys(1, 10)).rejects.toMatchObject({
      name: "HttpError",
      status: 401,
      message: "session expired"
    } satisfies Partial<HttpError>);
  });
});

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "content-type": "application/json"
    }
  });
}
