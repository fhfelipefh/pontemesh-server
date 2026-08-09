export type UnlockRequest = {
  token: string;
};

export type CompleteSetupRequest = {
  instanceName: string;
  role: "origin" | "replica-edge";
  adminUsername: string;
  adminPassword: string;
  httpPort?: number;
  s3Port?: number;
  publicWebUrl?: string;
  publicS3Url?: string;
  internalStoragePath?: string;
  originBaseUrl?: string;
  replicaId?: string;
  replicaToken?: string;
  replicaPublicEndpoint?: string;
};

export type SetupStatusResponse = {
  setupRequired: boolean;
  serverVersion: string;
  internalWebPort: number;
  internalS3Port: number;
};

export type InitialS3AccessKey = {
  id: string;
  name: string | null;
  accessKeyId: string;
  secretAccessKey: string;
  createdAt: string;
};

export type CompleteSetupResponse = {
  ready: boolean;
  initialS3AccessKey: InitialS3AccessKey | null;
};

export async function getSetupStatus(): Promise<SetupStatusResponse> {
  const response = await fetch("/api/setup/status", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<SetupStatusResponse>;
}

export async function unlockSetup(payload: UnlockRequest): Promise<void> {
  const response = await fetch("/api/setup/unlock", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(payload)
  });
  await ensureOk(response);
}

export async function completeSetup(payload: CompleteSetupRequest): Promise<CompleteSetupResponse> {
  const response = await fetch("/api/setup/complete", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(payload)
  });
  await ensureOk(response);
  return response.json() as Promise<CompleteSetupResponse>;
}

async function ensureOk(response: Response): Promise<void> {
  if (response.ok) {
    return;
  }

  const body = await response.json().catch(() => null) as { error?: string } | null;
  throw new Error(body?.error ?? "Request failed");
}
