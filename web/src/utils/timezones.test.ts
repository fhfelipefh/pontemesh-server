import { describe, expect, it } from "vitest";
import { formatTimezoneCity, getTimezoneOffsetString, getTimezoneOptions } from "./timezones";

describe("timezones utility", () => {
  it("formats city name correctly", () => {
    expect(formatTimezoneCity("UTC")).toBe("UTC");
    expect(formatTimezoneCity("America/Sao_Paulo")).toBe("Sao Paulo");
    expect(formatTimezoneCity("Europe/London")).toBe("London");
    expect(formatTimezoneCity("America/Argentina/Buenos_Aires")).toBe("Buenos Aires");
  });

  it("formats timezone offset string correctly", () => {
    expect(getTimezoneOffsetString("UTC")).toBe("UTC+00:00");
    const offsetSaoPaulo = getTimezoneOffsetString("America/Sao_Paulo", new Date("2026-06-01T00:00:00Z"));
    expect(offsetSaoPaulo).toMatch(/^UTC-03:00$/);
  });

  it("generates timezone options with labels and search keys", () => {
    const options = getTimezoneOptions(new Date("2026-06-01T00:00:00Z"));
    expect(options.length).toBeGreaterThan(10);

    const utc = options.find((opt) => opt.value === "UTC");
    expect(utc).toBeDefined();
    expect(utc?.label).toBe("UTC (UTC+00:00)");

    const sp = options.find((opt) => opt.value === "America/Sao_Paulo");
    expect(sp).toBeDefined();
    expect(sp?.label).toBe("Sao Paulo (UTC-03:00)");
  });
});
