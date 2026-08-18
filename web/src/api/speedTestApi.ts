import { ensureOk } from "./http";

export const MIN_DOWNLOAD_BYTES = 1 * 1024 * 1024;
export const MAX_DOWNLOAD_BYTES = 256 * 1024 * 1024;
export const MIN_UPLOAD_BYTES = 1 * 1024 * 1024;
export const MAX_UPLOAD_BYTES = 512 * 1024 * 1024;
export const TARGET_ROUND_MS = 2500;

export type SpeedTestResult = {
  bytes: number;
  durationMs: number;
  mbps: number;
};

export function bytesToMbps(bytes: number, durationMs: number): number {
  if (durationMs <= 0) {
    return 0;
  }
  return (bytes * 8) / durationMs / 1000;
}

export function nextPayloadSize(
  result: SpeedTestResult,
  previousBytes: number,
  minBytes: number,
  maxBytes: number
): number {
  const targetBytes = Math.round((result.mbps * TARGET_ROUND_MS * 1000) / 8);
  if (targetBytes <= 0) {
    return previousBytes;
  }
  return Math.min(maxBytes, Math.max(minBytes, targetBytes));
}

function xorshift32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
  };
}

function randomBody(
  bytes: number,
  onProgress?: (sent: number) => void
): ReadableStream<Uint8Array> {
  const chunkSize = 64 * 1024;
  let remaining = bytes;
  const next = xorshift32(0x9e3779b9);
  return new ReadableStream<Uint8Array>({
    pull(controller) {
      if (remaining <= 0) {
        controller.close();
        return;
      }
      const chunk = new Uint8Array(Math.min(chunkSize, remaining));
      for (let index = 0; index < chunk.length; index += 4) {
        const value = next();
        chunk[index] = value & 0xff;
        if (index + 1 < chunk.length) {
          chunk[index + 1] = (value >>> 8) & 0xff;
        }
        if (index + 2 < chunk.length) {
          chunk[index + 2] = (value >>> 16) & 0xff;
        }
        if (index + 3 < chunk.length) {
          chunk[index + 3] = (value >>> 24) & 0xff;
        }
      }
      remaining -= chunk.length;
      controller.enqueue(chunk);
      onProgress?.(bytes - remaining);
    }
  });
}

export async function downloadPayload(
  bytes: number,
  onProgress?: (received: number) => void
): Promise<SpeedTestResult> {
  const startedAt = performance.now();
  const response = await fetch(`/api/admin/speed-test/download?size=${bytes}`, {
    headers: { accept: "application/octet-stream" }
  });
  await ensureOk(response);
  if (!response.body) {
    throw new Error("response body is unavailable");
  }
  const reader = response.body.getReader();
  let received = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    received += value.byteLength;
    onProgress?.(received);
  }
  const durationMs = performance.now() - startedAt;
  return { bytes: received, durationMs, mbps: bytesToMbps(received, durationMs) };
}

export async function uploadPayload(
  bytes: number,
  onProgress?: (sent: number) => void
): Promise<SpeedTestResult> {
  const startedAt = performance.now();
  const response = await fetch("/api/admin/speed-test/upload", {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/octet-stream"
    },
    body: randomBody(bytes, onProgress)
  });
  await ensureOk(response);
  const { bytesReceived } = (await response.json()) as { bytesReceived: number };
  const durationMs = performance.now() - startedAt;
  return { bytes: bytesReceived, durationMs, mbps: bytesToMbps(bytesReceived, durationMs) };
}