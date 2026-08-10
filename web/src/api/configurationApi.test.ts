import { afterEach, describe, expect, it, vi } from "vitest";
import { exportConfiguration, importConfiguration } from "./configurationApi";
import { HttpError } from "./http";

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" }
  });
}

describe("configurationApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("exportConfiguration", () => {
    it("sends GET to /api/admin/configuration and returns a blob", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        new Response(JSON.stringify({ mcpSettings: {}, bucketPolicies: [] }), {
          status: 200,
          headers: { "content-type": "application/json" }
        })
      );

      const blob = await exportConfiguration();

      expect(blob).toBeInstanceOf(Blob);
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/configuration", {
        headers: { accept: "application/json" }
      });
    });

    it("surfaces 401 as HttpError", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ error: "authentication required" }, 401)
      );

      await expect(exportConfiguration()).rejects.toMatchObject({
        name: "HttpError",
        status: 401
      } satisfies Partial<HttpError>);
    });
  });

  describe("importConfiguration", () => {
    it("parses file content and posts it as JSON to /api/admin/configuration", async () => {
      const configPayload = { mcpSettings: { enabled: true }, bucketPolicies: [] };
      const file = new File([JSON.stringify(configPayload)], "config.json", { type: "application/json" });

      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ appliedMcpSettings: true, appliedBucketPolicies: 0, skippedBucketPolicies: [] })
      );

      const result = await importConfiguration(file);

      expect(result.appliedMcpSettings).toBe(true);
      expect(result.appliedBucketPolicies).toBe(0);
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/configuration", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(configPayload)
      });
    });

    it("reports how many bucket policies were applied and skipped", async () => {
      const configPayload = { mcpSettings: {}, bucketPolicies: [{ bucket: "media" }, { bucket: "ghost" }] };
      const file = new File([JSON.stringify(configPayload)], "config.json", { type: "application/json" });

      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ appliedMcpSettings: false, appliedBucketPolicies: 1, skippedBucketPolicies: ["ghost"] })
      );

      const result = await importConfiguration(file);

      expect(result.appliedBucketPolicies).toBe(1);
      expect(result.skippedBucketPolicies).toEqual(["ghost"]);
    });

    it("surfaces server-side validation errors as HttpError", async () => {
      const file = new File([JSON.stringify({ invalid: true })], "bad.json", { type: "application/json" });

      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ error: "invalid configuration format" }, 400)
      );

      await expect(importConfiguration(file)).rejects.toMatchObject({
        name: "HttpError",
        status: 400,
        message: "invalid configuration format"
      } satisfies Partial<HttpError>);
    });
  });
});
