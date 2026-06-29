import { ensureOk } from "./http";

export type ReplicaSummary = {
  id: string;
  name: string;
  allowedBuckets: string[];
  createdAt: string;
  revoked: boolean;
};

export type CreatedReplicaCredential = {
  replica: ReplicaSummary;
  token: string;
};

export async function listReplicas(): Promise<ReplicaSummary[]> {
  const response = await fetch("/api/admin/replicas", {
    headers: {
      accept: "application/json"
    }
  });
  await ensureOk(response);
  return response.json() as Promise<ReplicaSummary[]>;
}

export async function createReplicaCredential(
  name: string,
  allowedBuckets: string[]
): Promise<CreatedReplicaCredential> {
  const response = await fetch("/api/admin/replicas", {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify({ name, allowedBuckets })
  });
  await ensureOk(response);
  return response.json() as Promise<CreatedReplicaCredential>;
}

export async function revokeReplica(replicaId: string): Promise<void> {
  const response = await fetch(`/api/admin/replicas/${encodeURIComponent(replicaId)}/revoke`, {
    method: "POST"
  });
  await ensureOk(response);
}
