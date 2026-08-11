import { ensureOk } from "./http";

export type OperationalWebhookSettings = {
  enabled: boolean;
  url: string;
  cron: string;
  payloadPreview: Record<string, unknown>;
};

export type OperationalWebhookUpdate = Pick<OperationalWebhookSettings, "enabled" | "url" | "cron">;

export async function getOperationalWebhook(): Promise<OperationalWebhookSettings> {
  const response = await fetch("/api/admin/operational-webhook", {
    headers: { accept: "application/json" }
  });
  await ensureOk(response);
  return response.json() as Promise<OperationalWebhookSettings>;
}

export async function updateOperationalWebhook(
  settings: OperationalWebhookUpdate
): Promise<OperationalWebhookSettings> {
  const response = await fetch("/api/admin/operational-webhook", {
    method: "PUT",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(settings)
  });
  await ensureOk(response);
  return response.json() as Promise<OperationalWebhookSettings>;
}
