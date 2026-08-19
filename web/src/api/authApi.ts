import { ensureOk } from "./http";

export type AuthUser = {
  authenticated: boolean;
  username: string | null;
  role?: string;
};

export type LoginRequest = {
  username: string;
  password: string;
};

export async function login(payload: LoginRequest): Promise<AuthUser> {
  const response = await fetch("/api/auth/login", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(payload)
  });
  await ensureOk(response);
  return response.json() as Promise<AuthUser>;
}

export async function logout(): Promise<void> {
  const response = await fetch("/api/auth/logout", {
    method: "POST"
  });
  await ensureOk(response);
}

export async function getCurrentUser(): Promise<AuthUser> {
  const response = await fetch("/api/auth/me", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<AuthUser>;
}
