import { afterEach, describe, expect, it, vi } from "vitest";
import { getApplicationLogs, getInstanceSummary, updateInstanceName } from "./dashboardApi";
import { HttpError } from "./http";

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" }
  });
}

describe("dashboardApi — instance and logs endpoints", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("getInstanceSummary", () => {
    it("fetches from /api/admin/instance", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({
          name: "My Origin",
          role: "origin",
          environment: "native",
          version: "0.3.3",
          uptimeSeconds: 900
        })
      );

      const instance = await getInstanceSummary();

      expect(instance.role).toBe("origin");
      expect(instance.uptimeSeconds).toBe(900);
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/instance", {
        headers: { accept: "application/json" }
      });
    });

    it("surfaces 401 as HttpError", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ error: "authentication required" }, 401)
      );

      await expect(getInstanceSummary()).rejects.toMatchObject({
        name: "HttpError",
        status: 401
      } satisfies Partial<HttpError>);
    });
  });

  describe("updateInstanceName", () => {
    it("sends PUT with name to /api/admin/instance", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({
          name: "Renamed Origin",
          role: "origin",
          environment: "container",
          version: "0.3.3",
          uptimeSeconds: 100
        })
      );

      const result = await updateInstanceName("Renamed Origin");

      expect(result.name).toBe("Renamed Origin");
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/instance", {
        method: "PUT",
        headers: {
          accept: "application/json",
          "content-type": "application/json"
        },
        body: JSON.stringify({ name: "Renamed Origin" })
      });
    });

    it("surfaces 400 when name is invalid", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ error: "name is too long" }, 400)
      );

      await expect(updateInstanceName("x".repeat(300))).rejects.toMatchObject({
        status: 400,
        message: "name is too long"
      } satisfies Partial<HttpError>);
    });
  });

  describe("getApplicationLogs", () => {
    it("fetches logs with default limit of 80", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse([
          { timestamp: "2026-08-10T12:00:00Z", level: "info", target: "pontemesh_server", message: "server started" }
        ])
      );

      const logs = await getApplicationLogs();

      expect(logs).toHaveLength(1);
      expect(logs[0].level).toBe("info");
      expect(fetchMock).toHaveBeenCalledWith("/api/admin/logs/application?limit=80", {
        headers: { accept: "application/json" }
      });
    });

    it("accepts a custom limit parameter", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse([])
      );

      await getApplicationLogs(10);

      expect(fetchMock).toHaveBeenCalledWith("/api/admin/logs/application?limit=10", {
        headers: { accept: "application/json" }
      });
    });

    it("encodes the limit value in the URL", async () => {
      const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse([])
      );

      await getApplicationLogs(200);

      const calledUrl = fetchMock.mock.calls[0][0] as string;
      expect(calledUrl).toContain("limit=200");
    });

    it("surfaces 401 as HttpError", async () => {
      vi.spyOn(globalThis, "fetch").mockResolvedValue(
        jsonResponse({ error: "authentication required" }, 401)
      );

      await expect(getApplicationLogs()).rejects.toMatchObject({
        name: "HttpError",
        status: 401
      } satisfies Partial<HttpError>);
    });
  });
});
