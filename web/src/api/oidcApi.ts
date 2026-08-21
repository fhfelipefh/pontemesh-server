import { ensureOk } from "./http";

export type OidcSettings = {
  enabled: boolean;
  issuerUrl: string | null;
  clientId: string | null;
  clientSecret: string | null;
};

export type OidcSettingsUpdate = Pick<OidcSettings, "enabled" | "issuerUrl" | "clientId" | "clientSecret">;

export async function getOidcSettings(): Promise<OidcSettings> {
  const response = await fetch("/api/admin/oidc/settings", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<OidcSettings>;
}

export async function updateOidcSettings(payload: OidcSettingsUpdate): Promise<OidcSettings> {
  const response = await fetch("/api/admin/oidc/settings", {
    method: "PUT",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(payload)
  });
  await ensureOk(response);
  return response.json() as Promise<OidcSettings>;
}
