import { Check, ChevronDown, Globe2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { supportedLanguages } from "../i18n/languages";

export function LanguageSwitcher() {
  const { i18n, t } = useTranslation();
  const [open, setOpen] = useState(false);
  const activeLanguage =
    supportedLanguages.find((language) => language.code === i18n.language) ?? supportedLanguages[0];

  function changeLanguage(language: string) {
    void i18n.changeLanguage(language);
    setOpen(false);
  }

  return (
    <div className="language-switcher">
      <button
        className="language-switcher__button"
        type="button"
        aria-label={t("setup.language.label")}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((currentOpen) => !currentOpen)}
      >
        <Globe2 size={18} aria-hidden="true" />
        <span>{activeLanguage.shortLabel}</span>
        <ChevronDown size={16} aria-hidden="true" />
      </button>

      {open ? (
        <div className="language-switcher__menu" role="menu">
          {supportedLanguages.map((language) => {
            const selected = language.code === activeLanguage.code;
            return (
              <button
                key={language.code}
                className="language-switcher__item"
                type="button"
                role="menuitemradio"
                aria-checked={selected}
                onClick={() => changeLanguage(language.code)}
              >
                <span>
                  <strong>{language.shortLabel}</strong>
                  <small>{language.label}</small>
                </span>
                {selected ? <Check size={17} aria-hidden="true" /> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
