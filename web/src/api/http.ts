export class HttpError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "HttpError";
    this.status = status;
  }
}

export async function ensureOk(response: Response): Promise<void> {
  if (response.ok) {
    return;
  }

  const body = await response.json().catch(() => null) as { error?: string } | null;
  throw new HttpError(response.status, body?.error ?? "Request failed");
}
