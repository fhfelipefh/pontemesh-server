import { afterEach, describe, expect, it, vi } from "vitest";
import {
  THEME_STORAGE_KEY,
  applyTheme,
  isThemePreference,
  resolveInitialTheme,
  saveTheme,
} from "./theme";

describe("theme", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  describe("isThemePreference", () => {
    it("returns true for valid themes", () => {
      expect(isThemePreference("light")).toBe(true);
      expect(isThemePreference("dark")).toBe(true);
    });

    it("returns false for invalid themes", () => {
      expect(isThemePreference("system")).toBe(false);
      expect(isThemePreference("")).toBe(false);
      expect(isThemePreference("LIGHT")).toBe(false);
    });
  });

  describe("resolveInitialTheme", () => {
    it("returns stored theme if valid", () => {
      let storedValue: string | null = "dark";
      const getItem = vi.fn((key) => key === THEME_STORAGE_KEY ? storedValue : null);
      vi.stubGlobal("window", { localStorage: { getItem } });

      expect(resolveInitialTheme()).toBe("dark");

      storedValue = "light";
      expect(resolveInitialTheme()).toBe("light");
    });

    it("falls back to matchMedia dark if storage is missing or invalid", () => {
      const getItem = vi.fn().mockReturnValue("invalid");
      const matchMedia = vi.fn().mockImplementation((query) => ({
        matches: query === "(prefers-color-scheme: dark)",
      }));
      vi.stubGlobal("window", { localStorage: { getItem }, matchMedia });

      expect(resolveInitialTheme()).toBe("dark");
    });

    it("falls back to light if matchMedia does not match dark", () => {
      const getItem = vi.fn().mockReturnValue(null);
      const matchMedia = vi.fn().mockImplementation(() => ({
        matches: false,
      }));
      vi.stubGlobal("window", { localStorage: { getItem }, matchMedia });

      expect(resolveInitialTheme()).toBe("light");
    });
  });

  describe("applyTheme", () => {
    it("applies theme to document element", () => {
      const dataset = { theme: "" };
      const style = { colorScheme: "" };
      vi.stubGlobal("document", {
        documentElement: { dataset, style },
      });

      applyTheme("dark");
      expect(dataset.theme).toBe("dark");
      expect(style.colorScheme).toBe("dark");
    });
  });

  describe("saveTheme", () => {
    it("saves theme to local storage and applies it", () => {
      const setItem = vi.fn();
      vi.stubGlobal("window", { localStorage: { setItem } });

      const dataset = { theme: "" };
      const style = { colorScheme: "" };
      vi.stubGlobal("document", {
        documentElement: { dataset, style },
      });

      saveTheme("dark");
      expect(setItem).toHaveBeenCalledWith(THEME_STORAGE_KEY, "dark");
      expect(dataset.theme).toBe("dark");
      expect(style.colorScheme).toBe("dark");
    });
  });
});
