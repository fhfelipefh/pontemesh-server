import { describe, expect, it } from "vitest";
import { emptyObjectsPage, emptyPage, formatBytes } from "./adminFormat";

describe("emptyPage", () => {
  it("returns a page with the given page size and sensible defaults", () => {
    const page = emptyPage(50);
    expect(page.items).toEqual([]);
    expect(page.page).toBe(1);
    expect(page.pageSize).toBe(50);
    expect(page.totalItems).toBe(0);
    expect(page.totalPages).toBe(1);
  });
});

describe("emptyObjectsPage", () => {
  it("returns an objects page with empty commonPrefixes and given page size", () => {
    const page = emptyObjectsPage(20);
    expect(page.items).toEqual([]);
    expect(page.commonPrefixes).toEqual([]);
    expect(page.page).toBe(1);
    expect(page.pageSize).toBe(20);
    expect(page.totalItems).toBe(0);
    expect(page.totalPages).toBe(1);
  });
});

describe("formatBytes", () => {
  it("formats zero bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("formats bytes without decimal for values under 1 KB", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("formats kilobytes with one decimal place", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
  });

  it("formats megabytes", () => {
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
  });

  it("formats gigabytes", () => {
    expect(formatBytes(1024 ** 3)).toBe("1.0 GB");
  });
});
