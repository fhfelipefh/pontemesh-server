import { ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Activity,
  Ban,
  Check,
  ChevronLeft,
  ChevronRight,
  Network,
  Plus,
  Search,
  ShieldCheck,
  Wrench,
  X,
} from "lucide-react";
import {
  CreatedMcpAccessToken,
  CreatedMcpOAuthClient,
  McpAccessTokenSummary,
  McpActivityRecord,
  McpOAuthClientSummary,
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
import { McpIcon } from "../McpIcon";

export type McpSettingsCardProps = {
  settings: McpSettings | null;
  status: McpStatus | null;
  tokens: McpAccessTokenSummary[];
  activity: McpActivityRecord[];
  oauthClients?: McpOAuthClientSummary[];
  tokenName: string;
  tokenScopes: string[];
  onTokenScopesChange: (scopes: string[]) => void;
  createdToken: CreatedMcpAccessToken | null;
  createdOAuthClient?: CreatedMcpOAuthClient | null;
  loading: boolean;
  saving: boolean;
  creatingToken: boolean;
  creatingOAuthClient?: boolean;
  revokingToken: string | null;
  revokingOAuthClient?: string | null;
  error: string;
  onTokenNameChange: (value: string) => void;
  onUpdateSettings: (settings: McpSettings) => void;
  onCreateToken: () => void;
  onDismissCreatedToken: () => void;
  onRevokeToken: (id: string, name: string) => void;
  onCreateOAuthClient?: (name: string, redirectUris: string[], scopes: string[]) => void;
  onDismissCreatedOAuthClient?: () => void;
  onRevokeOAuthClient?: (id: string, name: string) => void;
};

export function McpSettingsCard({
  settings,
  status,
  tokens,
  activity,
  oauthClients,
  tokenName,
  tokenScopes,
  createdToken,
  createdOAuthClient,
  loading,
  saving,
  creatingToken,
  creatingOAuthClient,
  revokingToken,
  revokingOAuthClient,
  error,
  onTokenNameChange,
  onTokenScopesChange,
  onUpdateSettings,
  onCreateToken,
  onDismissCreatedToken,
  onRevokeToken,
  onCreateOAuthClient,
  onDismissCreatedOAuthClient,
  onRevokeOAuthClient,
}: McpSettingsCardProps) {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<"settings" | "tokens" | "oauth" | "activity">("settings");
  const [oauthClientName, setOauthClientName] = useState("");
  const [oauthRedirectUris, setOauthRedirectUris] = useState("");
  const [oauthScopes, setOauthScopes] = useState<string[]>(["read"]);
  const [oauthSearchQuery, setOauthSearchQuery] = useState("");
  const [oauthSortOrder, setOauthSortOrder] = useState<"newest" | "oldest" | "name_asc" | "name_desc">("newest");
  const [oauthPage, setOauthPage] = useState(1);
  const oauthPageSize = 10;

  const normalizedOauthSearch = oauthSearchQuery.trim().toLowerCase();
  const filteredOAuthClients = (oauthClients || []).filter(client => {
    if (!normalizedOauthSearch) {
      return true;
    }
    const nameMatch = client.clientName.toLowerCase().includes(normalizedOauthSearch);
    const idMatch = client.clientId.toLowerCase().includes(normalizedOauthSearch);
    const uriMatch = (client.redirectUris || []).some(uri =>
      uri.toLowerCase().includes(normalizedOauthSearch)
    );
    return nameMatch || idMatch || uriMatch;
  });

  const sortedOAuthClients = [...filteredOAuthClients].sort((a, b) => {
    if (oauthSortOrder === "newest") {
      return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
    }
    if (oauthSortOrder === "oldest") {
      return new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
    }
    if (oauthSortOrder === "name_asc") {
      return a.clientName.localeCompare(b.clientName);
    }
    if (oauthSortOrder === "name_desc") {
      return b.clientName.localeCompare(a.clientName);
    }
    return 0;
  });

  const totalOauthClients = sortedOAuthClients.length;
  const totalOauthPages = Math.max(1, Math.ceil(totalOauthClients / oauthPageSize));
  const effectiveOauthPage = Math.min(oauthPage, totalOauthPages);
  const firstVisibleClient = totalOauthClients === 0 ? 0 : (effectiveOauthPage - 1) * oauthPageSize + 1;
  const lastVisibleClient = Math.min(totalOauthClients, effectiveOauthPage * oauthPageSize);
  const pagedOAuthClients = sortedOAuthClients.slice(
    (effectiveOauthPage - 1) * oauthPageSize,
    effectiveOauthPage * oauthPageSize
  );

  const mcpServerUrl = status && settings ? absoluteMcpUrl(status.endpoint || settings.endpointPath) : "";
  const oauthConnectionConfig =
    createdOAuthClient && mcpServerUrl
      ? JSON.stringify(
          {
            serverUrl: mcpServerUrl,
            clientId: createdOAuthClient.client.clientId,
            clientSecret: createdOAuthClient.clientSecret,
            authType: "oauth2",
            scopes: createdOAuthClient.client.scopes,
          },
          null,
          2
        )
      : "";

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
      icon={<McpIcon size={20} />}
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
              {t("setup.settings.mcp.tabSettings")}
            </button>
            <button
              className={`mcp-tab ${activeTab === "tokens" ? "mcp-tab--active" : ""}`}
              onClick={() => setActiveTab("tokens")}
              aria-label={`${t("setup.settings.mcp.tabTokens")} Tokens de Acesso`}
              data-testid="mcp-tab-tokens"
            >
              {t("setup.settings.mcp.tabTokens")}
            </button>
            <button
              className={`mcp-tab ${activeTab === "oauth" ? "mcp-tab--active" : ""}`}
              onClick={() => setActiveTab("oauth")}
            >
              {t("setup.settings.mcp.tabConnectedApps")}
            </button>
            <button
              className={`mcp-tab ${activeTab === "activity" ? "mcp-tab--active" : ""}`}
              onClick={() => setActiveTab("activity")}
            >
              {t("setup.settings.mcp.tabActivity")}
            </button>
          </div>

          <div className="mcp-tab-content">
            {activeTab === "settings" && (
              <>
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
                <div className="settings-toggle-row">
                  <span>{t("setup.settings.mcp.authMode")}</span>
                  <select
                    value={settings.authMode || "hybrid"}
                    disabled={saving}
                    onChange={event =>
                      update({
                        authMode: event.target.value as "hybrid" | "token" | "oauth2"
                      })
                    }
                    aria-label={t("setup.settings.mcp.authMode")}
                  >
                    <option value="hybrid">{t("setup.settings.mcp.authModeHybrid")}</option>
                    <option value="oauth2">{t("setup.settings.mcp.authModeOAuth2")}</option>
                    <option value="token">{t("setup.settings.mcp.authModeToken")}</option>
                  </select>
                </div>
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
              {settings.authMode !== "token" && (
                <div className="mcp-oauth-endpoints-box">
                  <h4>{t("setup.settings.mcp.oauthEndpoints")}</h4>
                  <p>{t("setup.settings.mcp.geminiSparkHint")}</p>
                  <div className="mcp-oauth-endpoints-list">
                    <div>
                      <strong>{t("setup.settings.mcp.resourceMetadata")}:</strong>
                      <code>/.well-known/oauth-protected-resource/mcp</code>
                    </div>
                    <div>
                      <strong>{t("setup.settings.mcp.authServerMetadata")}:</strong>
                      <code>/.well-known/oauth-authorization-server</code>
                    </div>
                    <div>
                      <strong>{t("setup.settings.mcp.tokenEndpoint")}:</strong>
                      <code>/oauth/token</code>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}

            {activeTab === "tokens" && (
              <div className="mcp-tokens-tab">
                <p className="settings-field-hint" style={{ marginBottom: "1rem" }}>
                  {t("setup.settings.mcp.tokenAuthNotice")}
                </p>
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

            {activeTab === "oauth" && (
              <div className="mcp-tokens-tab">
                <p className="settings-field-hint" style={{ marginBottom: "1rem" }}>
                  {t("setup.settings.mcp.connectedAppsDescription")}
                </p>

                <form
                  className="inline-form mcp-token-form"
                  onSubmit={event => {
                    event.preventDefault();
                    if (!oauthClientName.trim()) {
                      return;
                    }
                    const uris = oauthRedirectUris
                      .split(",")
                      .map(u => u.trim())
                      .filter(Boolean);
                    onCreateOAuthClient?.(oauthClientName, uris, oauthScopes);
                    setOauthClientName("");
                    setOauthRedirectUris("");
                    setOauthScopes(["read"]);
                  }}
                >
                  <input
                    value={oauthClientName}
                    onChange={event => setOauthClientName(event.target.value)}
                    placeholder={t("setup.settings.mcp.clientNamePlaceholder")}
                    aria-label={t("setup.settings.mcp.clientName")}
                  />
                  <input
                    value={oauthRedirectUris}
                    onChange={event => setOauthRedirectUris(event.target.value)}
                    placeholder={t("setup.settings.mcp.redirectUrisPlaceholder")}
                    aria-label={t("setup.settings.mcp.redirectUris")}
                  />
                  <button
                    className="settings-create-key-button"
                    type="submit"
                    disabled={creatingOAuthClient || !oauthClientName.trim()}
                  >
                    <Plus size={17} aria-hidden="true" />
                    {t("setup.settings.mcp.createClient")}
                  </button>
                  <div className="mcp-token-scopes">
                    <span className="mcp-token-scopes__label">
                      {t("setup.settings.mcp.tokenScopes")}
                    </span>
                    <div
                      className="settings-checkbox-group"
                      role="group"
                      aria-label={t("setup.settings.mcp.tokenScopes")}
                    >
                      {["read", "write", "admin"].map(scope => (
                        <label key={scope} className="settings-checkbox-field">
                          <input
                            type="checkbox"
                            checked={oauthScopes.includes(scope)}
                            disabled={scope === "read"}
                            onChange={event => {
                              const next = event.target.checked
                                ? [...oauthScopes, scope]
                                : oauthScopes.filter(item => item !== scope);
                              setOauthScopes(Array.from(new Set(["read", ...next])));
                            }}
                          />
                          {scope}
                        </label>
                      ))}
                    </div>
                  </div>
                  {oauthScopes.some(s => s === "write" || s === "admin") ? (
                    <p className="settings-warning">
                      {t("setup.settings.mcp.permissionWarning")}
                    </p>
                  ) : null}
                </form>

                {createdOAuthClient ? (
                  <section className="secret-panel" role="status">
                    <strong>{t("setup.settings.mcp.clientCreated")}</strong>
                    <p>{t("setup.settings.mcp.clientCreatedHint")}</p>
                    <dl>
                      <div>
                        <dt>{t("setup.settings.mcp.serverUrl")}</dt>
                        <dd>
                          <code>{mcpServerUrl}</code>
                          <CopyButton
                            value={mcpServerUrl}
                            label={t("setup.settings.mcp.copyServerUrl")}
                          />
                        </dd>
                      </div>
                      <div>
                        <dt>{t("setup.settings.mcp.clientId")}</dt>
                        <dd>
                          <code>{createdOAuthClient.client.clientId}</code>
                          <CopyButton
                            value={createdOAuthClient.client.clientId}
                            label={t("setup.settings.mcp.copyClientId")}
                          />
                        </dd>
                      </div>
                      <div>
                        <dt>{t("setup.settings.mcp.clientSecret")}</dt>
                        <dd>
                          <code>{createdOAuthClient.clientSecret}</code>
                          <CopyButton
                            value={createdOAuthClient.clientSecret}
                            label={t("setup.settings.mcp.copyClientSecret")}
                          />
                        </dd>
                      </div>
                    </dl>
                    <div className="mcp-connection-config">
                      <div className="mcp-connection-config__header">
                        <strong>{t("setup.settings.mcp.connectionConfig")}</strong>
                        <CopyButton
                          value={oauthConnectionConfig}
                          label={t("setup.settings.mcp.copyOAuthJson")}
                        />
                      </div>
                      <pre>{oauthConnectionConfig}</pre>
                    </div>
                    <button
                      className="settings-secondary-button"
                      type="button"
                      onClick={onDismissCreatedOAuthClient}
                    >
                      <Check size={16} aria-hidden="true" />
                      {t("setup.common.ok")}
                    </button>
                  </section>
                ) : null}

                <div className="mcp-client-toolbar">
                  <div className="mcp-client-search">
                    <Search size={16} aria-hidden="true" className="mcp-client-search__icon" />
                    <input
                      type="search"
                      value={oauthSearchQuery}
                      onChange={event => {
                        setOauthSearchQuery(event.target.value);
                        setOauthPage(1);
                      }}
                      placeholder={t("setup.settings.mcp.searchClientsPlaceholder")}
                      aria-label={t("setup.settings.mcp.searchClientsPlaceholder")}
                    />
                    {oauthSearchQuery ? (
                      <button
                        type="button"
                        className="mcp-client-search__clear"
                        onClick={() => {
                          setOauthSearchQuery("");
                          setOauthPage(1);
                        }}
                        aria-label={t("setup.settings.mcp.clearSearch")}
                      >
                        <X size={14} aria-hidden="true" />
                      </button>
                    ) : null}
                  </div>
                  <div className="mcp-client-sort">
                    <select
                      value={oauthSortOrder}
                      onChange={event => {
                        setOauthSortOrder(
                          event.target.value as "newest" | "oldest" | "name_asc" | "name_desc"
                        );
                        setOauthPage(1);
                      }}
                      aria-label={t("setup.settings.mcp.sortClients")}
                    >
                      <option value="newest">{t("setup.settings.mcp.sortNewest")}</option>
                      <option value="oldest">{t("setup.settings.mcp.sortOldest")}</option>
                      <option value="name_asc">{t("setup.settings.mcp.sortNameAsc")}</option>
                      <option value="name_desc">{t("setup.settings.mcp.sortNameDesc")}</option>
                    </select>
                  </div>
                </div>

                <CredentialTable
                  minWidth={1180}
                  columns={[
                    {
                      key: "name",
                      label: t("setup.settings.mcp.clientName"),
                      className: "settings-table__col-oauth-name",
                    },
                    {
                      key: "clientId",
                      label: t("setup.settings.mcp.clientId"),
                      className: "settings-table__col-oauth-client-id",
                    },
                    {
                      key: "redirectUris",
                      label: t("setup.settings.mcp.redirectUrisCol"),
                      className: "settings-table__col-oauth-uris",
                    },
                    {
                      key: "scopes",
                      label: t("setup.settings.mcp.tokenScopes"),
                      className: "settings-table__col-oauth-scopes",
                    },
                    {
                      key: "status",
                      label: t("setup.settings.s3.status"),
                      className: "settings-table__col-oauth-status",
                    },
                    {
                      key: "createdAt",
                      label: t("setup.settings.s3.createdAt"),
                      className: "settings-table__col-oauth-created",
                    },
                    {
                      key: "actions",
                      ariaLabel: t("setup.settings.s3.actions"),
                      className: "settings-table__col-oauth-actions",
                    },
                  ]}
                >
                  {(!oauthClients || oauthClients.length === 0) ? (
                    <tr className="settings-table__empty-row">
                      <td colSpan={7}>
                        <EmptyState title={t("setup.settings.mcp.noClients")} />
                      </td>
                    </tr>
                  ) : sortedOAuthClients.length === 0 ? (
                    <tr className="settings-table__empty-row">
                      <td colSpan={7}>
                        <EmptyState title={t("setup.settings.mcp.noMatchingClients")} />
                      </td>
                    </tr>
                  ) : (
                    pagedOAuthClients.map(client => {
                      const isActive = client.isActive ?? client.active ?? true;
                      return (
                        <tr key={client.id}>
                          <td className="settings-table__name">{client.clientName}</td>
                          <td>
                            <div className="settings-access-key">
                              <code>{client.clientId}</code>
                              <CopyButton
                                value={client.clientId}
                                label={t("setup.settings.mcp.copyClientId")}
                              />
                            </div>
                          </td>
                          <td>
                            {client.redirectUris && client.redirectUris.length > 0 ? (
                              <div className="mcp-client-uris">
                                {client.redirectUris.map(uri => (
                                  <span key={uri} className="mcp-client-uri" title={uri}>
                                    {uri}
                                  </span>
                                ))}
                              </div>
                            ) : (
                              <span className="settings-table__muted">
                                {t("setup.common.unavailable")}
                              </span>
                            )}
                          </td>
                          <td>
                            {client.scopes && client.scopes.length > 0 ? (
                              <div className="mcp-scope-badges">
                                {client.scopes.map(scope => (
                                  <span key={scope} className="mcp-scope-badge">
                                    {scope}
                                  </span>
                                ))}
                              </div>
                            ) : (
                              <span className="settings-table__muted">
                                {t("setup.common.unavailable")}
                              </span>
                            )}
                          </td>
                          <td>
                            <StatusBadge
                              active={isActive}
                              activeLabel={t("setup.settings.s3.active")}
                              revokedLabel={t("setup.settings.s3.revoked")}
                            />
                          </td>
                          <td>{formatDate(client.createdAt, i18n.language)}</td>
                          <td className="settings-table__actions">
                            {isActive ? (
                              <IconButton
                                variant="danger"
                                label={t("setup.settings.mcp.revokeClient")}
                                icon={<Ban size={16} aria-hidden="true" />}
                                disabled={revokingOAuthClient === client.id}
                                onClick={() => onRevokeOAuthClient?.(client.id, client.clientName)}
                              />
                            ) : null}
                          </td>
                        </tr>
                      );
                    })
                  )}
                </CredentialTable>

                {totalOauthClients > oauthPageSize ? (
                  <nav
                    className="settings-pagination"
                    aria-label={t("setup.settings.mcp.oauthPaginationLabel")}
                  >
                    <p>
                      {t("setup.settings.mcp.oauthPaginationSummary", {
                        start: firstVisibleClient,
                        end: lastVisibleClient,
                        total: totalOauthClients,
                      })}
                    </p>
                    <div className="settings-pagination__actions">
                      <button
                        type="button"
                        className="settings-pagination__button"
                        aria-label={t("setup.settings.s3.previousPage")}
                        disabled={effectiveOauthPage === 1}
                        onClick={() => setOauthPage(Math.max(1, effectiveOauthPage - 1))}
                      >
                        <ChevronLeft size={16} aria-hidden="true" />
                      </button>
                      <span>
                        {t("setup.settings.s3.pageIndicator", {
                          page: effectiveOauthPage,
                          total: totalOauthPages,
                        })}
                      </span>
                      <button
                        type="button"
                        className="settings-pagination__button"
                        aria-label={t("setup.settings.s3.nextPage")}
                        disabled={effectiveOauthPage === totalOauthPages}
                        onClick={() => setOauthPage(Math.min(totalOauthPages, effectiveOauthPage + 1))}
                      >
                        <ChevronRight size={16} aria-hidden="true" />
                      </button>
                    </div>
                  </nav>
                ) : null}
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
