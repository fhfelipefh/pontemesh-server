import { Braces, Check, Save, Webhook } from "lucide-react";
import { useTranslation } from "react-i18next";
import { OperationalWebhookSettings } from "../../api/webhookApi";
import { Button } from "../Button";
import { SettingsSection } from "./SettingsSection";
import { ToggleRow } from "./ToggleRow";

type OperationalWebhookCardProps = {
  settings: OperationalWebhookSettings | null;
  loading: boolean;
  saving: boolean;
  saved: boolean;
  error: string;
  onChange: (settings: OperationalWebhookSettings) => void;
  onToggle: (enabled: boolean) => void;
  onSave: () => void;
};

export function OperationalWebhookCard({
  settings,
  loading,
  saving,
  saved,
  error,
  onChange,
  onToggle,
  onSave
}: OperationalWebhookCardProps) {
  const { t } = useTranslation();
  const canSave = settings !== null
    && settings.cron.trim().split(/\s+/).length === 5
    && (!settings.enabled || settings.url.trim().length > 0);

  return (
    <SettingsSection
      className="settings-card--wide"
      title={t("setup.settings.webhook.title")}
      description={t("setup.settings.webhook.description")}
      icon={<Webhook size={20} />}
    >
      {error ? <p className="error-message">{error}</p> : null}
      {loading || !settings ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : (
        <form
          className="operational-webhook-form"
          onSubmit={(event) => {
            event.preventDefault();
            onSave();
          }}
        >
          <ToggleRow
            label={t("setup.settings.webhook.enabled")}
            checked={settings.enabled}
            disabled={saving}
            onChange={onToggle}
          />
          {settings.enabled ? (
            <>
              <div className="operational-webhook-form__fields">
                <label htmlFor="operational-webhook-url">
                  <span>{t("setup.settings.webhook.url")}</span>
                  <input
                    id="operational-webhook-url"
                    type="url"
                    maxLength={2048}
                    placeholder="https://automation.example.net/webhook/pontemesh"
                    value={settings.url}
                    disabled={saving}
                    required
                    onChange={(event) => onChange({ ...settings, url: event.target.value })}
                  />
                </label>
                <label htmlFor="operational-webhook-cron">
                  <span>{t("setup.settings.webhook.cron")}</span>
                  <input
                    id="operational-webhook-cron"
                    value={settings.cron}
                    disabled={saving}
                    required
                    placeholder="*/15 * * * *"
                    onChange={(event) => onChange({ ...settings, cron: event.target.value })}
                  />
                  <small>{t("setup.settings.webhook.cronHint")}</small>
                </label>
              </div>
              <details className="operational-webhook-payload">
                <summary>
                  <Braces size={17} aria-hidden="true" />
                  {t("setup.settings.webhook.payload")}
                </summary>
                <pre>{JSON.stringify(settings.payloadPreview, null, 2)}</pre>
              </details>
              <div className="operational-webhook-form__actions">
                {saved ? (
                  <span className="storage-capacity-form__saved" role="status">
                    <Check size={16} aria-hidden="true" />
                    {t("setup.settings.webhook.saved")}
                  </span>
                ) : null}
                <Button
                  data-testid="save-operational-webhook"
                  type="submit"
                  loading={saving}
                  disabled={saving || !canSave}
                  icon={<Save size={17} aria-hidden="true" />}
                >
                  {t("setup.common.save")}
                </Button>
              </div>
            </>
          ) : null}
        </form>
      )}
    </SettingsSection>
  );
}
