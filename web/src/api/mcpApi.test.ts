import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createMcpToken,
  getMcpSettings,
  getMcpStatus,
  listMcpActivity,
  listMcpTokens,
  revokeMcpToken,
  updateMcpSettings
} from "./mcpApi";

describe("mcpApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("loads MCP settings and status from protected admin routes", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse({
        enabled: false,
        endpointPath: "/mcp",
        bindHost: null,
        requireAuth: true,
        readToolsEnabled: true,
        writeToolsEnabled: false,
        exposeResources: true,
        exposePrompts: true,
        allowLocalhostOnly: true,
        createdAt: "2026-07-01T12:00:00Z",
        updatedAt: "2026-07-01T12:00:00Z"
      }))
      .mockResolvedValueOnce(jsonResponse({
        enabled: false,
        endpoint: "/mcp",
        authRequired: true,
        readToolsEnabled: true,
        writeToolsEnabled: false,
        resourcesEnabled: true,
        promptsEnabled: true,
        lastActivityAt: null,
        activeSessionsCount: 0,
        recentCallsCount: 0
      }));

    await expect(getMcpSettings()).resolves.toMatchObject({ endpointPath: "/mcp" });
    await expect(getMcpStatus()).resolves.toMatchObject({ authRequired: true });
    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/mcp/settings", {
      headers: { accept: "application/json" }
    });
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/admin/mcp/status", {
      headers: { accept: "application/json" }
    });
  });

  it("updates MCP settings without disabling authentication", async () => {
    const settings = {
      enabled: true,
      endpointPath: "/mcp",
      bindHost: null,
      requireAuth: true,
      readToolsEnabled: true,
      writeToolsEnabled: false,
      exposeResources: true,
      exposePrompts: true,
      allowLocalhostOnly: true
    };
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse({
      ...settings,
      createdAt: "2026-07-01T12:00:00Z",
      updatedAt: "2026-07-01T12:01:00Z"
    }));

    const saved = await updateMcpSettings(settings);

    expect(saved.enabled).toBe(true);
    expect(fetchMock).toHaveBeenCalledWith("/api/admin/mcp/settings", {
      method: "PUT",
      headers: {
        accept: "application/json",
        "content-type": "application/json"
      },
      body: JSON.stringify(settings)
    });
  });

  it("creates, lists, revokes tokens, and lists activity through admin routes", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(jsonResponse({
        token: {
          id: "token-1",
          name: "desktop",
          tokenPrefix: "pmcp_abcd1234",
          active: true,
          createdAt: "2026-07-01T12:00:00Z",
          revokedAt: null,
          lastUsedAt: null
        },
        secret: "pmcp_abcd1234secret"
      }, 201))
      .mockResolvedValueOnce(jsonResponse([
        {
          id: "token-1",
          name: "desktop",
          tokenPrefix: "pmcp_abcd1234",
          active: true,
          createdAt: "2026-07-01T12:00:00Z",
          revokedAt: null,
          lastUsedAt: null
        }
      ]))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(jsonResponse([
        {
          id: "activity-1",
          tokenId: "token-1",
          method: "tools/list",
          target: null,
          outcome: "success",
          createdAt: "2026-07-01T12:02:00Z"
        }
      ]));

    const created = await createMcpToken(" desktop ");
    const tokens = await listMcpTokens();
    await revokeMcpToken("token/1");
    const activity = await listMcpActivity();

    expect(created.secret).toBe("pmcp_abcd1234secret");
    expect(tokens[0].tokenPrefix).toBe("pmcp_abcd1234");
    expect(activity[0].method).toBe("tools/list");
    expect(fetchMock).toHaveBeenNthCalledWith(1, "/api/admin/mcp/tokens", {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json"
      },
      body: JSON.stringify({ name: "desktop" })
    });
    expect(fetchMock).toHaveBeenNthCalledWith(3, "/api/admin/mcp/tokens/token%2F1", {
      method: "DELETE"
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
