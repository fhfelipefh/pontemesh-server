import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Ban, Copy, KeyRound, Plus } from "lucide-react";
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

  return (
    <div className="settings-page">
      <header className="settings-page__header">
        <div>
          <h1>{t("setup.settings.title")}</h1>
          <p>{t("setup.settings.description")}</p>
        </div>
      </header>

      <div className="settings-page__grid">
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
            <p>{t("setup.settings.applications.description")}</p>
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
