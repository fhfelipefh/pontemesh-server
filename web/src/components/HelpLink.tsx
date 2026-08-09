import { BookOpen } from "lucide-react";
import { useTranslation } from "react-i18next";

const DOCS_URL = "https://github.com/fhfelipefh/pontemesh-docs";

export function HelpLink() {
  const { t } = useTranslation();

  return (
    <p className="help-link">
      <a href={DOCS_URL} target="_blank" rel="noreferrer">
        <BookOpen size={17} aria-hidden="true" />
        {t("setup.help.readDocs")}
      </a>
    </p>
  );
}
