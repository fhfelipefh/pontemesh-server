import { describe, expect, it } from "vitest";
import { HttpError, ensureOk } from "./http";

describe("ensureOk", () => {
  it("does not throw for successful responses", async () => {
    await expect(ensureOk(new Response(null, { status: 204 }))).resolves.toBeUndefined();
  });

  it("throws HttpError with status and backend message", async () => {
    const response = new Response(JSON.stringify({ error: "authentication required" }), {
      status: 401,
      headers: {
        "content-type": "application/json"
      }
    });

    await expect(ensureOk(response)).rejects.toMatchObject({
      name: "HttpError",
      status: 401,
      message: "authentication required"
    } satisfies Partial<HttpError>);
  });

  it("uses a generic message when the response body is not JSON", async () => {
    const response = new Response("not json", { status: 500 });

    await expect(ensureOk(response)).rejects.toMatchObject({
      status: 500,
      message: "Request failed"
    });
  });
});
