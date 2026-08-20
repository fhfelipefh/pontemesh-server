import { useTranslation } from "react-i18next";
import { Download, Upload } from "lucide-react";
import { ConfigurationImportResult } from "../../api/configurationApi";
import { Button } from "../Button";
import { SettingsSection } from "./SettingsSection";
import { InfoBox } from "./InfoBox";

export type ConfigurationBackupCardProps = {
  importing: boolean;
  result: ConfigurationImportResult | null;
  error: string;
  onExport: () => void;
  onImport: (file: File | null) => void;
};

export function ConfigurationBackupCard({
  importing,
  result,
  error,
  onExport,
  onImport,
}: ConfigurationBackupCardProps) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      className="settings-card--wide"
      title={t("setup.settings.configuration.title")}
      icon={<Download size={20} />}
    >
      <div className="settings-actions-row">
        <Button
          className="settings-create-key-button"
          type="button"
          variant="secondary"
          icon={<Download size={17} aria-hidden="true" />}
          onClick={onExport}
        >
          {t("setup.settings.configuration.export")}
        </Button>
        <label className="settings-file-button">
          <Upload size={17} aria-hidden="true" />
          <span>
            {importing
              ? t("setup.common.loading")
              : t("setup.settings.configuration.import")}
          </span>
          <input
            type="file"
            accept="application/json,.json"
            disabled={importing}
            onChange={event => {
              onImport(event.currentTarget.files?.[0] ?? null);
              event.currentTarget.value = "";
            }}
          />
        </label>
      </div>

      {error ? <p className="error-message">{error}</p> : null}
      {result ? (
        <InfoBox variant="success">
          <p>
            {t("setup.settings.configuration.imported", {
              count: result.appliedBucketPolicies,
              skipped: result.skippedBucketPolicies.length,
            })}
          </p>
        </InfoBox>
      ) : null}
    </SettingsSection>
  );
}
