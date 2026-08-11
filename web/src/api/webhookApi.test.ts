import { afterEach, describe, expect, it, vi } from "vitest";
import { getOperationalWebhook, updateOperationalWebhook } from "./webhookApi";

const settings = {
  enabled: true,
  url: "http://localhost:5678/webhook/storage",
  cron: "*/15 * * * *",
  payloadPreview: { schemaVersion: 1, event: "pontemesh.operational_status" }
};

describe("webhookApi", () => {
  afterEach(() => vi.restoreAllMocks());

  it("loads the operational webhook configuration", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse(settings));

    await expect(getOperationalWebhook()).resolves.toEqual(settings);
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/operational-webhook", {
      headers: { accept: "application/json" }
    });
  });

  it("saves URL, enabled state, and cron", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse(settings));
    const update = { enabled: true, url: settings.url, cron: settings.cron };

    await expect(updateOperationalWebhook(update)).resolves.toEqual(settings);
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/operational-webhook", {
      method: "PUT",
      headers: { accept: "application/json", "content-type": "application/json" },
      body: JSON.stringify(update)
    });
  });
});

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" }
  });
}
