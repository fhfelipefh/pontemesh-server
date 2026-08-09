import { ReactNode, useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, Ban, Check, Copy, Download, KeyRound, Network, Plus, ShieldCheck, Upload, Wrench } from "lucide-react";
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
import { Button } from "../components/Button";
import { ConfirmDialog } from "../components/AdminListControls";
import { CredentialTable } from "../components/settings/CredentialTable";
import { EmptyState } from "../components/settings/EmptyState";
import { IconButton } from "../components/settings/IconButton";
import { InfoBox } from "../components/settings/InfoBox";
import { S3CredentialsCard } from "../components/settings/S3CredentialsCard";
import { SettingsSection } from "../components/settings/SettingsSection";
import { StatusBadge } from "../components/settings/StatusBadge";

const S3_KEYS_PAGE_SIZE = 10;

type DestructiveConfirmation =
  | { kind: "s3Key"; id: string; name: string }
  | { kind: "application"; id: string; name: string }
  | { kind: "mcpToken"; id: string; name: string }
  | null;

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
  const [applicationPreset, setApplicationPreset] = useState<"downloader" | "full">("downloader");
  const [loadingApplications, setLoadingApplications] = useState(true);
  const [creatingApplication, setCreatingApplication] = useState(false);
  const [revokingApplication, setRevokingApplication] = useState<string | null>(null);
  const [applicationError, setApplicationError] = useState("");
  const [mcpSettings, setMcpSettings] = useState<McpSettings | null>(null);
  const [mcpStatus, setMcpStatus] = useState<McpStatus | null>(null);
  const [mcpTokens, setMcpTokens] = useState<McpAccessTokenSummary[]>([]);
  const [mcpActivity, setMcpActivity] = useState<McpActivityRecord[]>([]);
  const [mcpTokenName, setMcpTokenName] = useState("default-mcp-client");
  const [mcpTokenScopes, setMcpTokenScopes] = useState<string[]>(["read"]);
  const [createdMcpToken, setCreatedMcpToken] = useState<CreatedMcpAccessToken | null>(null);
  const [loadingMcp, setLoadingMcp] = useState(true);
  const [savingMcp, setSavingMcp] = useState(false);
  const [creatingMcpToken, setCreatingMcpToken] = useState(false);
  const [revokingMcpToken, setRevokingMcpToken] = useState<string | null>(null);
  const [mcpError, setMcpError] = useState("");
  const [configurationImporting, setConfigurationImporting] = useState(false);
  const [configurationResult, setConfigurationResult] = useState<ConfigurationImportResult | null>(null);
  const [configurationError, setConfigurationError] = useState("");
  const [destructiveConfirmation, setDestructiveConfirmation] = useState<DestructiveConfirmation>(null);

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
      setDestructiveConfirmation(null);
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
      const created = await createApplicationCredential(applicationName.trim(), undefined, applicationPreset);
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
      setDestructiveConfirmation(null);
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
        adminToolsEnabled: nextSettings.adminToolsEnabled,
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
      const created = await createMcpToken(mcpTokenName, mcpTokenScopes);
      setCreatedMcpToken(created);
      setMcpTokenName("");
      setMcpTokenScopes(["read"]);
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
      setDestructiveConfirmation(null);
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
        <McpSettingsCard
          settings={mcpSettings}
          status={mcpStatus}
          tokens={mcpTokens}
          activity={mcpActivity}
          tokenName={mcpTokenName}
          tokenScopes={mcpTokenScopes}
          onTokenScopesChange={setMcpTokenScopes}
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
          onRevokeToken={(id, name) => setDestructiveConfirmation({ kind: "mcpToken", id, name })}
        />
          <ApplicationCredentialsCard
          applications={applications}
          createdApplication={createdApplication}
            applicationName={applicationName}
            applicationPreset={applicationPreset}
          loading={loadingApplications}
          creating={creatingApplication}
          revoking={revokingApplication}
          error={applicationError}
            onApplicationNameChange={setApplicationName}
            onApplicationPresetChange={setApplicationPreset}
          onCreateApplication={handleCreateApplication}
          onDismissCreatedApplication={() => setCreatedApplication(null)}
          onRevokeApplication={(id, name) => setDestructiveConfirmation({ kind: "application", id, name })}
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
          onRevokeKey={(id, name) => setDestructiveConfirmation({ kind: "s3Key", id, name })}
        />
        <ConfigurationBackupCard
          importing={configurationImporting}
          result={configurationResult}
          error={configurationError}
          onExport={() => void handleExportConfiguration()}
          onImport={(file) => void handleImportConfiguration(file)}
        />
      </div>
      {destructiveConfirmation ? (
        <ConfirmDialog
          title={
            destructiveConfirmation.kind === "s3Key"
              ? t("setup.settings.s3.confirmRevokeTitle")
              : destructiveConfirmation.kind === "application"
                ? t("setup.settings.applications.confirmRevokeTitle")
                : t("setup.settings.mcp.confirmRevokeTokenTitle")
          }
          description={
            destructiveConfirmation.kind === "s3Key"
              ? t("setup.settings.s3.confirmRevokeDescription", { name: destructiveConfirmation.name })
              : destructiveConfirmation.kind === "application"
                ? t("setup.settings.applications.confirmRevokeDescription", { name: destructiveConfirmation.name })
                : t("setup.settings.mcp.confirmRevokeTokenDescription", { name: destructiveConfirmation.name })
          }
          onCancel={() => setDestructiveConfirmation(null)}
          onConfirm={() => {
            if (destructiveConfirmation.kind === "s3Key") {
              void handleRevokeKey(destructiveConfirmation.id);
              return;
            }
            if (destructiveConfirmation.kind === "application") {
              void handleRevokeApplication(destructiveConfirmation.id);
              return;
            }
            void handleRevokeMcpToken(destructiveConfirmation.id);
          }}
        />
      ) : null}
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
        <InfoBox variant="success">
          <p>
          {t("setup.settings.configuration.imported", {
            count: result.appliedBucketPolicies,
            skipped: result.skippedBucketPolicies.length
          })}
          </p>
        </InfoBox>
      ) : null}
    </SettingsSection>
  );
}

