import { describe, expect, it } from "vitest";
import { isValidAdminPassword } from "./adminPassword";

describe("isValidAdminPassword", () => {
  it("accepts strong ASCII and Unicode passwords", () => {
    expect(isValidAdminPassword("PonteMesh123!")).toBe(true);
    expect(isValidAdminPassword("ÁrvoreSegura１２!x")).toBe(true);
  });

  it("rejects a password missing any required character class", () => {
    expect(isValidAdminPassword("pontemesh123!")).toBe(false);
    expect(isValidAdminPassword("PONTEMESH123!")).toBe(false);
    expect(isValidAdminPassword("PonteMeshAdmin!")).toBe(false);
    expect(isValidAdminPassword("PonteMesh1234")).toBe(false);
  });

  it("accepts the setup token-sized password and rejects oversized values", () => {
    expect(isValidAdminPassword("pm_init_aa8sRcjUfTvQV3Ud1lNxaf1zc9uCm1eNYw-HVY5VDiM")).toBe(true);
    expect(isValidAdminPassword(`PonteMesh123!${"a".repeat(244)}`)).toBe(false);
  });
});
