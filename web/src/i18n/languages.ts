export const LANGUAGE_STORAGE_KEY = "pontemesh.language";

export const supportedLanguages = [
  {
    code: "en",
    shortLabel: "EN",
    label: "English"
  },
  {
    code: "pt-BR",
    shortLabel: "PT-BR",
    label: "Português (Brasil)"
  }
] as const;

export type SupportedLanguageCode = (typeof supportedLanguages)[number]["code"];

export function isSupportedLanguage(language: string): language is SupportedLanguageCode {
  return supportedLanguages.some((supportedLanguage) => supportedLanguage.code === language);
}

export function resolveInitialLanguage(): SupportedLanguageCode {
  const storedLanguage = window.localStorage.getItem(LANGUAGE_STORAGE_KEY);
  if (storedLanguage && isSupportedLanguage(storedLanguage)) {
    return storedLanguage;
  }

  const browserLanguage = navigator.language;
  if (isSupportedLanguage(browserLanguage)) {
    return browserLanguage;
  }

  const browserBaseLanguage = browserLanguage.split("-")[0];
  if (isSupportedLanguage(browserBaseLanguage)) {
    return browserBaseLanguage;
  }

  return "en";
}
