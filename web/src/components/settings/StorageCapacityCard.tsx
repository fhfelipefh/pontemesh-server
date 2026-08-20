import { useTranslation } from "react-i18next";
import { Check, HardDrive, Save } from "lucide-react";
import { DiskGuardSettings } from "../../api/storageApi";
import { Button } from "../Button";
import { SettingsSection } from "./SettingsSection";
import { ToggleRow } from "./ToggleRow";

export function StorageCapacityCard({
  settings,
  loading,
  saving,
  saved,
  error,
  isAdmin,
  onChange,
  onToggle,
  onSave,
}: {
  settings: DiskGuardSettings | null;
  loading: boolean;
  saving: boolean;
  saved: boolean;
  error: string;
  isAdmin: boolean;
  onChange: (settings: DiskGuardSettings) => void;
  onToggle: (enabled: boolean) => void;
  onSave: () => void;
}) {
  const { t } = useTranslation();
  const thresholdsValid =
    settings !== null &&
    settings.warningPercent >= 0 &&
    settings.warningPercent < settings.degradedPercent &&
    settings.degradedPercent < settings.blockPercent &&
    settings.blockPercent <= 100;

  return (
    <SettingsSection
      className="settings-card--wide"
      title={t("setup.settings.storage.title")}
      description={t("setup.settings.storage.description")}
      icon={<HardDrive size={20} />}
    >
      {error ? <p className="error-message">{error}</p> : null}
      {loading || !settings ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : (
        <form
          className="storage-capacity-form"
          onSubmit={event => {
            event.preventDefault();
            onSave();
          }}
        >
          {!settings.enabled ? (
            <div className="mcp-settings-grid mcp-settings-grid--single">
              <ToggleRow
                label={t("setup.settings.storage.enabled")}
                checked={settings.enabled}
                disabled={saving || !isAdmin}
                onChange={onToggle}
              />
            </div>
          ) : (
            <>
              <div className="storage-capacity-form__summary">
                <ToggleRow
                  label={t("setup.settings.storage.enabled")}
                  checked={settings.enabled}
                  disabled={saving || !isAdmin}
                  onChange={onToggle}
                />
                <div className="storage-capacity-form__usage">
                  <span>{t("setup.settings.storage.currentUsage")}</span>
                  <strong>
                    {settings.usedPercent === null
                      ? t("setup.common.unavailable")
                      : `${settings.usedPercent.toFixed(1)}%`}
                  </strong>
                </div>
              </div>
              <div className="storage-capacity-form__thresholds">
                <StorageThresholdField
                  id="storage-warning-percent"
                  label={t("setup.settings.storage.warningPercent")}
                  value={settings.warningPercent}
                  disabled={saving || !settings.enabled || !isAdmin}
                  onChange={warningPercent =>
                    onChange({ ...settings, warningPercent })
                  }
                />
                <StorageThresholdField
                  id="storage-degraded-percent"
                  label={t("setup.settings.storage.degradedPercent")}
                  value={settings.degradedPercent}
                  disabled={saving || !settings.enabled || !isAdmin}
                  onChange={degradedPercent =>
                    onChange({ ...settings, degradedPercent })
                  }
                />
                <StorageThresholdField
                  id="storage-block-percent"
                  label={t("setup.settings.storage.blockPercent")}
                  value={settings.blockPercent}
                  disabled={saving || !settings.enabled || !isAdmin}
                  onChange={blockPercent =>
                    onChange({ ...settings, blockPercent })
                  }
                />
              </div>
              {!thresholdsValid ? (
                <p className="settings-warning">
                  {t("setup.settings.storage.invalidThresholds")}
                </p>
              ) : null}
              <div className="storage-capacity-form__actions">
                {saved ? (
                  <span className="storage-capacity-form__saved" role="status">
                    <Check size={16} aria-hidden="true" />
                    {t("setup.settings.storage.saved")}
                  </span>
                ) : null}
                <Button
                  data-testid="save-storage-capacity"
                  type="submit"
                  loading={saving}
                  disabled={saving || !thresholdsValid || !isAdmin}
                  icon={<Save size={17} aria-hidden="true" />}
                >
                  {t("setup.common.save")}
                </Button>
              </div>
            </>
          )}
        </form>
      )}
    </SettingsSection>
  );
}

function StorageThresholdField({
  id,
  label,
  value,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <label htmlFor={id}>
      <span>{label}</span>
      <div>
        <input
          id={id}
          type="number"
          min={0}
          max={100}
          step={0.1}
          required
          value={value}
          disabled={disabled}
          onChange={event => onChange(Number(event.target.value))}
        />
        <span aria-hidden="true">%</span>
      </div>
    </label>
  );
}
