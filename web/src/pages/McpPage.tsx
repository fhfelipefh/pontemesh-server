import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CreatedMcpAccessToken,
  CreatedMcpOAuthClient,
  McpAccessTokenSummary,
  McpActivityRecord,
  McpOAuthClientSummary,
  McpSettings,
  McpStatus,
  createMcpOAuthClient,
  createMcpToken,
  getMcpSettings,
  getMcpStatus,
  listMcpActivity,
  listMcpOAuthClients,
  listMcpTokens,
  revokeMcpOAuthClient,
  revokeMcpToken,
  updateMcpSettings,
} from "../api/mcpApi";
import { ConfirmDialog } from "../components/AdminListControls";
import { McpSettingsCard } from "../components/settings/McpSettingsCard";

export function McpPage() {
  const { t } = useTranslation();
  const [mcpSettings, setMcpSettings] = useState<McpSettings | null>(null);
  const [mcpStatus, setMcpStatus] = useState<McpStatus | null>(null);
  const [mcpTokens, setMcpTokens] = useState<McpAccessTokenSummary[]>([]);
  const [mcpOAuthClients, setMcpOAuthClients] = useState<McpOAuthClientSummary[]>([]);
  const [mcpActivity, setMcpActivity] = useState<McpActivityRecord[]>([]);
  const [mcpTokenName, setMcpTokenName] = useState("default-mcp-client");
  const [mcpTokenScopes, setMcpTokenScopes] = useState<string[]>(["read"]);
  const [createdMcpToken, setCreatedMcpToken] =
    useState<CreatedMcpAccessToken | null>(null);
  const [createdOAuthClient, setCreatedOAuthClient] =
    useState<CreatedMcpOAuthClient | null>(null);
  const [loadingMcp, setLoadingMcp] = useState(true);
  const [savingMcp, setSavingMcp] = useState(false);
  const [creatingMcpToken, setCreatingMcpToken] = useState(false);
  const [creatingOAuthClient, setCreatingOAuthClient] = useState(false);
  const [revokingMcpToken, setRevokingMcpToken] = useState<string | null>(null);
  const [revokingOAuthClient, setRevokingOAuthClient] = useState<string | null>(null);
  const [mcpError, setMcpError] = useState("");
  const [tokenToRevoke, setTokenToRevoke] = useState<{ id: string; name: string } | null>(null);
  const [oauthClientToRevoke, setOauthClientToRevoke] = useState<{ id: string; name: string } | null>(null);

  const refreshMcp = useCallback(async () => {
    setLoadingMcp(true);
    setMcpError("");
    try {
      const [settings, status, tokens, activity, oauthClients] = await Promise.all([
        getMcpSettings(),
        getMcpStatus(),
        listMcpTokens(),
        listMcpActivity(),
        listMcpOAuthClients(),
      ]);
      setMcpSettings(settings);
      setMcpStatus(status);
      setMcpTokens(tokens);
      setMcpActivity(activity);
      setMcpOAuthClients(oauthClients);
    } catch (loadError) {
      setMcpError(
        loadError instanceof Error
          ? loadError.message
          : t("setup.settings.mcp.loadFailed")
      );
    } finally {
      setLoadingMcp(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshMcp();
  }, [refreshMcp]);

  async function handleUpdateMcpSettings(nextSettings: McpSettings) {
    setSavingMcp(true);
    setMcpError("");
    try {
      const saved = await updateMcpSettings({
        enabled: nextSettings.enabled,
        endpointPath: nextSettings.endpointPath,
        bindHost: nextSettings.bindHost,
        requireAuth: nextSettings.requireAuth,
        authMode: nextSettings.authMode,
        readToolsEnabled: nextSettings.readToolsEnabled,
        writeToolsEnabled: nextSettings.writeToolsEnabled,
        adminToolsEnabled: nextSettings.adminToolsEnabled,
        exposeResources: nextSettings.exposeResources,
        exposePrompts: nextSettings.exposePrompts,
        allowLocalhostOnly: nextSettings.allowLocalhostOnly,
      });
      setMcpSettings(saved);
      setMcpStatus(await getMcpStatus());
    } catch (saveError) {
      setMcpError(
        saveError instanceof Error
          ? saveError.message
          : t("setup.settings.mcp.saveFailed")
      );
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
      setMcpError(
        createError instanceof Error
          ? createError.message
          : t("setup.settings.mcp.createTokenFailed")
      );
    } finally {
      setCreatingMcpToken(false);
    }
  }

  async function handleRevokeMcpToken(id: string) {
    setRevokingMcpToken(id);
    setMcpError("");
    try {
      await revokeMcpToken(id);
      setTokenToRevoke(null);
      setMcpTokens(await listMcpTokens());
    } catch (revokeError) {
      setMcpError(
        revokeError instanceof Error
          ? revokeError.message
          : t("setup.settings.mcp.revokeTokenFailed")
      );
    } finally {
      setRevokingMcpToken(null);
    }
  }

  async function handleCreateOAuthClient(
    name: string,
    redirectUris: string[],
    scopes: string[]
  ) {
    if (!name.trim()) {
      return;
    }
    setCreatingOAuthClient(true);
    setMcpError("");
    try {
      const created = await createMcpOAuthClient(name, redirectUris, scopes);
      setCreatedOAuthClient(created);
      setMcpOAuthClients(await listMcpOAuthClients());
    } catch (createError) {
      setMcpError(
        createError instanceof Error
          ? createError.message
          : t("setup.settings.mcp.createClientFailed")
      );
    } finally {
      setCreatingOAuthClient(false);
    }
  }

  async function handleRevokeOAuthClient(id: string) {
    setRevokingOAuthClient(id);
    setMcpError("");
    try {
      await revokeMcpOAuthClient(id);
      setOauthClientToRevoke(null);
      setMcpOAuthClients(await listMcpOAuthClients());
    } catch (revokeError) {
      setMcpError(
        revokeError instanceof Error
          ? revokeError.message
          : t("setup.settings.mcp.revokeClientFailed")
      );
    } finally {
      setRevokingOAuthClient(null);
    }
  }

  return (
    <div className="settings-page">
      <header className="settings-page__header">
        <div>
          <h1>{t("setup.settings.mcp.title")}</h1>
          <p>{t("setup.settings.mcp.description")}</p>
        </div>
      </header>

      {mcpError ? <p className="error-message">{mcpError}</p> : null}

      <div className="settings-page__grid">
        <McpSettingsCard
          settings={mcpSettings}
          status={mcpStatus}
          tokens={mcpTokens}
          oauthClients={mcpOAuthClients}
          activity={mcpActivity}
          tokenName={mcpTokenName}
          tokenScopes={mcpTokenScopes}
          onTokenScopesChange={setMcpTokenScopes}
          createdToken={createdMcpToken}
          createdOAuthClient={createdOAuthClient}
          loading={loadingMcp}
          saving={savingMcp}
          creatingToken={creatingMcpToken}
          creatingOAuthClient={creatingOAuthClient}
          revokingToken={revokingMcpToken}
          revokingOAuthClient={revokingOAuthClient}
          error={mcpError}
          onTokenNameChange={setMcpTokenName}
          onUpdateSettings={handleUpdateMcpSettings}
          onCreateToken={handleCreateMcpToken}
          onDismissCreatedToken={() => setCreatedMcpToken(null)}
          onRevokeToken={(id, name) => setTokenToRevoke({ id, name })}
          onCreateOAuthClient={handleCreateOAuthClient}
          onDismissCreatedOAuthClient={() => setCreatedOAuthClient(null)}
          onRevokeOAuthClient={(id, name) => setOauthClientToRevoke({ id, name })}
        />
      </div>

      {tokenToRevoke ? (
        <ConfirmDialog
          title={t("setup.settings.mcp.confirmRevokeTokenTitle")}
          description={t("setup.settings.mcp.confirmRevokeTokenDescription", {
            name: tokenToRevoke.name,
          })}
          confirmLabel={t("setup.settings.mcp.revokeToken")}
          onCancel={() => setTokenToRevoke(null)}
          onConfirm={() => void handleRevokeMcpToken(tokenToRevoke.id)}
        />
      ) : null}

      {oauthClientToRevoke ? (
        <ConfirmDialog
          title={t("setup.settings.mcp.confirmRevokeClientTitle")}
          description={t("setup.settings.mcp.confirmRevokeClientDescription", {
            name: oauthClientToRevoke.name,
          })}
          confirmLabel={t("setup.settings.mcp.revokeClient")}
          onCancel={() => setOauthClientToRevoke(null)}
          onConfirm={() => void handleRevokeOAuthClient(oauthClientToRevoke.id)}
        />
      ) : null}
    </div>
  );
}
