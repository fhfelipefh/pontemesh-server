import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ShieldCheck, Save } from "lucide-react";
import { OidcSettings, OidcSettingsUpdate, updateOidcSettings } from "../../api/oidcApi";
import { Button } from "../Button";
import { SettingsSection } from "./SettingsSection";
import { ToggleRow } from "./ToggleRow";

type OidcSettingsCardProps = {
  settings: OidcSettings | null;
  onSettingsUpdated: (settings: OidcSettings) => void;
};

export function OidcSettingsCard({ settings, onSettingsUpdated }: OidcSettingsCardProps) {
  const { t } = useTranslation();
  const [enabled, setEnabled] = useState(settings?.enabled ?? false);
  const [issuerUrl, setIssuerUrl] = useState(settings?.issuerUrl ?? "");
  const [clientId, setClientId] = useState(settings?.clientId ?? "");
  const [clientSecret, setClientSecret] = useState(settings?.clientSecret ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const hasChanges =
    (settings?.enabled ?? false) !== enabled ||
    (settings?.issuerUrl ?? "") !== issuerUrl ||
    (settings?.clientId ?? "") !== clientId ||
    (settings?.clientSecret ?? "") !== clientSecret;

  async function handleSave(event: React.FormEvent) {
    event.preventDefault();
    if (!hasChanges) {
      return;
    }
    setError("");
    setSaving(true);
    try {
      const update: OidcSettingsUpdate = {
        enabled,
        issuerUrl: issuerUrl.trim() || null,
        clientId: clientId.trim() || null,
        clientSecret: clientSecret.trim() || null,
      };
      const updated = await updateOidcSettings(update);
      onSettingsUpdated(updated);
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : "Failed to update OIDC settings");
    } finally {
      setSaving(false);
    }
  }

  if (!settings) {
    return (
      <SettingsSection className="settings-card--wide" title="OIDC Authentication (Keycloak)" icon={<ShieldCheck size={20} />}>
        <div className="settings-loading">{t("setup.common.loading")}</div>
      </SettingsSection>
    );
  }

  return (
    <SettingsSection className="settings-card--wide" title="OIDC Authentication (Keycloak)" icon={<ShieldCheck size={20} />}>
      <form className="settings-form" onSubmit={(e) => { void handleSave(e); }}>
        <ToggleRow
          label="Enable OIDC Authentication"
          checked={enabled}
          onChange={setEnabled}
        />

        <div className="settings-form__field">
          <label htmlFor="oidcIssuerUrl">Issuer URL</label>
          <input
            id="oidcIssuerUrl"
            type="url"
            value={issuerUrl}
            onChange={(e) => setIssuerUrl(e.target.value)}
            placeholder="https://keycloak.example.com/realms/master"
            disabled={!enabled}
            required={enabled}
          />
        </div>

        <div className="settings-form__field">
          <label htmlFor="oidcClientId">Client ID</label>
          <input
            id="oidcClientId"
            type="text"
            value={clientId}
            onChange={(e) => setClientId(e.target.value)}
            placeholder="pontemesh"
            disabled={!enabled}
            required={enabled}
          />
        </div>

        <div className="settings-form__field">
          <label htmlFor="oidcClientSecret">Client Secret</label>
          <input
            id="oidcClientSecret"
            type="password"
            value={clientSecret}
            onChange={(e) => setClientSecret(e.target.value)}
            placeholder="Client Secret"
            disabled={!enabled}
            required={enabled}
          />
        </div>

        {error ? <p className="error-message">{error}</p> : null}

        <div className="settings-actions-row">
          <Button
            type="submit"
            disabled={!hasChanges || saving}
            loading={saving}
            icon={<Save size={18} aria-hidden="true" />}
          >
            {t("setup.common.save")}
          </Button>
        </div>
      </form>
    </SettingsSection>
  );
}
