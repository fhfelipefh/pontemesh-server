import { afterEach, describe, expect, it, vi } from "vitest";
import { listAuditEvents } from "./auditApi";
import { HttpError } from "./http";

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" }
  });
}

const sampleEvent = {
  id: "evt-1",
  event: "object_uploaded",
  principal: "admin",
  outcome: "success",
  detail: "bucket=media key=logo.png",
  createdAt: "2026-08-10T12:00:00Z"
};

describe("auditApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches all audit events without filters", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([sampleEvent])
    );

    const events = await listAuditEvents();

    expect(events).toHaveLength(1);
    expect(events[0].event).toBe("object_uploaded");
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/audit-events", {
      headers: { accept: "application/json" }
    });
  });

  it("passes event filter as a query parameter", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([sampleEvent])
    );

    await listAuditEvents({ event: "object_uploaded" });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/admin/audit-events?event=object_uploaded",
      { headers: { accept: "application/json" } }
    );
  });

  it("passes principal and outcome filters together", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([sampleEvent])
    );

    await listAuditEvents({ principal: "admin", outcome: "success" });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/admin/audit-events?principal=admin&outcome=success",
      { headers: { accept: "application/json" } }
    );
  });

  it("passes time range filters since and until", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([])
    );

    await listAuditEvents({ since: "2026-08-01T00:00:00Z", until: "2026-08-10T23:59:59Z" });

    const calledUrl = fetchMock.mock.calls[0][0] as string;
    expect(calledUrl).toContain("since=2026-08-01T00%3A00%3A00Z");
    expect(calledUrl).toContain("until=2026-08-10T23%3A59%3A59Z");
  });

  it("passes limit as a numeric query parameter", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([])
    );

    await listAuditEvents({ limit: 50 });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/admin/audit-events?limit=50",
      { headers: { accept: "application/json" } }
    );
  });

  it("omits filters with undefined or empty string values", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([])
    );

    await listAuditEvents({ event: "", principal: undefined, outcome: "success" });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/admin/audit-events?outcome=success",
      { headers: { accept: "application/json" } }
    );
  });

  it("combines all filters in a single request", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse([])
    );

    await listAuditEvents({
      event: "bucket_deleted",
      principal: "admin",
      outcome: "success",
      limit: 10
    });

    const calledUrl = fetchMock.mock.calls[0][0] as string;
    expect(calledUrl).toContain("event=bucket_deleted");
    expect(calledUrl).toContain("principal=admin");
    expect(calledUrl).toContain("outcome=success");
    expect(calledUrl).toContain("limit=10");
  });

  it("surfaces 401 as HttpError", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({ error: "authentication required" }, 401)
    );

    await expect(listAuditEvents()).rejects.toMatchObject({
      name: "HttpError",
      status: 401,
      message: "authentication required"
    } satisfies Partial<HttpError>);
  });
});
