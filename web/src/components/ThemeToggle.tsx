import { Moon, Sun } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { applyTheme, resolveInitialTheme, saveTheme, ThemePreference } from "../theme";

export function ThemeToggle() {
  const { t } = useTranslation();
  const [theme, setTheme] = useState<ThemePreference>(() => resolveInitialTheme());
  const nextTheme: ThemePreference = theme === "dark" ? "light" : "dark";
  const label = theme === "dark" ? t("setup.theme.switchToLight") : t("setup.theme.switchToDark");

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  function toggleTheme() {
    setTheme(nextTheme);
    saveTheme(nextTheme);
  }

  return (
    <button
      className="theme-toggle"
      data-testid="theme-toggle"
      type="button"
      aria-label={label}
      title={label}
      onClick={toggleTheme}
    >
      {theme === "dark" ? <Sun size={18} aria-hidden="true" /> : <Moon size={18} aria-hidden="true" />}
    </button>
  );
}
