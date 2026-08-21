import { useTranslation } from "react-i18next";
import { Ban, Check, KeyRound, Plus } from "lucide-react";
import {
  ApplicationCredentialSummary,
  CreatedApplicationCredential,
} from "../../api/applicationCredentialsApi";
import { SettingsSection } from "./SettingsSection";
import { CopyButton } from "./CopyButton";
import { CredentialTable } from "./CredentialTable";
import { EmptyState } from "./EmptyState";
import { StatusBadge } from "./StatusBadge";
import { IconButton } from "./IconButton";

export type ApplicationCredentialsCardProps = {
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

export function ApplicationCredentialsCard({
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
  onRevokeApplication,
}: ApplicationCredentialsCardProps) {
  const { t, i18n } = useTranslation();

  return (
    <SettingsSection
      title={t("setup.settings.applications.title")}
      icon={<KeyRound size={20} />}
    >
      <form
        className="inline-form"
        onSubmit={event => {
          event.preventDefault();
          onCreateApplication();
        }}
      >
        <input
          value={applicationName}
          onChange={event => onApplicationNameChange(event.target.value)}
          placeholder={t("setup.settings.applications.namePlaceholder")}
          aria-label={t("setup.settings.applications.name")}
        />
        <select
          value={applicationPreset}
          onChange={event =>
            onApplicationPresetChange(
              event.target.value as "downloader" | "full"
            )
          }
          aria-label={t("setup.settings.applications.preset")}
        >
          <option value="downloader">
            {t("setup.settings.applications.downloaderPreset")}
          </option>
          <option value="full">
            {t("setup.settings.applications.fullPreset")}
          </option>
        </select>
        <button
          className="settings-create-key-button"
          type="submit"
          disabled={creating || !applicationName.trim()}
        >
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
              <dd>
                <code>{createdApplication.credential.id}</code>
              </dd>
            </div>
            <div>
              <dt>{t("setup.settings.applications.token")}</dt>
              <dd>
                <code>{createdApplication.token}</code>
                <CopyButton
                  value={createdApplication.token}
                  label={t("setup.settings.applications.copyToken")}
                />
              </dd>
            </div>
          </dl>
          <button
            className="settings-secondary-button"
            type="button"
            onClick={onDismissCreatedApplication}
          >
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
        />
      ) : (
        <CredentialTable
          columns={[
            {
              key: "name",
              label: t("setup.settings.applications.name"),
              className: "settings-table__col-name",
            },
            {
              key: "scopes",
              label: t("setup.settings.applications.scopes"),
              className: "settings-table__col-key",
            },
            {
              key: "status",
              label: t("setup.settings.s3.status"),
              className: "settings-table__col-status",
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
          minWidth={820}
        >
          {applications.map(application => (
            <tr key={application.id}>
              <td className="settings-table__name">{application.name}</td>
              <td>
                {formatScopes(
                  application.scopes,
                  t("setup.common.unavailable")
                )}
              </td>
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
                    onClick={() =>
                      onRevokeApplication(application.id, application.name)
                    }
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
    timeStyle: "short",
  }).format(new Date(value));
}

function formatScopes(scopes: string[] | undefined, fallback: string): string {
  return scopes && scopes.length > 0 ? scopes.join(", ") : fallback;
}
