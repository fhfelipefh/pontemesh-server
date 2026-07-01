import { ReactNode, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, Ban, Copy, Download, KeyRound, Network, Plus, ShieldCheck, Upload, Wrench } from "lucide-react";
import {
  ApplicationCredentialSummary,
  CreatedApplicationCredential,
  createApplicationCredential,
  listApplicationCredentials,
  revokeApplicationCredential
} from "../api/applicationCredentialsApi";
import {
  CreatedS3AccessKey,
  S3AccessKeySummary,
  createS3AccessKey,
  listS3AccessKeys,
  revokeS3AccessKey
} from "../api/s3KeysApi";
import {
  CreatedMcpAccessToken,
  McpAccessTokenSummary,
  McpActivityRecord,
  McpSettings,
  McpStatus,
  createMcpToken,
  getMcpSettings,
  getMcpStatus,
  listMcpActivity,
  listMcpTokens,
  revokeMcpToken,
  updateMcpSettings
} from "../api/mcpApi";
import { ConfigurationImportResult, exportConfiguration, importConfiguration } from "../api/configurationApi";
import { S3CredentialsCard } from "../components/settings/S3CredentialsCard";

const S3_KEYS_PAGE_SIZE = 10;

export function SettingsPage() {
  const { t } = useTranslation();
  const [keys, setKeys] = useState<S3AccessKeySummary[]>([]);
  const [createdKey, setCreatedKey] = useState<CreatedS3AccessKey | null>(null);
  const [keyName, setKeyName] = useState("default-admin-key");
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [revoking, setRevoking] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [currentPage, setCurrentPage] = useState(1);
  const [totalKeys, setTotalKeys] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [applications, setApplications] = useState<ApplicationCredentialSummary[]>([]);
  const [createdApplication, setCreatedApplication] = useState<CreatedApplicationCredential | null>(null);
  const [applicationName, setApplicationName] = useState("default-sdk");
  const [loadingApplications, setLoadingApplications] = useState(true);
  const [creatingApplication, setCreatingApplication] = useState(false);
  const [revokingApplication, setRevokingApplication] = useState<string | null>(null);
  const [applicationError, setApplicationError] = useState("");
  const [mcpSettings, setMcpSettings] = useState<McpSettings | null>(null);
  const [mcpStatus, setMcpStatus] = useState<McpStatus | null>(null);
  const [mcpTokens, setMcpTokens] = useState<McpAccessTokenSummary[]>([]);
  const [mcpActivity, setMcpActivity] = useState<McpActivityRecord[]>([]);
  const [mcpTokenName, setMcpTokenName] = useState("default-mcp-client");
  const [createdMcpToken, setCreatedMcpToken] = useState<CreatedMcpAccessToken | null>(null);
  const [loadingMcp, setLoadingMcp] = useState(true);
  const [savingMcp, setSavingMcp] = useState(false);
  const [creatingMcpToken, setCreatingMcpToken] = useState(false);
  const [revokingMcpToken, setRevokingMcpToken] = useState<string | null>(null);
  const [mcpError, setMcpError] = useState("");
  const [configurationImporting, setConfigurationImporting] = useState(false);
  const [configurationResult, setConfigurationResult] = useState<ConfigurationImportResult | null>(null);
  const [configurationError, setConfigurationError] = useState("");

  const refreshKeys = useCallback(async (page: number) => {
    setLoading(true);
    setError("");
    try {
      const result = await listS3AccessKeys(page, S3_KEYS_PAGE_SIZE);
      setKeys(result.items);
      setCurrentPage(result.page);
      setTotalKeys(result.total);
      setTotalPages(result.totalPages);
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : t("setup.settings.s3.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshKeys(1);
  }, [refreshKeys]);

  const refreshApplications = useCallback(async () => {
    setLoadingApplications(true);
    setApplicationError("");
    try {
      setApplications(await listApplicationCredentials());
    } catch (loadError) {
      setApplicationError(loadError instanceof Error ? loadError.message : t("setup.settings.applications.loadFailed"));
    } finally {
      setLoadingApplications(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshApplications();
  }, [refreshApplications]);

  const refreshMcp = useCallback(async () => {
    setLoadingMcp(true);
    setMcpError("");
    try {
      const [settings, status, tokens, activity] = await Promise.all([
        getMcpSettings(),
        getMcpStatus(),
        listMcpTokens(),
        listMcpActivity()
      ]);
      setMcpSettings(settings);
      setMcpStatus(status);
      setMcpTokens(tokens);
      setMcpActivity(activity);
    } catch (loadError) {
      setMcpError(loadError instanceof Error ? loadError.message : t("setup.settings.mcp.loadFailed"));
    } finally {
      setLoadingMcp(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshMcp();
  }, [refreshMcp]);

  async function handleCreateKey() {
    setCreating(true);
    setError("");
    try {
      const nextKey = await createS3AccessKey(keyName);
      setCreatedKey(nextKey);
      await refreshKeys(1);
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : t("setup.settings.s3.createFailed"));
    } finally {
      setCreating(false);
    }
  }

  async function handleRevokeKey(id: string) {
    setRevoking(id);
    setError("");
    try {
      await revokeS3AccessKey(id);
      await refreshKeys(currentPage);
    } catch (revokeError) {
      setError(revokeError instanceof Error ? revokeError.message : t("setup.settings.s3.revokeFailed"));
    } finally {
      setRevoking(null);
    }
  }

  async function handleCreateApplication() {
    if (!applicationName.trim()) {
      return;
    }
    setCreatingApplication(true);
    setApplicationError("");
    try {
      const created = await createApplicationCredential(applicationName.trim());
      setCreatedApplication(created);
      setApplicationName("");
      await refreshApplications();
    } catch (createError) {
      setApplicationError(createError instanceof Error ? createError.message : t("setup.settings.applications.createFailed"));
    } finally {
      setCreatingApplication(false);
    }
  }

  async function handleRevokeApplication(id: string) {
    setRevokingApplication(id);
    setApplicationError("");
    try {
      await revokeApplicationCredential(id);
      await refreshApplications();
    } catch (revokeError) {
      setApplicationError(revokeError instanceof Error ? revokeError.message : t("setup.settings.applications.revokeFailed"));
    } finally {
      setRevokingApplication(null);
    }
  }

  async function handleUpdateMcpSettings(nextSettings: McpSettings) {
    setSavingMcp(true);
    setMcpError("");
    try {
      const saved = await updateMcpSettings({
        enabled: nextSettings.enabled,
        endpointPath: nextSettings.endpointPath,
        bindHost: nextSettings.bindHost,
        requireAuth: nextSettings.requireAuth,
        readToolsEnabled: nextSettings.readToolsEnabled,
        writeToolsEnabled: nextSettings.writeToolsEnabled,
        exposeResources: nextSettings.exposeResources,
        exposePrompts: nextSettings.exposePrompts,
        allowLocalhostOnly: nextSettings.allowLocalhostOnly
      });
      setMcpSettings(saved);
      setMcpStatus(await getMcpStatus());
    } catch (saveError) {
      setMcpError(saveError instanceof Error ? saveError.message : t("setup.settings.mcp.saveFailed"));
    } finally {
      setSavingMcp(false);
    }
  }

  async function handleCreateMcpToken() {
    if (!mcpTokenName.trim()) {
      return;
    }
    setCreatingMcpToken(true);
    setMcpError("");
    try {
      const created = await createMcpToken(mcpTokenName);
      setCreatedMcpToken(created);
      setMcpTokenName("");
      setMcpTokens(await listMcpTokens());
    } catch (createError) {
      setMcpError(createError instanceof Error ? createError.message : t("setup.settings.mcp.createTokenFailed"));
    } finally {
      setCreatingMcpToken(false);
    }
  }

  async function handleRevokeMcpToken(id: string) {
    setRevokingMcpToken(id);
    setMcpError("");
    try {
      await revokeMcpToken(id);
      setMcpTokens(await listMcpTokens());
    } catch (revokeError) {
      setMcpError(revokeError instanceof Error ? revokeError.message : t("setup.settings.mcp.revokeTokenFailed"));
    } finally {
      setRevokingMcpToken(null);
    }
  }

  async function handleExportConfiguration() {
    setConfigurationError("");
    try {
      const blob = await exportConfiguration();
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `pontemesh-configuration-${new Date().toISOString().slice(0, 10)}.json`;
      link.click();
      URL.revokeObjectURL(url);
    } catch (exportError) {
      setConfigurationError(exportError instanceof Error ? exportError.message : t("setup.settings.configuration.exportFailed"));
    }
  }

  async function handleImportConfiguration(file: File | null) {
    if (!file) {
      return;
    }
    setConfigurationImporting(true);
    setConfigurationError("");
    setConfigurationResult(null);
    try {
      const result = await importConfiguration(file);
      setConfigurationResult(result);
      await Promise.all([refreshMcp(), refreshKeys(currentPage)]);
    } catch (importError) {
      setConfigurationError(importError instanceof Error ? importError.message : t("setup.settings.configuration.importFailed"));
    } finally {
      setConfigurationImporting(false);
    }
  }

  return (
    <div className="settings-page">
      <header className="settings-page__header">
        <div>
          <h1>{t("setup.settings.title")}</h1>
        </div>
      </header>

      <div className="settings-page__grid">
        <ConfigurationBackupCard
          importing={configurationImporting}
          result={configurationResult}
          error={configurationError}
          onExport={() => void handleExportConfiguration()}
          onImport={(file) => void handleImportConfiguration(file)}
        />
        <McpSettingsCard
          settings={mcpSettings}
          status={mcpStatus}
          tokens={mcpTokens}
          activity={mcpActivity}
          tokenName={mcpTokenName}
          createdToken={createdMcpToken}
          loading={loadingMcp}
          saving={savingMcp}
          creatingToken={creatingMcpToken}
          revokingToken={revokingMcpToken}
          error={mcpError}
          onTokenNameChange={setMcpTokenName}
          onUpdateSettings={handleUpdateMcpSettings}
          onCreateToken={handleCreateMcpToken}
          onDismissCreatedToken={() => setCreatedMcpToken(null)}
          onRevokeToken={handleRevokeMcpToken}
        />
        <ApplicationCredentialsCard
          applications={applications}
          createdApplication={createdApplication}
          applicationName={applicationName}
          loading={loadingApplications}
          creating={creatingApplication}
          revoking={revokingApplication}
          error={applicationError}
          onApplicationNameChange={setApplicationName}
          onCreateApplication={handleCreateApplication}
          onDismissCreatedApplication={() => setCreatedApplication(null)}
          onRevokeApplication={handleRevokeApplication}
        />
        <S3CredentialsCard
          keys={keys}
          createdKey={createdKey}
          keyName={keyName}
          loading={loading}
          creating={creating}
          revoking={revoking}
          error={error}
          currentPage={currentPage}
          pageSize={S3_KEYS_PAGE_SIZE}
          totalKeys={totalKeys}
          totalPages={totalPages}
          onKeyNameChange={setKeyName}
          onCreateKey={handleCreateKey}
          onDismissCreatedKey={() => setCreatedKey(null)}
          onPageChange={(page) => void refreshKeys(page)}
          onRevokeKey={handleRevokeKey}
        />
      </div>
    </div>
  );
}

type ConfigurationBackupCardProps = {
  importing: boolean;
  result: ConfigurationImportResult | null;
  error: string;
  onExport: () => void;
  onImport: (file: File | null) => void;
};

function ConfigurationBackupCard({ importing, result, error, onExport, onImport }: ConfigurationBackupCardProps) {
  const { t } = useTranslation();

  return (
    <section className="settings-card settings-card--wide">
      <div className="settings-card__header">
        <div className="settings-card__title-group">
          <div className="settings-card__title-icon">
            <Download size={20} aria-hidden="true" />
          </div>
          <div>
            <h2>{t("setup.settings.configuration.title")}</h2>
          </div>
        </div>
      </div>

      <div className="settings-actions-row">
        <button className="settings-create-key-button" type="button" onClick={onExport}>
          <Download size={17} aria-hidden="true" />
          {t("setup.settings.configuration.export")}
        </button>
        <label className="settings-file-button">
          <Upload size={17} aria-hidden="true" />
          <span>{importing ? t("setup.common.loading") : t("setup.settings.configuration.import")}</span>
          <input
            type="file"
            accept="application/json,.json"
            disabled={importing}
            onChange={(event) => {
              onImport(event.currentTarget.files?.[0] ?? null);
              event.currentTarget.value = "";
            }}
          />
        </label>
      </div>

      {error ? <p className="error-message">{error}</p> : null}
      {result ? (
        <p className="settings-inline-status">
          {t("setup.settings.configuration.imported", {
            count: result.appliedBucketPolicies,
            skipped: result.skippedBucketPolicies.length
          })}
        </p>
      ) : null}
    </section>
  );
}

type McpSettingsCardProps = {
  settings: McpSettings | null;
  status: McpStatus | null;
  tokens: McpAccessTokenSummary[];
  activity: McpActivityRecord[];
  tokenName: string;
  createdToken: CreatedMcpAccessToken | null;
  loading: boolean;
  saving: boolean;
  creatingToken: boolean;
  revokingToken: string | null;
  error: string;
  onTokenNameChange: (value: string) => void;
  onUpdateSettings: (settings: McpSettings) => void;
  onCreateToken: () => void;
  onDismissCreatedToken: () => void;
  onRevokeToken: (id: string) => void;
};

function McpSettingsCard({
  settings,
  status,
  tokens,
  activity,
  tokenName,
  createdToken,
  loading,
  saving,
  creatingToken,
  revokingToken,
  error,
  onTokenNameChange,
  onUpdateSettings,
  onCreateToken,
  onDismissCreatedToken,
  onRevokeToken
}: McpSettingsCardProps) {
  const { t, i18n } = useTranslation();

  function update(patch: Partial<McpSettings>) {
    if (!settings || saving) {
      return;
    }
    onUpdateSettings({ ...settings, ...patch });
  }

  return (
    <section className="settings-card settings-card--wide" id="mcp">
      <div className="settings-card__header">
        <div className="settings-card__title-group">
          <div className="settings-card__title-icon">
            <Network size={20} aria-hidden="true" />
          </div>
          <div>
            <h2>{t("setup.settings.mcp.title")}</h2>
            <p>{t("setup.settings.mcp.subtitle")}</p>
          </div>
        </div>
      </div>

      {error ? <p className="error-message">{error}</p> : null}

      {loading || !settings || !status ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : !settings.enabled ? (
        <div className="mcp-settings-grid mcp-settings-grid--single">
          <ToggleRow label={t("setup.settings.mcp.enable")} checked={settings.enabled} disabled={saving} onChange={(checked) => update({ enabled: checked })} />
        </div>
      ) : (
        <>
          <div className="mcp-summary-grid">
            <McpSummaryItem icon={<Activity size={17} />} label={t("setup.settings.mcp.status")} value={status.enabled ? t("setup.settings.mcp.enabled") : t("setup.settings.mcp.disabled")} />
            <McpSummaryItem icon={<Network size={17} />} label={t("setup.settings.mcp.endpoint")} value={status.endpoint} />
            <McpSummaryItem icon={<ShieldCheck size={17} />} label={t("setup.settings.mcp.accessMode")} value={status.writeToolsEnabled ? t("setup.settings.mcp.readWrite") : t("setup.settings.mcp.readOnly")} />
            <McpSummaryItem icon={<Wrench size={17} />} label={t("setup.settings.mcp.lastActivity")} value={status.lastActivityAt ? formatDate(status.lastActivityAt, i18n.language) : t("setup.common.unavailable")} />
          </div>

          <div className="mcp-settings-grid">
            <ToggleRow label={t("setup.settings.mcp.enable")} checked={settings.enabled} disabled={saving} onChange={(checked) => update({ enabled: checked })} />
            <ToggleRow label={t("setup.settings.mcp.requireAuth")} checked={settings.requireAuth} disabled />
            <ToggleRow label={t("setup.settings.mcp.localhostOnly")} checked={settings.allowLocalhostOnly} disabled={saving} onChange={(checked) => update({ allowLocalhostOnly: checked })} />
            <ToggleRow label={t("setup.settings.mcp.readTools")} checked={settings.readToolsEnabled} disabled={saving} onChange={(checked) => update({ readToolsEnabled: checked })} />
            <ToggleRow label={t("setup.settings.mcp.writeTools")} checked={settings.writeToolsEnabled} disabled={saving} onChange={(checked) => update({ writeToolsEnabled: checked })} />
            <ToggleRow label={t("setup.settings.mcp.resources")} checked={settings.exposeResources} disabled={saving} onChange={(checked) => update({ exposeResources: checked })} />
            <ToggleRow label={t("setup.settings.mcp.prompts")} checked={settings.exposePrompts} disabled={saving} onChange={(checked) => update({ exposePrompts: checked })} />
          </div>

          <form className="inline-form" onSubmit={(event) => {
            event.preventDefault();
            onCreateToken();
          }}>
            <input
              value={tokenName}
              onChange={(event) => onTokenNameChange(event.target.value)}
              placeholder={t("setup.settings.mcp.tokenNamePlaceholder")}
              aria-label={t("setup.settings.mcp.tokenName")}
            />
            <button className="settings-create-key-button" type="submit" disabled={creatingToken || !tokenName.trim()}>
              <Plus size={17} aria-hidden="true" />
              {t("setup.settings.mcp.createToken")}
            </button>
          </form>

          {createdToken ? (
            <section className="secret-panel" role="status">
              <strong>{t("setup.settings.mcp.tokenCreated")}</strong>
              <p>{t("setup.settings.mcp.tokenCreatedHint")}</p>
              <dl>
                <div>
                  <dt>{t("setup.settings.mcp.tokenPrefix")}</dt>
                  <dd><code>{createdToken.token.tokenPrefix}</code></dd>
                </div>
                <div>
                  <dt>{t("setup.settings.mcp.tokenSecret")}</dt>
                  <dd>
                    <code>{createdToken.secret}</code>
                    <button className="icon-button" type="button" title={t("setup.settings.mcp.copyToken")} aria-label={t("setup.settings.mcp.copyToken")} onClick={() => void navigator.clipboard?.writeText(createdToken.secret)}>
                      <Copy size={16} aria-hidden="true" />
                    </button>
                  </dd>
                </div>
              </dl>
              <button className="settings-secondary-button" type="button" onClick={onDismissCreatedToken}>
                {t("setup.common.ok")}
              </button>
            </section>
          ) : null}

          <div className="settings-table-wrap">
            <table className="settings-table">
              <thead>
                <tr>
                  <th>{t("setup.settings.mcp.tokenName")}</th>
                  <th>{t("setup.settings.mcp.tokenPrefix")}</th>
                  <th>{t("setup.settings.s3.status")}</th>
                  <th>{t("setup.settings.s3.lastUsed")}</th>
                  <th>{t("setup.settings.s3.createdAt")}</th>
                  <th aria-label={t("setup.settings.s3.actions")} />
                </tr>
              </thead>
              <tbody>
                {tokens.length === 0 ? (
                  <tr>
                    <td colSpan={6}>{t("setup.settings.mcp.noTokens")}</td>
                  </tr>
                ) : tokens.map((token) => (
                  <tr key={token.id}>
                    <td className="settings-table__name">{token.name}</td>
                    <td><code>{token.tokenPrefix}</code></td>
                    <td>{token.active ? t("setup.settings.s3.active") : t("setup.settings.s3.revoked")}</td>
                    <td>{token.lastUsedAt ? formatDate(token.lastUsedAt, i18n.language) : t("setup.common.unavailable")}</td>
                    <td>{formatDate(token.createdAt, i18n.language)}</td>
                    <td className="settings-table__actions">
                      {token.active ? (
                        <button className="settings-revoke-button" type="button" title={t("setup.settings.mcp.revokeToken")} aria-label={t("setup.settings.mcp.revokeToken")} disabled={revokingToken === token.id} onClick={() => onRevokeToken(token.id)}>
                          <Ban size={16} aria-hidden="true" />
                        </button>
                      ) : null}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <section className="mcp-activity">
            <h3>{t("setup.settings.mcp.recentActivity")}</h3>
            {activity.length === 0 ? (
              <p>{t("setup.settings.mcp.noActivity")}</p>
            ) : (
              <ol>
                {activity.slice(0, 8).map((entry) => (
                  <li key={entry.id}>
                    <span>{entry.method}</span>
                    <strong>{entry.outcome}</strong>
                    <time dateTime={entry.createdAt}>{formatDate(entry.createdAt, i18n.language)}</time>
                  </li>
                ))}
              </ol>
            )}
          </section>
        </>
      )}
    </section>
  );
}

function McpSummaryItem({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="mcp-summary-item">
      {icon}
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function ToggleRow({ label, checked, disabled = false, onChange }: { label: string; checked: boolean; disabled?: boolean; onChange?: (checked: boolean) => void }) {
  return (
    <div className="settings-toggle-row">
      <span>{label}</span>
      <button type="button" role="switch" aria-checked={checked} aria-label={label} title={label} disabled={disabled} onClick={() => onChange?.(!checked)}>
        <span aria-hidden="true" />
      </button>
    </div>
  );
}

type ApplicationCredentialsCardProps = {
  applications: ApplicationCredentialSummary[];
  createdApplication: CreatedApplicationCredential | null;
  applicationName: string;
  loading: boolean;
  creating: boolean;
  revoking: string | null;
  error: string;
  onApplicationNameChange: (value: string) => void;
  onCreateApplication: () => void;
  onDismissCreatedApplication: () => void;
  onRevokeApplication: (id: string) => void;
};

function ApplicationCredentialsCard({
  applications,
  createdApplication,
  applicationName,
  loading,
  creating,
  revoking,
  error,
  onApplicationNameChange,
  onCreateApplication,
  onDismissCreatedApplication,
  onRevokeApplication
}: ApplicationCredentialsCardProps) {
  const { t, i18n } = useTranslation();

  return (
    <section className="settings-card">
      <div className="settings-card__header">
        <div className="settings-card__title-group">
          <div className="settings-card__title-icon">
            <KeyRound size={20} aria-hidden="true" />
          </div>
          <div>
            <h2>{t("setup.settings.applications.title")}</h2>
          </div>
        </div>
      </div>

      <form className="inline-form" onSubmit={(event) => {
        event.preventDefault();
        onCreateApplication();
      }}>
        <input
          value={applicationName}
          onChange={(event) => onApplicationNameChange(event.target.value)}
          placeholder={t("setup.settings.applications.namePlaceholder")}
          aria-label={t("setup.settings.applications.name")}
        />
        <button className="settings-create-key-button" type="submit" disabled={creating || !applicationName.trim()}>
          <Plus size={17} aria-hidden="true" />
          {t("setup.settings.applications.create")}
        </button>
      </form>

      {error ? <p className="error-message">{error}</p> : null}

      {createdApplication ? (
        <section className="secret-panel" role="status">
          <strong>{t("setup.settings.applications.createdTitle")}</strong>
          <dl>
            <div>
              <dt>{t("setup.settings.applications.applicationId")}</dt>
              <dd><code>{createdApplication.credential.id}</code></dd>
            </div>
            <div>
              <dt>{t("setup.settings.applications.token")}</dt>
              <dd>
                <code>{createdApplication.token}</code>
                <button className="icon-button" type="button" title={t("setup.settings.applications.copyToken")} aria-label={t("setup.settings.applications.copyToken")} onClick={() => void navigator.clipboard?.writeText(createdApplication.token)}>
                  <Copy size={16} aria-hidden="true" />
                </button>
              </dd>
            </div>
          </dl>
          <button className="settings-secondary-button" type="button" onClick={onDismissCreatedApplication}>
            {t("setup.common.ok")}
          </button>
        </section>
      ) : null}

      {loading ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : applications.length === 0 ? (
        <div className="settings-empty-state">
          <h3>{t("setup.settings.applications.emptyTitle")}</h3>
          <p>{t("setup.settings.applications.emptyDescription")}</p>
        </div>
      ) : (
        <div className="settings-table-wrap">
          <table className="settings-table">
            <thead>
              <tr>
                <th>{t("setup.settings.applications.name")}</th>
                <th>{t("setup.settings.applications.scopes")}</th>
                <th>{t("setup.settings.s3.status")}</th>
                <th>{t("setup.settings.s3.createdAt")}</th>
                <th aria-label={t("setup.settings.s3.actions")} />
              </tr>
            </thead>
            <tbody>
              {applications.map((application) => (
                <tr key={application.id}>
                  <td className="settings-table__name">{application.name}</td>
                  <td>{application.scopes.join(", ")}</td>
                  <td>{application.revoked ? t("setup.settings.s3.revoked") : t("setup.settings.s3.active")}</td>
                  <td>{formatDate(application.createdAt, i18n.language)}</td>
                  <td className="settings-table__actions">
                    {!application.revoked ? (
                      <button className="settings-revoke-button" type="button" title={t("setup.settings.applications.revoke")} aria-label={t("setup.settings.applications.revoke")} disabled={revoking === application.id} onClick={() => onRevokeApplication(application.id)}>
                        <Ban size={16} aria-hidden="true" />
                      </button>
                    ) : null}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function formatDate(value: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "short"
  }).format(new Date(value));
}
