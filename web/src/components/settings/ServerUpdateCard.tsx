import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ServerUpdateStatus } from "../../api/serverUpdateApi";
import { Button } from "../Button";
import { InfoBox } from "./InfoBox";
import { SettingsSection } from "./SettingsSection";

type ServerUpdateCardProps = {
  status: ServerUpdateStatus | null;
  loading: boolean;
  requesting: boolean;
  error: string;
  restartPending: boolean;
  onUpdate: () => void;
};

export function ServerUpdateCard({ status, loading, requesting, error, restartPending, onUpdate }: ServerUpdateCardProps) {
  const { t } = useTranslation();
  const updateAvailable = status?.updateAvailable === true;

  return (
    <SettingsSection className="settings-card--wide" title={t("setup.settings.update.title")} icon={<RefreshCw size={20} />}>
      {loading ? <p>{t("setup.common.loading")}</p> : null}
      {!loading && error ? <p className="error-message">{error}</p> : null}
      {!loading && status ? (
        <InfoBox variant={updateAvailable ? "info" : "success"}>
          {updateAvailable
            ? t("setup.settings.update.available", { version: status.latestVersion })
            : t("setup.settings.update.current", { version: status.currentVersion })}
        </InfoBox>
      ) : null}
      {restartPending ? <InfoBox variant="warning">{t("setup.settings.update.restartPending")}</InfoBox> : null}
      {updateAvailable && !restartPending ? (
        <div className="settings-actions-row">
          <Button
            data-testid="request-server-update"
            type="button"
            loading={requesting}
            disabled={!status?.automaticUpdateEnabled || requesting}
            icon={<RefreshCw size={17} aria-hidden="true" />}
            onClick={onUpdate}
          >
            {t("setup.settings.update.action")}
          </Button>
          {!status?.automaticUpdateEnabled ? <span className="settings-muted-text">{t("setup.settings.update.notConfigured")}</span> : null}
        </div>
      ) : null}
    </SettingsSection>
  );
}
