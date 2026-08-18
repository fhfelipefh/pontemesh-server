import { afterEach, describe, expect, it, vi } from "vitest";
import {
  bytesToMbps,
  downloadPayload,
  MAX_DOWNLOAD_BYTES,
  MIN_DOWNLOAD_BYTES,
  nextPayloadSize,
  uploadPayload
} from "./speedTestApi";

describe("speedTestApi", () => {
  afterEach(() => vi.restoreAllMocks());

  it("converts bytes and duration into Mbps using decimal bits", () => {
    expect(bytesToMbps(1_250_000, 1000)).toBe(10);
    expect(bytesToMbps(0, 1000)).toBe(0);
    expect(bytesToMbps(1_000_000, 0)).toBe(0);
  });

  it("scales the next payload size toward the target duration", () => {
    const fast = nextPayloadSize({ bytes: 1_000_000, durationMs: 1000, mbps: 8 }, 1_000_000, MIN_DOWNLOAD_BYTES, MAX_DOWNLOAD_BYTES);
    expect(fast).toBe(2_500_000);

    const slow = nextPayloadSize({ bytes: 1_000_000, durationMs: 10_000, mbps: 0.8 }, 1_000_000, MIN_DOWNLOAD_BYTES, MAX_DOWNLOAD_BYTES);
    expect(slow).toBe(MIN_DOWNLOAD_BYTES);

    const huge = nextPayloadSize({ bytes: 1_000_000, durationMs: 10, mbps: 2000 }, 1_000_000, MIN_DOWNLOAD_BYTES, MAX_DOWNLOAD_BYTES);
    expect(huge).toBe(MAX_DOWNLOAD_BYTES);
  });

  it("downloads the requested payload and measures real transferred bytes", async () => {
    const body = new Uint8Array(512 * 1024).fill(7);
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(body);
        controller.close();
      }
    });
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(new Response(stream, { status: 200 }));

    const onProgress = vi.fn();
    const result = await downloadPayload(body.byteLength, onProgress);

    expect(fetchMock).toHaveBeenCalledWith(
      `/api/admin/speed-test/download?size=${body.byteLength}`,
      { headers: { accept: "application/octet-stream" } }
    );
    expect(result.bytes).toBe(body.byteLength);
    expect(result.durationMs).toBeGreaterThanOrEqual(0);
    expect(result.mbps).toBe(bytesToMbps(body.byteLength, result.durationMs));
    expect(onProgress).toHaveBeenLastCalledWith(body.byteLength);
  });

  it("downloads throws HttpError for a failed response", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ error: "unauthorized" }), {
        status: 401,
        headers: { "content-type": "application/json" }
      })
    );

    await expect(downloadPayload(1024)).rejects.toThrow("unauthorized");
  });

  it("uploads a streamed payload and uses the reported bytesReceived", async () => {
    const jsonResponse = new Response(JSON.stringify({ bytesReceived: 2048 }), {
      status: 200,
      headers: { "content-type": "application/json" }
    });
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(jsonResponse);

    const onProgress = vi.fn();
    const result = await uploadPayload(2048, onProgress);

    expect(fetchMock).toHaveBeenCalledWith("/api/admin/speed-test/upload", {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/octet-stream"
      },
      body: expect.any(ReadableStream)
    });
    expect(result.bytes).toBe(2048);
    expect(result.mbps).toBe(bytesToMbps(2048, result.durationMs));
    expect(onProgress).toHaveBeenLastCalledWith(2048);
  });
});