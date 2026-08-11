import { describe, expect, it } from "vitest";
import { buildBreadcrumb, navigateUpFrom, prefixLabel } from "./objectNavigation";

describe("buildBreadcrumb", () => {
  it("returns empty array for empty prefix", () => {
    expect(buildBreadcrumb("")).toEqual([]);
  });

  it("returns single segment for one-level prefix", () => {
    expect(buildBreadcrumb("folder/")).toEqual([
      { prefix: "folder/", label: "folder" }
    ]);
  });

  it("returns cumulative prefixes for nested path", () => {
    expect(buildBreadcrumb("a/b/c/")).toEqual([
      { prefix: "a/", label: "a" },
      { prefix: "a/b/", label: "b" },
      { prefix: "a/b/c/", label: "c" }
    ]);
  });

  it("handles prefix without trailing slash", () => {
    expect(buildBreadcrumb("a/b")).toEqual([
      { prefix: "a/", label: "a" },
      { prefix: "a/b/", label: "b" }
    ]);
  });
});

describe("prefixLabel", () => {
  it("returns the segment name relative to current prefix", () => {
    expect(prefixLabel("a/b/", "a/")).toBe("b");
  });

  it("returns full prefix stripped of trailing slash when current prefix is empty", () => {
    expect(prefixLabel("folder/", "")).toBe("folder");
  });

  it("returns full prefix when it does not start with current prefix", () => {
    expect(prefixLabel("x/y/", "a/")).toBe("x/y/");
  });

  it("returns full prefix when relative part is empty", () => {
    expect(prefixLabel("a/", "a/")).toBe("a/");
  });
});

describe("navigateUpFrom", () => {
  it("returns empty string from empty prefix", () => {
    expect(navigateUpFrom("")).toBe("");
  });

  it("returns empty string when navigating up from a single-level prefix", () => {
    expect(navigateUpFrom("folder/")).toBe("");
  });

  it("returns parent prefix when navigating up from nested path", () => {
    expect(navigateUpFrom("a/b/c/")).toBe("a/b/");
    expect(navigateUpFrom("a/b/")).toBe("a/");
  });
});