type McpSettingsCardProps = {
  settings: McpSettings | null;
  status: McpStatus | null;
  tokens: McpAccessTokenSummary[];
  activity: McpActivityRecord[];
  tokenName: string;
  tokenScopes: string[];
  onTokenScopesChange: (scopes: string[]) => void;
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
  onRevokeToken: (id: string, name: string) => void;
};

function McpSettingsCard({
  settings,
  status,
  tokens,
  activity,
  tokenName,
  tokenScopes,
  createdToken,
  loading,
  saving,
  creatingToken,
  revokingToken,
  error,
  onTokenNameChange,
  onTokenScopesChange,
  onUpdateSettings,
  onCreateToken,
  onDismissCreatedToken,
  onRevokeToken
}: McpSettingsCardProps) {
  const { t, i18n } = useTranslation();
  const mcpConnectionConfig = createdToken && settings && status
    ? JSON.stringify(buildMcpConnectionConfig(createdToken, settings, status), null, 2)
    : "";

  function update(patch: Partial<McpSettings>) {
    if (!settings || saving) {
      return;
    }
    onUpdateSettings({ ...settings, ...patch });
  }

  return (
    <SettingsSection
      className="settings-card--wide"
      id="mcp"
      title={t("setup.settings.mcp.title")}
      description={t("setup.settings.mcp.subtitle")}
      icon={<Network size={20} />}
    >

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
            <McpSummaryItem icon={<ShieldCheck size={17} />} label={t("setup.settings.mcp.accessMode")} value={status.adminToolsEnabled ? t("setup.settings.mcp.fullAdmin") : status.writeToolsEnabled ? t("setup.settings.mcp.readWrite") : t("setup.settings.mcp.readOnly")} />
            <McpSummaryItem icon={<Wrench size={17} />} label={t("setup.settings.mcp.lastActivity")} value={status.lastActivityAt ? formatDate(status.lastActivityAt, i18n.language) : t("setup.common.unavailable")} />
          </div>

          <div className="mcp-settings-grid">
            <ToggleRow label={t("setup.settings.mcp.enable")} checked={settings.enabled} disabled={saving} onChange={(checked) => update({ enabled: checked })} />
            <ToggleRow label={t("setup.settings.mcp.requireAuth")} checked={settings.requireAuth} disabled />
            <ToggleRow label={t("setup.settings.mcp.localhostOnly")} checked={settings.allowLocalhostOnly} disabled={saving} onChange={(checked) => update({ allowLocalhostOnly: checked })} />
            <ToggleRow label={t("setup.settings.mcp.readTools")} checked={settings.readToolsEnabled} disabled={saving} onChange={(checked) => update({ readToolsEnabled: checked })} />
            <ToggleRow label={t("setup.settings.mcp.writeTools")} checked={settings.writeToolsEnabled} disabled={saving} onChange={(checked) => update({ writeToolsEnabled: checked })} />
            <ToggleRow label={t("setup.settings.mcp.adminTools")} checked={settings.adminToolsEnabled} disabled={saving} onChange={(checked) => update({ adminToolsEnabled: checked })} />
            <ToggleRow label={t("setup.settings.mcp.resources")} checked={settings.exposeResources} disabled={saving} onChange={(checked) => update({ exposeResources: checked })} />
            <ToggleRow label={t("setup.settings.mcp.prompts")} checked={settings.exposePrompts} disabled={saving} onChange={(checked) => update({ exposePrompts: checked })} />
          </div>

          <form className="inline-form mcp-token-form" onSubmit={(event) => {
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
            <div className="mcp-token-scopes">
              <span className="mcp-token-scopes__label">{t("setup.settings.mcp.tokenScopes")}</span>
              <div className="settings-checkbox-group" role="group" aria-label={t("setup.settings.mcp.tokenScopes")} data-testid="mcp-token-scope-group">
                {["read", "write", "admin"].map((scope) => (
                  <label key={scope} className="settings-checkbox-field">
                    <input
                      type="checkbox"
                      checked={tokenScopes.includes(scope)}
                      disabled={scope === "read"}
                      data-testid={`mcp-token-scope-${scope}`}
                      onChange={(event) => {
                        const next = event.target.checked ? [...tokenScopes, scope] : tokenScopes.filter((item) => item !== scope);
                        onTokenScopesChange(Array.from(new Set(["read", ...next])));
                      }}
                    />
                    {scope}
                  </label>
                ))}
              </div>
            </div>
            {tokenScopes.some((scope) => scope === "write" || scope === "admin") ? <p className="settings-warning">{t("setup.settings.mcp.permissionWarning")}</p> : null}
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
                    <IconButton label={t("setup.settings.mcp.copyToken")} icon={<Copy size={16} aria-hidden="true" />} onClick={() => void navigator.clipboard?.writeText(createdToken.secret)} />
                  </dd>
                </div>
              </dl>
              <div className="mcp-connection-config">
                <div className="mcp-connection-config__header">
                  <strong>{t("setup.settings.mcp.connectionConfig")}</strong>
                  <IconButton
                    label={t("setup.settings.mcp.copyConnectionConfig")}
                    icon={<Copy size={16} aria-hidden="true" />}
                    onClick={() => void navigator.clipboard?.writeText(mcpConnectionConfig)}
                  />
                </div>
                <p>{t("setup.settings.mcp.connectionConfigHint")}</p>
                <pre>{mcpConnectionConfig}</pre>
              </div>
              <button className="settings-secondary-button" type="button" onClick={onDismissCreatedToken}>
                <Check size={16} aria-hidden="true" />
                {t("setup.common.ok")}
              </button>
            </section>
          ) : null}

          <CredentialTable
            columns={[
              { key: "name", label: t("setup.settings.mcp.tokenName"), className: "settings-table__col-name" },
              { key: "prefix", label: t("setup.settings.mcp.tokenPrefix"), className: "settings-table__col-key" },
              { key: "scopes", label: t("setup.settings.mcp.tokenScopes"), className: "settings-table__col-status" },
              { key: "status", label: t("setup.settings.s3.status"), className: "settings-table__col-status" },
              { key: "lastUsed", label: t("setup.settings.s3.lastUsed"), className: "settings-table__col-last-used" },
              { key: "createdAt", label: t("setup.settings.s3.createdAt"), className: "settings-table__col-created" },
              { key: "actions", ariaLabel: t("setup.settings.s3.actions"), className: "settings-table__col-actions" }
            ]}
          >
                {tokens.length === 0 ? (
                  <tr className="settings-table__empty-row">
                    <td colSpan={7}>
                      <EmptyState title={t("setup.settings.mcp.noTokens")} />
                    </td>
                  </tr>
                ) : tokens.map((token) => (
                  <tr key={token.id}>
                    <td className="settings-table__name">{token.name}</td>
                    <td><code>{token.tokenPrefix}</code></td>
                    <td>{formatScopes(token.scopes, t("setup.common.unavailable"))}</td>
                    <td>
                      <StatusBadge
                        active={token.active}
                        activeLabel={t("setup.settings.s3.active")}
                        revokedLabel={t("setup.settings.s3.revoked")}
                      />
                    </td>
                    <td>{token.lastUsedAt ? formatDate(token.lastUsedAt, i18n.language) : t("setup.common.unavailable")}</td>
                    <td>{formatDate(token.createdAt, i18n.language)}</td>
                    <td className="settings-table__actions">
                      {token.active ? (
                        <IconButton
                          variant="danger"
                          label={t("setup.settings.mcp.revokeToken")}
                          icon={<Ban size={16} aria-hidden="true" />}
                          disabled={revokingToken === token.id}
                          onClick={() => onRevokeToken(token.id, token.name)}
                        />
                      ) : null}
                    </td>
                  </tr>
                ))}
          </CredentialTable>

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
    </SettingsSection>
  );
}

