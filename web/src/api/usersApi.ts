import { ensureOk } from "./http";

export type AdminUserSummary = { id: string; username: string; role: string; createdAt: string; lastLoginAt: string | null };

export async function listAdminUsers(): Promise<AdminUserSummary[]> {
  const response = await fetch("/api/admin/users", { headers: { accept: "application/json" } });
  await ensureOk(response);
  return response.json() as Promise<AdminUserSummary[]>;
}

export async function createAdminUser(payload: { username: string; password: string; currentPassword: string; role: string }): Promise<void> {
  const response = await fetch("/api/admin/users", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(payload) });
  await ensureOk(response);
}

export async function deleteAdminUser(id: string): Promise<void> {
  const response = await fetch(`/api/admin/users/${id}`, { method: "DELETE" });
  await ensureOk(response);
}

export async function updateMyCredentials(payload: { username: string; currentPassword: string; newPassword: string }): Promise<void> {
  const response = await fetch("/api/admin/users/me/credentials", { method: "PUT", headers: { "content-type": "application/json" }, body: JSON.stringify(payload) });
  await ensureOk(response);
}
