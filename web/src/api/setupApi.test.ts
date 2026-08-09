import { afterEach, describe, expect, it, vi } from "vitest";
import { getSetupStatus } from "./setupApi";

describe("setupApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns the setup state and compiled server version", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({
        setupRequired: true,
        serverVersion: "0.2.1",
        internalWebPort: 8080,
        internalS3Port: 9000
      }), {
        status: 200,
        headers: { "content-type": "application/json" }
      })
    );

    await expect(getSetupStatus()).resolves.toEqual({
      setupRequired: true,
      serverVersion: "0.2.1",
      internalWebPort: 8080,
      internalS3Port: 9000
    });
    expect(fetchMock).toHaveBeenCalledWith("/api/setup/status", {
      headers: { accept: "application/json" }
    });
  });
});
