import { useTranslation } from "react-i18next";
import { PageShell } from "../components/PageShell";

export function ReadyPage() {
  const { t } = useTranslation();

  return <PageShell title={t("setup.ready.title")} description={t("setup.ready.description")} />;
}
