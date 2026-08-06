import { afterEach, describe, expect, it, vi } from "vitest";
import { getCurrentUser, login, logout } from "./authApi";
import { HttpError } from "./http";

describe("authApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("posts login credentials to the authenticated backend endpoint", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({ authenticated: true, username: "admin" })
    );

    const user = await login({ username: "admin", password: "correct-password" });

    expect(user).toEqual({ authenticated: true, username: "admin" });
    expect(fetchMock).toHaveBeenCalledWith("/api/auth/login", {
      method: "POST",
      headers: {
        "content-type": "application/json"
      },
      body: JSON.stringify({ username: "admin", password: "correct-password" })
    });
  });

  it("surfaces 401 login failures as HttpError", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({ error: "invalid username or password" }, 401)
    );

    await expect(login({ username: "admin", password: "wrong-password" })).rejects.toMatchObject({
      name: "HttpError",
      status: 401,
      message: "invalid username or password"
    } satisfies Partial<HttpError>);
  });

  it("checks the current session through /api/auth/me", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      jsonResponse({ authenticated: false, username: null }, 401)
    );

    await expect(getCurrentUser()).rejects.toMatchObject({ status: 401 });
    expect(fetchMock).toHaveBeenCalledWith("/api/auth/me", {
      headers: {
        accept: "application/json"
      }
    });
  });

  it("logs out through a POST without sending credentials in the URL", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(null, { status: 204 }));

    await logout();

    expect(fetchMock).toHaveBeenCalledWith("/api/auth/logout", {
      method: "POST"
    });
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
