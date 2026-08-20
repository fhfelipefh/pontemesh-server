import { ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { Activity, Ban, Check, Network, Plus, ShieldCheck, Wrench } from "lucide-react";
import {
  CreatedMcpAccessToken,
  McpAccessTokenSummary,
  McpActivityRecord,
  McpSettings,
  McpStatus,
} from "../../api/mcpApi";
import { SettingsSection } from "./SettingsSection";
import { ToggleRow } from "./ToggleRow";
import { CopyButton } from "./CopyButton";
import { CredentialTable } from "./CredentialTable";
import { EmptyState } from "./EmptyState";
import { StatusBadge } from "./StatusBadge";
import { IconButton } from "./IconButton";

export type McpSettingsCardProps = {
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

export function McpSettingsCard({
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
  onRevokeToken,
}: McpSettingsCardProps) {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<"settings" | "tokens" | "activity">("settings");

  const mcpConnectionConfig =
    createdToken && settings && status
      ? JSON.stringify(
          buildMcpConnectionConfig(createdToken, settings, status),
          null,
          2
        )
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
      icon={<Network size={20} />}
    >
      {error ? <p className="error-message">{error}</p> : null}

      {loading || !settings || !status ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : !settings.enabled ? (
        <div className="mcp-settings-grid mcp-settings-grid--single">
          <ToggleRow
            label={t("setup.settings.mcp.enable")}
            checked={settings.enabled}
            disabled={saving}
            onChange={checked => update({ enabled: checked })}
          />
        </div>
      ) : (
        <>
          <div className="mcp-summary-grid">
            <McpSummaryItem
              icon={<Activity size={17} />}
              label={t("setup.settings.mcp.status")}
              value={
                status.enabled
                  ? t("setup.settings.mcp.enabled")
                  : t("setup.settings.mcp.disabled")
              }
            />
            <McpSummaryItem
              icon={<Network size={17} />}
              label={t("setup.settings.mcp.endpoint")}
              value={status.endpoint}
            />
            <McpSummaryItem
              icon={<ShieldCheck size={17} />}
              label={t("setup.settings.mcp.accessMode")}
              value={
                status.adminToolsEnabled
                  ? t("setup.settings.mcp.fullAdmin")
                  : status.writeToolsEnabled
                    ? t("setup.settings.mcp.readWrite")
                    : t("setup.settings.mcp.readOnly")
              }
            />
            <McpSummaryItem
              icon={<Wrench size={17} />}
              label={t("setup.settings.mcp.lastActivity")}
              value={
                status.lastActivityAt
                  ? formatDate(status.lastActivityAt, i18n.language)
                  : t("setup.common.unavailable")
              }
            />
          </div>

          <div className="mcp-tabs">
            <button
              className={`mcp-tab ${activeTab === "settings" ? "mcp-tab--active" : ""}`}
              onClick={() => setActiveTab("settings")}
            >
              Configurações
            </button>
            <button
              className={`mcp-tab ${activeTab === "tokens" ? "mcp-tab--active" : ""}`}
              onClick={() => setActiveTab("tokens")}
            >
              Tokens de Acesso
            </button>
            <button
              className={`mcp-tab ${activeTab === "activity" ? "mcp-tab--active" : ""}`}
              onClick={() => setActiveTab("activity")}
            >
              Logs de Atividade
            </button>
          </div>

          <div className="mcp-tab-content">
            {activeTab === "settings" && (
              <div className="mcp-settings-grid">
                <ToggleRow
                  label={t("setup.settings.mcp.enable")}
                  checked={settings.enabled}
                  disabled={saving}
                  onChange={checked => update({ enabled: checked })}
                />
                <ToggleRow
                  label={t("setup.settings.mcp.requireAuth")}
                  checked={settings.requireAuth}
                  disabled
                />
                <ToggleRow
                  label={t("setup.settings.mcp.localhostOnly")}
                  checked={settings.allowLocalhostOnly}
                  disabled={saving}
                  onChange={checked => update({ allowLocalhostOnly: checked })}
                />
                <ToggleRow
                  label={t("setup.settings.mcp.readTools")}
                  checked={settings.readToolsEnabled}
                  disabled={saving}
                  onChange={checked => update({ readToolsEnabled: checked })}
                />
                <ToggleRow
                  label={t("setup.settings.mcp.writeTools")}
                  checked={settings.writeToolsEnabled}
                  disabled={saving}
                  onChange={checked => update({ writeToolsEnabled: checked })}
                />
                <ToggleRow
                  label={t("setup.settings.mcp.adminTools")}
                  checked={settings.adminToolsEnabled}
                  disabled={saving}
                  onChange={checked => update({ adminToolsEnabled: checked })}
                />
                <ToggleRow
                  label={t("setup.settings.mcp.resources")}
                  checked={settings.exposeResources}
                  disabled={saving}
                  onChange={checked => update({ exposeResources: checked })}
                />
                <ToggleRow
                  label={t("setup.settings.mcp.prompts")}
                  checked={settings.exposePrompts}
                  disabled={saving}
                  onChange={checked => update({ exposePrompts: checked })}
                />
              </div>
            )}

            {activeTab === "tokens" && (
              <div className="mcp-tokens-tab">
                <form
                  className="inline-form mcp-token-form"
                  onSubmit={event => {
                    event.preventDefault();
                    onCreateToken();
                  }}
                >
                  <input
                    value={tokenName}
                    onChange={event => onTokenNameChange(event.target.value)}
                    placeholder={t("setup.settings.mcp.tokenNamePlaceholder")}
                    aria-label={t("setup.settings.mcp.tokenName")}
                  />
                  <button
                    className="settings-create-key-button"
                    type="submit"
                    disabled={creatingToken || !tokenName.trim()}
                  >
                    <Plus size={17} aria-hidden="true" />
                    {t("setup.settings.mcp.createToken")}
                  </button>
                  <div className="mcp-token-scopes">
                    <span className="mcp-token-scopes__label">
                      {t("setup.settings.mcp.tokenScopes")}
                    </span>
                    <div
                      className="settings-checkbox-group"
                      role="group"
                      aria-label={t("setup.settings.mcp.tokenScopes")}
                      data-testid="mcp-token-scope-group"
                    >
                      {["read", "write", "admin"].map(scope => (
                        <label key={scope} className="settings-checkbox-field">
                          <input
                            type="checkbox"
                            checked={tokenScopes.includes(scope)}
                            disabled={scope === "read"}
                            data-testid={`mcp-token-scope-${scope}`}
                            onChange={event => {
                              const next = event.target.checked
                                ? [...tokenScopes, scope]
                                : tokenScopes.filter(item => item !== scope);
                              onTokenScopesChange(
                                Array.from(new Set(["read", ...next]))
                              );
                            }}
                          />
                          {scope}
                        </label>
                      ))}
                    </div>
                  </div>
                  {tokenScopes.some(
                    scope => scope === "write" || scope === "admin"
                  ) ? (
                    <p className="settings-warning">
                      {t("setup.settings.mcp.permissionWarning")}
                    </p>
                  ) : null}
                </form>

                {createdToken ? (
                  <section className="secret-panel" role="status">
                    <strong>{t("setup.settings.mcp.tokenCreated")}</strong>
                    <p>{t("setup.settings.mcp.tokenCreatedHint")}</p>
                    <dl>
                      <div>
                        <dt>{t("setup.settings.mcp.tokenPrefix")}</dt>
                        <dd>
                          <code>{createdToken.token.tokenPrefix}</code>
                        </dd>
                      </div>
                      <div>
                        <dt>{t("setup.settings.mcp.tokenSecret")}</dt>
                        <dd>
                          <code>{createdToken.secret}</code>
                          <CopyButton
                            value={createdToken.secret}
                            label={t("setup.settings.mcp.copyToken")}
                          />
                        </dd>
                      </div>
                    </dl>
                    <div className="mcp-connection-config">
                      <div className="mcp-connection-config__header">
                        <strong>{t("setup.settings.mcp.connectionConfig")}</strong>
                        <CopyButton
                          value={mcpConnectionConfig}
                          label={t("setup.settings.mcp.copyConnectionConfig")}
                        />
                      </div>
                      <pre>{mcpConnectionConfig}</pre>
                    </div>
                    <button
                      className="settings-secondary-button"
                      type="button"
                      onClick={onDismissCreatedToken}
                    >
                      <Check size={16} aria-hidden="true" />
                      {t("setup.common.ok")}
                    </button>
                  </section>
                ) : null}

                <CredentialTable
                  columns={[
                    {
                      key: "name",
                      label: t("setup.settings.mcp.tokenName"),
                      className: "settings-table__col-name",
                    },
                    {
                      key: "prefix",
                      label: t("setup.settings.mcp.tokenPrefix"),
                      className: "settings-table__col-key",
                    },
                    {
                      key: "scopes",
                      label: t("setup.settings.mcp.tokenScopes"),
                      className: "settings-table__col-status",
                    },
                    {
                      key: "status",
                      label: t("setup.settings.s3.status"),
                      className: "settings-table__col-status",
                    },
                    {
                      key: "lastUsed",
                      label: t("setup.settings.s3.lastUsed"),
                      className: "settings-table__col-last-used",
                    },
                    {
                      key: "createdAt",
                      label: t("setup.settings.s3.createdAt"),
                      className: "settings-table__col-created",
                    },
                    {
                      key: "actions",
                      ariaLabel: t("setup.settings.s3.actions"),
                      className: "settings-table__col-actions",
                    },
                  ]}
                >
                  {tokens.length === 0 ? (
                    <tr className="settings-table__empty-row">
                      <td colSpan={7}>
                        <EmptyState title={t("setup.settings.mcp.noTokens")} />
                      </td>
                    </tr>
                  ) : (
                    tokens.map(token => (
                      <tr key={token.id}>
                        <td className="settings-table__name">{token.name}</td>
                        <td>
                          <code>{token.tokenPrefix}</code>
                        </td>
                        <td>
                          {formatScopes(token.scopes, t("setup.common.unavailable"))}
                        </td>
                        <td>
                          <StatusBadge
                            active={token.active}
                            activeLabel={t("setup.settings.s3.active")}
                            revokedLabel={t("setup.settings.s3.revoked")}
                          />
                        </td>
                        <td>
                          {token.lastUsedAt
                            ? formatDate(token.lastUsedAt, i18n.language)
                            : t("setup.common.unavailable")}
                        </td>
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
                    ))
                  )}
                </CredentialTable>
              </div>
            )}

            {activeTab === "activity" && (
              <section className="mcp-activity">
                {activity.length === 0 ? (
                  <p>{t("setup.settings.mcp.noActivity")}</p>
                ) : (
                  <ol>
                    {activity.slice(0, 15).map(entry => (
                      <li key={entry.id}>
                        <span>{entry.method}</span>
                        <strong>{entry.outcome}</strong>
                        <time dateTime={entry.createdAt}>
                          {formatDate(entry.createdAt, i18n.language)}
                        </time>
                      </li>
                    ))}
                  </ol>
                )}
              </section>
            )}
          </div>
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
      "Content-Type": "application/json",
    },
    environment: {
      PONTEMESH_MCP_URL: url,
      PONTEMESH_MCP_TOKEN: createdToken.secret,
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
          version: "1.0.0",
        },
      },
    },
    options: {
      localhostOnly: settings.allowLocalhostOnly,
      readToolsEnabled: settings.readToolsEnabled,
      writeToolsEnabled: settings.writeToolsEnabled,
      resourcesEnabled: settings.exposeResources,
      promptsEnabled: settings.exposePrompts,
    },
  };
}

function absoluteMcpUrl(endpoint: string) {
  if (/^https?:\/\//i.test(endpoint)) {
    return endpoint;
  }
  const origin =
    typeof window === "undefined"
      ? "http://127.0.0.1:8080"
      : window.location.origin;
  return `${origin}${endpoint.startsWith("/") ? endpoint : `/${endpoint}`}`;
}

function McpSummaryItem({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="mcp-summary-item">
      {icon}
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function formatDate(value: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "short",
  }).format(new Date(value));
}

function formatScopes(scopes: string[] | undefined, fallback: string): string {
  return scopes && scopes.length > 0 ? scopes.join(", ") : fallback;
}
