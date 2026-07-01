import { ensureOk } from "./http";

export type ConfigurationImportResult = {
  appliedMcpSettings: boolean;
  appliedBucketPolicies: number;
  skippedBucketPolicies: string[];
};

export async function exportConfiguration(): Promise<Blob> {
  const response = await fetch("/api/admin/configuration", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.blob();
}

export async function importConfiguration(file: File): Promise<ConfigurationImportResult> {
  const text = await file.text();
  const parsed = JSON.parse(text) as unknown;
  const response = await fetch("/api/admin/configuration", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(parsed)
  });
  await ensureOk(response);
  return response.json() as Promise<ConfigurationImportResult>;
}
