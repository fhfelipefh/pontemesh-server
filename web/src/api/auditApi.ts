import { ensureOk } from "./http";

export type AuditEvent = {
  id: string;
  event: string;
  principal: string | null;
  outcome: string;
  detail: string;
  createdAt: string;
};

export type AuditEventFilters = {
  event?: string;
  principal?: string;
  outcome?: string;
  since?: string;
  until?: string;
  limit?: number;
};

export async function listAuditEvents(filters: AuditEventFilters = {}): Promise<AuditEvent[]> {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    if (value !== undefined && value !== "") {
      params.set(key, String(value));
    }
  }
  const query = params.toString();
  const response = await fetch(`/api/admin/audit-events${query ? `?${query}` : ""}`, {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<AuditEvent[]>;
}
