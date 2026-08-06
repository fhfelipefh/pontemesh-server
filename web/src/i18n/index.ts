import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import enSetup from "./locales/en/setup.json";
import ptBrSetup from "./locales/pt-BR/setup.json";
import { LANGUAGE_STORAGE_KEY, resolveInitialLanguage } from "./languages";

i18next.use(initReactI18next).init({
  resources: {
    en: {
      translation: enSetup
    },
    "pt-BR": {
      translation: ptBrSetup
    }
  },
  lng: resolveInitialLanguage(),
  fallbackLng: "en",
  interpolation: {
    escapeValue: false
  }
});

i18next.on("languageChanged", (language) => {
  window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
  document.documentElement.lang = language;
});

document.documentElement.lang = i18next.language;

export default i18next;