function buildMcpConnectionConfig(
  createdToken: CreatedMcpAccessToken,
  settings: McpSettings,
  status: McpStatus
) {
  const url = absoluteMcpUrl(status.endpoint || settings.endpointPath);
  return {
    name: createdToken.token.name,
    transport: "streamable-http",
    url,
    method: "POST",
    headers: {
      Authorization: `Bearer ${createdToken.secret}`,
      "Content-Type": "application/json"
    },
    environment: {
      PONTEMESH_MCP_URL: url,
      PONTEMESH_MCP_TOKEN: createdToken.secret
    },
    initialize: {
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: "2025-06-18",
        capabilities: {},
        clientInfo: {
          name: "pontemesh-mcp-client",
          version: "1.0.0"
        }
      }
    },
    options: {
      localhostOnly: settings.allowLocalhostOnly,
      readToolsEnabled: settings.readToolsEnabled,
      writeToolsEnabled: settings.writeToolsEnabled,
      resourcesEnabled: settings.exposeResources,
      promptsEnabled: settings.exposePrompts
    }
  };
}

function absoluteMcpUrl(endpoint: string) {
  if (/^https?:\/\//i.test(endpoint)) {
    return endpoint;
  }
  const origin = typeof window === "undefined" ? "http://127.0.0.1:8080" : window.location.origin;
  return `${origin}${endpoint.startsWith("/") ? endpoint : `/${endpoint}`}`;
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
  applicationPreset: "downloader" | "full";
  loading: boolean;
  creating: boolean;
  revoking: string | null;
  error: string;
  onApplicationNameChange: (value: string) => void;
  onApplicationPresetChange: (value: "downloader" | "full") => void;
  onCreateApplication: () => void;
  onDismissCreatedApplication: () => void;
  onRevokeApplication: (id: string, name: string) => void;
};

