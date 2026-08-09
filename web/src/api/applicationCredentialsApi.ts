import { ensureOk } from "./http";

export type ApplicationCredentialSummary = {
  id: string;
  name: string;
  scopes: string[];
  createdAt: string;
  revoked: boolean;
};

export type CreatedApplicationCredential = {
  credential: ApplicationCredentialSummary;
  token: string;
};

export async function listApplicationCredentials(): Promise<ApplicationCredentialSummary[]> {
  const response = await fetch("/api/admin/application-credentials", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ApplicationCredentialSummary[]>;
}

export async function createApplicationCredential(
  name: string,
  scopes?: string[],
  preset: "downloader" | "full" = "downloader"
): Promise<CreatedApplicationCredential> {
  const response = await fetch("/api/admin/application-credentials", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(scopes ? { name, scopes } : { name, preset })
  });
  await ensureOk(response);
  return response.json() as Promise<CreatedApplicationCredential>;
}

export async function revokeApplicationCredential(id: string): Promise<void> {
  const response = await fetch(`/api/admin/application-credentials/${encodeURIComponent(id)}/revoke`, {
    method: "POST"
  });
  await ensureOk(response);
}
