import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Save,
  Server,
} from "lucide-react";

import {
  getInstanceSummary,
  updateInstanceSettings,
} from "../api/dashboardApi";
import { TimezoneSelect } from "../components/settings/TimezoneSelect";
import {
  getServerUpdateStatus,
  requestServerUpdate,
  ServerUpdateStatus,
} from "../api/serverUpdateApi";
import {
  ApplicationCredentialSummary,
  CreatedApplicationCredential,
  createApplicationCredential,
  listApplicationCredentials,
  revokeApplicationCredential,
} from "../api/applicationCredentialsApi";
import {
  CreatedS3AccessKey,
  S3AccessKeySummary,
  createS3AccessKey,
  listS3AccessKeys,
  revokeS3AccessKey,
} from "../api/s3KeysApi";
import {
  ConfigurationImportResult,
  exportConfiguration,
  importConfiguration,
} from "../api/configurationApi";
import {
  DiskGuardSettings,
  getDiskGuardSettings,
  updateDiskGuardSettings,
} from "../api/storageApi";
import {
  OperationalWebhookSettings,
  getOperationalWebhook,
  updateOperationalWebhook,
} from "../api/webhookApi";
import { OidcSettings, getOidcSettings } from "../api/oidcApi";
import { OidcSettingsCard } from "../components/settings/OidcSettingsCard";
import { Button } from "../components/Button";
import { ConfirmDialog } from "../components/AdminListControls";

import { OperationalWebhookCard } from "../components/settings/OperationalWebhookCard";
import { S3CredentialsCard } from "../components/settings/S3CredentialsCard";
import { ServerUpdateCard } from "../components/settings/ServerUpdateCard";
import { SettingsSection } from "../components/settings/SettingsSection";
import { SpeedTestCard } from "../components/settings/SpeedTestCard";
import { StorageCapacityCard } from "../components/settings/StorageCapacityCard";
import { ConfigurationBackupCard } from "../components/settings/ConfigurationBackupCard";
import { ApplicationCredentialsCard } from "../components/settings/ApplicationCredentialsCard";
const S3_KEYS_PAGE_SIZE = 10;

type DestructiveConfirmation =
  | { kind: "s3Key"; id: string; name: string }
  | { kind: "application"; id: string; name: string }
  | null;

