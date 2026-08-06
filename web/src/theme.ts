export const THEME_STORAGE_KEY = "pontemesh.theme";

export const themes = ["light", "dark"] as const;

export type ThemePreference = (typeof themes)[number];

export function isThemePreference(value: string): value is ThemePreference {
  return themes.includes(value as ThemePreference);
}

export function resolveInitialTheme(): ThemePreference {
  const storedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (storedTheme && isThemePreference(storedTheme)) {
    return storedTheme;
  }

  if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
    return "dark";
  }

  return "light";
}

export function applyTheme(theme: ThemePreference): void {
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
}

export function saveTheme(theme: ThemePreference): void {
  window.localStorage.setItem(THEME_STORAGE_KEY, theme);
  applyTheme(theme);
}
