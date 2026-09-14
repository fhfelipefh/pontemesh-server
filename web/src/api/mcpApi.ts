import { ensureOk } from "./http";

export type McpAuthMode = "hybrid" | "token" | "oauth2";

export type McpSettings = {
  enabled: boolean;
  endpointPath: string;
  bindHost: string | null;
  requireAuth: boolean;
  authMode: McpAuthMode;
  readToolsEnabled: boolean;
  writeToolsEnabled: boolean;
  adminToolsEnabled: boolean;
  exposeResources: boolean;
  exposePrompts: boolean;
  allowLocalhostOnly: boolean;
  createdAt: string;
  updatedAt: string;
};

export type McpSettingsUpdate = Omit<McpSettings, "createdAt" | "updatedAt" | "authMode"> & {
  authMode?: McpAuthMode;
};

export type McpStatus = {
  enabled: boolean;
  endpoint: string;
  authRequired: boolean;
  readToolsEnabled: boolean;
  writeToolsEnabled: boolean;
  adminToolsEnabled: boolean;
  resourcesEnabled: boolean;
  promptsEnabled: boolean;
  lastActivityAt: string | null;
  activeSessionsCount: number;
  recentCallsCount: number;
};

export type McpAccessTokenSummary = {
  id: string;
  name: string;
  tokenPrefix: string;
  active: boolean;
  createdAt: string;
  revokedAt: string | null;
  lastUsedAt: string | null;
  scopes: string[];
};

export type CreatedMcpAccessToken = {
  token: McpAccessTokenSummary;
  secret: string;
};

export type McpActivityRecord = {
  id: string;
  tokenId: string | null;
  method: string;
  target: string | null;
  outcome: string;
  createdAt: string;
};

export async function getMcpSettings(): Promise<McpSettings> {
  const response = await fetch("/api/admin/mcp/settings", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<McpSettings>;
}

export async function updateMcpSettings(settings: McpSettingsUpdate): Promise<McpSettings> {
  const response = await fetch("/api/admin/mcp/settings", {
    method: "PUT",
    headers: {
      accept: "application/json",
      "content-type": "application/json"
    },
    body: JSON.stringify(settings)
  });
  await ensureOk(response);
  return response.json() as Promise<McpSettings>;
}

export async function getMcpStatus(): Promise<McpStatus> {
  const response = await fetch("/api/admin/mcp/status", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<McpStatus>;
}

export async function listMcpTokens(): Promise<McpAccessTokenSummary[]> {
  const response = await fetch("/api/admin/mcp/tokens", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<McpAccessTokenSummary[]>;
}

export async function createMcpToken(name: string, scopes: string[] = ["read"]): Promise<CreatedMcpAccessToken> {
  const response = await fetch("/api/admin/mcp/tokens", {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json"
    },
    body: JSON.stringify({ name: name.trim(), scopes })
  });
  await ensureOk(response);
  return response.json() as Promise<CreatedMcpAccessToken>;
}

export async function revokeMcpToken(id: string): Promise<void> {
  const response = await fetch(`/api/admin/mcp/tokens/${encodeURIComponent(id)}`, {
    method: "DELETE"
  });
  await ensureOk(response);
}

export async function listMcpActivity(): Promise<McpActivityRecord[]> {
  const response = await fetch("/api/admin/mcp/activity", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<McpActivityRecord[]>;
}

export type McpOAuthClientSummary = {
  id: string;
  clientId: string;
  clientName: string;
  redirectUris: string[];
  clientUri?: string | null;
  scopes: string[];
  isActive: boolean;
  active?: boolean;
  createdAt: string;
  revokedAt?: string | null;
  lastUsedAt?: string | null;
};

export type CreatedMcpOAuthClient = {
  client: McpOAuthClientSummary;
  clientSecret: string;
};

export async function listMcpOAuthClients(): Promise<McpOAuthClientSummary[]> {
  const response = await fetch("/api/admin/mcp/oauth-clients", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<McpOAuthClientSummary[]>;
}

export async function createMcpOAuthClient(
  clientName: string,
  redirectUris: string[],
  scopes: string[] = ["read"]
): Promise<CreatedMcpOAuthClient> {
  const response = await fetch("/api/admin/mcp/oauth-clients", {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json"
    },
    body: JSON.stringify({ clientName: clientName.trim(), redirectUris, scopes })
  });
  await ensureOk(response);
  return response.json() as Promise<CreatedMcpOAuthClient>;
}

export async function revokeMcpOAuthClient(id: string): Promise<void> {
  const response = await fetch(`/api/admin/mcp/oauth-clients/${encodeURIComponent(id)}`, {
    method: "DELETE"
  });
  await ensureOk(response);
}