export function SettingsPage({ role }: { role?: string }) {
  const isAdmin = role === "admin";
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
  const [applications, setApplications] = useState<
    ApplicationCredentialSummary[]
  >([]);
  const [createdApplication, setCreatedApplication] =
    useState<CreatedApplicationCredential | null>(null);
  const [applicationName, setApplicationName] = useState("default-sdk");
  const [applicationPreset, setApplicationPreset] = useState<
    "downloader" | "full"
  >("downloader");
  const [loadingApplications, setLoadingApplications] = useState(true);
  const [creatingApplication, setCreatingApplication] = useState(false);
  const [revokingApplication, setRevokingApplication] = useState<string | null>(
    null
  );
  const [applicationError, setApplicationError] = useState("");

  const [configurationImporting, setConfigurationImporting] = useState(false);
  const [configurationResult, setConfigurationResult] =
    useState<ConfigurationImportResult | null>(null);
  const [configurationError, setConfigurationError] = useState("");
  const [destructiveConfirmation, setDestructiveConfirmation] =
    useState<DestructiveConfirmation>(null);
  const [instanceName, setInstanceName] = useState("");
  const [instanceTimezone, setInstanceTimezone] = useState("UTC");
  const [loadingInstance, setLoadingInstance] = useState(true);
  const [savingInstance, setSavingInstance] = useState(false);
  const [instanceError, setInstanceError] = useState("");
  const [diskGuard, setDiskGuard] = useState<DiskGuardSettings | null>(null);
  const [loadingDiskGuard, setLoadingDiskGuard] = useState(true);
  const [savingDiskGuard, setSavingDiskGuard] = useState(false);
  const [diskGuardError, setDiskGuardError] = useState("");
  const [diskGuardSaved, setDiskGuardSaved] = useState(false);
  const [operationalWebhook, setOperationalWebhook] =
    useState<OperationalWebhookSettings | null>(null);
  const [loadingOperationalWebhook, setLoadingOperationalWebhook] =
    useState(true);
  const [savingOperationalWebhook, setSavingOperationalWebhook] =
    useState(false);
  const [operationalWebhookSaved, setOperationalWebhookSaved] = useState(false);
  const [operationalWebhookError, setOperationalWebhookError] = useState("");
  const [oidc, setOidc] = useState<OidcSettings | null>(null);
  const [serverUpdate, setServerUpdate] = useState<ServerUpdateStatus | null>(
    null
  );
  const [loadingServerUpdate, setLoadingServerUpdate] = useState(true);
  const [requestingServerUpdate, setRequestingServerUpdate] = useState(false);
  const [serverUpdateError, setServerUpdateError] = useState("");
  const [serverUpdateConfirmation, setServerUpdateConfirmation] =
    useState(false);
  const [restartPending, setRestartPending] = useState(false);

  useEffect(() => {
    getInstanceSummary()
      .then(summary => {
        setInstanceName(summary.name);
        if (summary.timezone) {
          setInstanceTimezone(summary.timezone);
        }
      })
      .catch(loadError =>
        setInstanceError(
          loadError instanceof Error
            ? loadError.message
            : t("setup.settings.instance.loadFailed")
        )
      )
      .finally(() => setLoadingInstance(false));
  }, [t]);

  useEffect(() => {
    getServerUpdateStatus()
      .then(setServerUpdate)
      .catch(loadError =>
        setServerUpdateError(
          loadError instanceof Error
            ? loadError.message
            : t("setup.settings.update.loadFailed")
        )
      )
      .finally(() => setLoadingServerUpdate(false));
  }, [t]);

  useEffect(() => {
    getDiskGuardSettings()
      .then(setDiskGuard)
      .catch(loadError =>
        setDiskGuardError(
          loadError instanceof Error
            ? loadError.message
            : t("setup.settings.storage.loadFailed")
        )
      )
      .finally(() => setLoadingDiskGuard(false));
  }, [t]);

  useEffect(() => {
    getOperationalWebhook()
      .then(setOperationalWebhook)
      .catch(loadError =>
        setOperationalWebhookError(
          loadError instanceof Error
            ? loadError.message
            : t("setup.settings.webhook.loadFailed")
        )
      )
      .finally(() => setLoadingOperationalWebhook(false));
  }, [t]);

  useEffect(() => {
    getOidcSettings()
      .then(setOidc)
      .catch(() => setOidc(null));
  }, []);

  async function handleRequestServerUpdate() {
    setRequestingServerUpdate(true);
    setServerUpdateError("");
    try {
      await requestServerUpdate();
      setRestartPending(true);
      setServerUpdateConfirmation(false);
    } catch (requestError) {
      setServerUpdateError(
        requestError instanceof Error
          ? requestError.message
          : t("setup.settings.update.requestFailed")
      );
    } finally {
      setRequestingServerUpdate(false);
    }
  }

  async function handleSaveInstanceSettings() {
    if (!instanceName.trim()) {
      return;
    }
    setSavingInstance(true);
    setInstanceError("");
    try {
      const summary = await updateInstanceSettings(
        instanceName.trim(),
        instanceTimezone
      );
      setInstanceName(summary.name);
      setInstanceTimezone(summary.timezone);
      window.dispatchEvent(
        new CustomEvent("pontemesh:instance-updated", { detail: summary })
      );
    } catch (saveError) {
      setInstanceError(
        saveError instanceof Error
          ? saveError.message
          : t("setup.settings.instance.saveFailed")
      );
    } finally {
      setSavingInstance(false);
    }
  }

  async function handleSaveDiskGuard(nextSettings = diskGuard) {
    if (!nextSettings) {
      return;
    }
    const previousSettings = diskGuard;
    setDiskGuard(nextSettings);
    setSavingDiskGuard(true);
    setDiskGuardError("");
    setDiskGuardSaved(false);
    try {
      const saved = await updateDiskGuardSettings({
        enabled: nextSettings.enabled,
        warningPercent: nextSettings.warningPercent,
        degradedPercent: nextSettings.degradedPercent,
        blockPercent: nextSettings.blockPercent,
      });
      setDiskGuard(saved);
      setDiskGuardSaved(true);
    } catch (saveError) {
      setDiskGuard(previousSettings);
      setDiskGuardError(
        saveError instanceof Error
          ? saveError.message
          : t("setup.settings.storage.saveFailed")
      );
    } finally {
      setSavingDiskGuard(false);
    }
  }

  async function handleSaveOperationalWebhook(
    nextSettings = operationalWebhook
  ) {
    if (!nextSettings) {
      return;
    }
    const previousSettings = operationalWebhook;
    setOperationalWebhook(nextSettings);
    setSavingOperationalWebhook(true);
    setOperationalWebhookError("");
    setOperationalWebhookSaved(false);
    try {
      const saved = await updateOperationalWebhook({
        enabled: nextSettings.enabled,
        url: nextSettings.url,
        cron: nextSettings.cron,
      });
      setOperationalWebhook(saved);
      setOperationalWebhookSaved(true);
    } catch (saveError) {
      setOperationalWebhook(previousSettings);
      setOperationalWebhookError(
        saveError instanceof Error
          ? saveError.message
          : t("setup.settings.webhook.saveFailed")
      );
    } finally {
      setSavingOperationalWebhook(false);
    }
  }

  const refreshKeys = useCallback(
    async (page: number) => {
      setLoading(true);
      setError("");
      try {
        const result = await listS3AccessKeys(page, S3_KEYS_PAGE_SIZE);
        setKeys(result.items);
        setCurrentPage(result.page);
        setTotalKeys(result.total);
        setTotalPages(result.totalPages);
      } catch (loadError) {
        setError(
          loadError instanceof Error
            ? loadError.message
            : t("setup.settings.s3.loadFailed")
        );
      } finally {
        setLoading(false);
      }
    },
    [t]
  );

  useEffect(() => {
    void refreshKeys(1);
  }, [refreshKeys]);

  const refreshApplications = useCallback(async () => {
    setLoadingApplications(true);
    setApplicationError("");
    try {
      setApplications(await listApplicationCredentials());
    } catch (loadError) {
      setApplicationError(
        loadError instanceof Error
          ? loadError.message
          : t("setup.settings.applications.loadFailed")
      );
    } finally {
      setLoadingApplications(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshApplications();
  }, [refreshApplications]);



  async function handleCreateKey() {
    setCreating(true);
    setError("");
    try {
      const nextKey = await createS3AccessKey(keyName);
      setCreatedKey(nextKey);
      await refreshKeys(1);
    } catch (createError) {
      setError(
        createError instanceof Error
          ? createError.message
          : t("setup.settings.s3.createFailed")
      );
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
      setError(
        revokeError instanceof Error
          ? revokeError.message
          : t("setup.settings.s3.revokeFailed")
      );
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
      const created = await createApplicationCredential(
        applicationName.trim(),
        undefined,
        applicationPreset
      );
      setCreatedApplication(created);
      setApplicationName("");
      await refreshApplications();
    } catch (createError) {
      setApplicationError(
        createError instanceof Error
          ? createError.message
          : t("setup.settings.applications.createFailed")
      );
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
      setApplicationError(
        revokeError instanceof Error
          ? revokeError.message
          : t("setup.settings.applications.revokeFailed")
      );
    } finally {
      setRevokingApplication(null);
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
      setConfigurationError(
        exportError instanceof Error
          ? exportError.message
          : t("setup.settings.configuration.exportFailed")
      );
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
      await refreshKeys(currentPage);
    } catch (importError) {
      setConfigurationError(
        importError instanceof Error
          ? importError.message
          : t("setup.settings.configuration.importFailed")
      );
    } finally {
      setConfigurationImporting(false);
    }
  }

  return (
    <div className="settings-page">
      <header className="settings-page__header">
        <div>
          <h1>{t("setup.settings.title")}</h1>
          <p>{t("setup.settings.description")}</p>
        </div>
      </header>

      <div className="settings-page__grid">
        <SettingsSection
          className="settings-card--wide"
          title={t("setup.settings.instance.title")}
          icon={<Server size={20} />}
        >
          <form
            className="instance-settings-form"
            onSubmit={event => {
              event.preventDefault();
              void handleSaveInstanceSettings();
            }}
          >
            <div className="instance-settings-fields">
              <label>
                <span>{t("setup.settings.instance.name")}</span>
                <input
                  data-testid="instance-name-input"
                  value={instanceName}
                  maxLength={100}
                  disabled={loadingInstance || savingInstance}
                  onChange={event => setInstanceName(event.target.value)}
                />
              </label>
              <TimezoneSelect
                label={t("setup.settings.instance.timezone")}
                help={t("setup.settings.instance.timezoneHelp")}
                value={instanceTimezone}
                disabled={loadingInstance || savingInstance}
                onChange={setInstanceTimezone}
              />
            </div>
            <div>
              <Button
                data-testid="save-instance-name"
                type="submit"
                loading={savingInstance}
                disabled={
                  loadingInstance || savingInstance || !instanceName.trim()
                }
                icon={<Save size={17} aria-hidden="true" />}
              >
                {t("setup.common.save")}
              </Button>
            </div>
          </form>
          {instanceError ? (
            <p className="error-message">{instanceError}</p>
          ) : null}
        </SettingsSection>
        <StorageCapacityCard
          settings={diskGuard}
          loading={loadingDiskGuard}
          saving={savingDiskGuard}
          saved={diskGuardSaved}
          error={diskGuardError}
          isAdmin={isAdmin}
          onChange={nextSettings => {
            setDiskGuardSaved(false);
            setDiskGuard(nextSettings);
          }}
          onToggle={enabled =>
            diskGuard && void handleSaveDiskGuard({ ...diskGuard, enabled })
          }
          onSave={() => void handleSaveDiskGuard()}
        />
                <OperationalWebhookCard
          settings={operationalWebhook}
          loading={loadingOperationalWebhook}
          saving={savingOperationalWebhook}
          saved={operationalWebhookSaved}
          error={operationalWebhookError}
          onChange={settings => {
            setOperationalWebhookSaved(false);
            setOperationalWebhook(settings);
          }}
          onToggle={enabled => {
            if (!operationalWebhook) {
              return;
            }
            const nextSettings = { ...operationalWebhook, enabled };
            if (enabled) {
              setOperationalWebhookSaved(false);
              setOperationalWebhook(nextSettings);
              return;
            }
            void handleSaveOperationalWebhook(nextSettings);
          }}
          onSave={() => void handleSaveOperationalWebhook()}
        />
        <OidcSettingsCard
          settings={oidc}
          onSettingsUpdated={setOidc}
        />
        <ServerUpdateCard
          status={serverUpdate}
          loading={loadingServerUpdate}
          requesting={requestingServerUpdate}
          error={serverUpdateError}
          restartPending={restartPending}
          onUpdate={() => setServerUpdateConfirmation(true)}
        />
        <SpeedTestCard />

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
          onRevokeApplication={(id, name) =>
            setDestructiveConfirmation({ kind: "application", id, name })
          }
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
          onPageChange={page => void refreshKeys(page)}
          onRevokeKey={(id, name) =>
            setDestructiveConfirmation({ kind: "s3Key", id, name })
          }
        />
        {isAdmin ? (
          <ConfigurationBackupCard
            importing={configurationImporting}
            result={configurationResult}
            error={configurationError}
            onExport={() => void handleExportConfiguration()}
            onImport={file => void handleImportConfiguration(file)}
          />
        ) : null}
      </div>
      {destructiveConfirmation ? (
        <ConfirmDialog
          title={
            destructiveConfirmation.kind === "s3Key"
              ? t("setup.settings.s3.confirmRevokeTitle")
              : t("setup.settings.applications.confirmRevokeTitle")
          }
          description={
            destructiveConfirmation.kind === "s3Key"
              ? t("setup.settings.s3.confirmRevokeDescription", {
                  name: destructiveConfirmation.name,
                })
              : t("setup.settings.applications.confirmRevokeDescription", {
                  name: destructiveConfirmation.name,
                })
          }
          onCancel={() => setDestructiveConfirmation(null)}
          onConfirm={() => {
            if (destructiveConfirmation.kind === "s3Key") {
              void handleRevokeKey(destructiveConfirmation.id);
              return;
            }
            void handleRevokeApplication(destructiveConfirmation.id);
          }}
        />
      ) : null}
      {serverUpdateConfirmation && serverUpdate ? (
        <ConfirmDialog
          title={t("setup.settings.update.confirmTitle")}
          description={t("setup.settings.update.confirmDescription", {
            version: serverUpdate.latestVersion,
          })}
          confirmLabel={t("setup.settings.update.confirmAction")}
          onCancel={() => setServerUpdateConfirmation(false)}
          onConfirm={() => void handleRequestServerUpdate()}
        />
      ) : null}
    </div>
  );
}


