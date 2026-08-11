import { afterEach, describe, expect, it, vi } from "vitest";
import { getServerUpdateStatus, requestServerUpdate } from "./serverUpdateApi";

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
}

describe("serverUpdateApi", () => {
  afterEach(() => vi.restoreAllMocks());

  it("loads the available server update", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse({
      currentVersion: "0.3.3", latestVersion: "0.3.4", releaseUrl: "https://example.test/release", updateAvailable: true
    }));

    await expect(getServerUpdateStatus()).resolves.toMatchObject({ latestVersion: "0.3.4", updateAvailable: true });
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/system/update", { headers: { accept: "application/json" } });
  });

  it("requests the configured updater", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse({ restartRequired: true }, 202));

    await expect(requestServerUpdate()).resolves.toBeUndefined();
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/system/update", { method: "POST", headers: { accept: "application/json" } });
  });
});