function ApplicationCredentialsCard({
  applications,
  createdApplication,
  applicationName,
  applicationPreset,
  loading,
  creating,
  revoking,
  error,
  onApplicationNameChange,
  onApplicationPresetChange,
  onCreateApplication,
  onDismissCreatedApplication,
  onRevokeApplication
}: ApplicationCredentialsCardProps) {
  const { t, i18n } = useTranslation();

  return (
    <SettingsSection
      title={t("setup.settings.applications.title")}
      icon={<KeyRound size={20} />}
    >
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
        <select
          value={applicationPreset}
          onChange={(event) => onApplicationPresetChange(event.target.value as "downloader" | "full")}
          aria-label={t("setup.settings.applications.preset")}
        >
          <option value="downloader">{t("setup.settings.applications.downloaderPreset")}</option>
          <option value="full">{t("setup.settings.applications.fullPreset")}</option>
        </select>
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
                  <IconButton label={t("setup.settings.applications.copyToken")} icon={<Copy size={16} aria-hidden="true" />} onClick={() => void navigator.clipboard?.writeText(createdApplication.token)} />
                </dd>
              </div>
            </dl>
          <button className="settings-secondary-button" type="button" onClick={onDismissCreatedApplication}>
            <Check size={16} aria-hidden="true" />
            {t("setup.common.ok")}
          </button>
        </section>
      ) : null}

      {loading ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : applications.length === 0 ? (
        <EmptyState
          icon={<KeyRound size={22} />}
          title={t("setup.settings.applications.emptyTitle")}
          description={t("setup.settings.applications.emptyDescription")}
        />
      ) : (
        <CredentialTable
          columns={[
            { key: "name", label: t("setup.settings.applications.name"), className: "settings-table__col-name" },
            { key: "scopes", label: t("setup.settings.applications.scopes"), className: "settings-table__col-key" },
            { key: "status", label: t("setup.settings.s3.status"), className: "settings-table__col-status" },
            { key: "createdAt", label: t("setup.settings.s3.createdAt"), className: "settings-table__col-created" },
            { key: "actions", ariaLabel: t("setup.settings.s3.actions"), className: "settings-table__col-actions" }
          ]}
          minWidth={820}
        >
              {applications.map((application) => (
                <tr key={application.id}>
                  <td className="settings-table__name">{application.name}</td>
                  <td>{formatScopes(application.scopes, t("setup.common.unavailable"))}</td>
                  <td>
                    <StatusBadge
                      active={!application.revoked}
                      activeLabel={t("setup.settings.s3.active")}
                      revokedLabel={t("setup.settings.s3.revoked")}
                    />
                  </td>
                  <td>{formatDate(application.createdAt, i18n.language)}</td>
                  <td className="settings-table__actions">
                    {!application.revoked ? (
                      <IconButton
                        variant="danger"
                        label={t("setup.settings.applications.revoke")}
                        icon={<Ban size={16} aria-hidden="true" />}
                        disabled={revoking === application.id}
                        onClick={() => onRevokeApplication(application.id, application.name)}
                      />
                    ) : null}
                  </td>
                </tr>
              ))}
        </CredentialTable>
      )}
    </SettingsSection>
  );
}

function formatDate(value: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "short"
  }).format(new Date(value));
}

function formatScopes(scopes: string[] | undefined, fallback: string): string {
  return scopes && scopes.length > 0 ? scopes.join(", ") : fallback;
}
