import { ensureOk } from "./http";

export type ServerUpdateStatus = {
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  updateAvailable: boolean;
  automaticUpdateEnabled: boolean;
};

export async function getServerUpdateStatus(): Promise<ServerUpdateStatus> {
  const response = await fetch("/api/admin/system/update", { headers: { accept: "application/json" } });
  await ensureOk(response);
  return response.json() as Promise<ServerUpdateStatus>;
}

export async function requestServerUpdate(): Promise<void> {
  const response = await fetch("/api/admin/system/update", {
    method: "POST",
    headers: { accept: "application/json" }
  });
  await ensureOk(response);
}
