import { Ban, ChevronLeft, ChevronRight, Info, KeyRound, Plus } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { CreatedS3AccessKey, S3AccessKeySummary } from "../../api/s3KeysApi";
import { Button } from "../Button";
import { ErrorMessage } from "../ErrorMessage";
import { CopyButton } from "./CopyButton";
import { StatusBadge } from "./StatusBadge";

const S3_KEYS_PER_PAGE = 5;

type S3CredentialsCardProps = {
  keys: S3AccessKeySummary[];
  createdKey: CreatedS3AccessKey | null;
  keyName: string;
  loading: boolean;
  creating: boolean;
  revoking: string | null;
  error: string;
  onKeyNameChange: (value: string) => void;
  onCreateKey: () => void;
  onRevokeKey: (id: string) => void;
};

export function S3CredentialsCard({
  keys,
  createdKey,
  keyName,
  loading,
  creating,
  revoking,
  error,
  onKeyNameChange,
  onCreateKey,
  onRevokeKey
}: S3CredentialsCardProps) {
  const { t, i18n } = useTranslation();

  return (
    <section className="settings-card">
      <div className="settings-card__header">
        <div>
          <h2>{t("setup.settings.s3.title")}</h2>
          <p>{t("setup.settings.s3.description")}</p>
        </div>
        <Button
          type="button"
          loading={creating}
          onClick={onCreateKey}
          icon={<Plus size={17} aria-hidden="true" />}
        >
          {t("setup.settings.s3.create")}
        </Button>
      </div>

      <div className="settings-key-name-field">
        <label htmlFor="s3-key-name">{t("setup.settings.s3.name")}</label>
        <input
          id="s3-key-name"
          type="text"
          value={keyName}
          onChange={(event) => onKeyNameChange(event.target.value)}
          placeholder={t("setup.settings.s3.namePlaceholder")}
        />
      </div>

      <ErrorMessage message={error} />

      {createdKey && (
        <div className="settings-secret-panel" role="status">
          <div className="settings-secret-panel__title">
            <KeyRound size={18} aria-hidden="true" />
            <strong>{t("setup.settings.s3.createdTitle")}</strong>
          </div>
          <dl>
            <div>
              <dt>{t("setup.settings.s3.accessKeyId")}</dt>
              <dd>
                <code>{createdKey.accessKeyId}</code>
                <CopyButton value={createdKey.accessKeyId} label={t("setup.settings.s3.copyAccessKeyId")} />
              </dd>
            </div>
            <div>
              <dt>{t("setup.settings.s3.secretAccessKey")}</dt>
              <dd>
                <code>{createdKey.secretAccessKey}</code>
                <CopyButton value={createdKey.secretAccessKey} label={t("setup.settings.s3.copySecretAccessKey")} />
              </dd>
            </div>
          </dl>
          <p>{t("setup.settings.s3.createdHint")}</p>
        </div>
      )}

      {loading ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : keys.length === 0 ? (
        <div className="settings-empty-state">
          <h3>{t("setup.settings.s3.emptyTitle")}</h3>
          <p>{t("setup.settings.s3.emptyDescription")}</p>
        </div>
      ) : (
        <S3CredentialsTable
          keys={keys}
          locale={i18n.language}
          revoking={revoking}
          onRevokeKey={onRevokeKey}
        />
      )}

      <div className="settings-inline-help">
        <Info size={18} aria-hidden="true" />
        <p>{t("setup.settings.s3.helpText")}</p>
      </div>
    </section>
  );
}

type S3CredentialsTableProps = {
  keys: S3AccessKeySummary[];
  locale: string;
  revoking: string | null;
  onRevokeKey: (id: string) => void;
};

function S3CredentialsTable({ keys, locale, revoking, onRevokeKey }: S3CredentialsTableProps) {
  const { t } = useTranslation();
  const [currentPage, setCurrentPage] = useState(1);
  const totalPages = Math.max(1, Math.ceil(keys.length / S3_KEYS_PER_PAGE));
  const firstVisibleKey = (currentPage - 1) * S3_KEYS_PER_PAGE + 1;
  const lastVisibleKey = Math.min(currentPage * S3_KEYS_PER_PAGE, keys.length);
  const visibleKeys = useMemo(
    () => keys.slice((currentPage - 1) * S3_KEYS_PER_PAGE, currentPage * S3_KEYS_PER_PAGE),
    [currentPage, keys]
  );

  useEffect(() => {
    setCurrentPage((page) => Math.min(page, totalPages));
  }, [totalPages]);

  return (
    <>
      <div className="settings-table-wrap">
        <table className="settings-table">
          <thead>
            <tr>
              <th>{t("setup.settings.s3.name")}</th>
              <th>{t("setup.settings.s3.accessKeyId")}</th>
              <th>{t("setup.settings.s3.status")}</th>
              <th>{t("setup.settings.s3.lastUsed")}</th>
              <th>{t("setup.settings.s3.createdAt")}</th>
              <th aria-label={t("setup.settings.s3.actions")} />
            </tr>
          </thead>
          <tbody>
            {visibleKeys.map((key) => (
              <tr key={key.id}>
                <td className="settings-table__name">{key.name ?? t("setup.common.unavailable")}</td>
                <td>
                  <div className="settings-access-key">
                    <code>{key.accessKeyId}</code>
                    <CopyButton value={key.accessKeyId} label={t("setup.settings.s3.copyAccessKeyId")} />
                  </div>
                </td>
                <td>
                  <StatusBadge
                    active={key.isActive}
                    activeLabel={t("setup.settings.s3.active")}
                    revokedLabel={t("setup.settings.s3.revoked")}
                  />
                </td>
                <td>{key.lastUsedAt ? formatDate(key.lastUsedAt, locale) : t("setup.common.notApplicable")}</td>
                <td>{formatDate(key.createdAt, locale)}</td>
                <td>
                  {key.isActive && (
                    <button
                      className="settings-revoke-button"
                      type="button"
                      title={t("setup.settings.s3.revoke")}
                      aria-label={t("setup.settings.s3.revoke")}
                      disabled={revoking === key.id}
                      onClick={() => onRevokeKey(key.id)}
                    >
                      <Ban size={16} aria-hidden="true" />
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {keys.length > S3_KEYS_PER_PAGE && (
        <nav className="settings-pagination" aria-label={t("setup.settings.s3.paginationLabel")}>
          <p>
            {t("setup.settings.s3.paginationSummary", {
              start: firstVisibleKey,
              end: lastVisibleKey,
              total: keys.length
            })}
          </p>
          <div className="settings-pagination__actions">
            <button
              type="button"
              className="settings-pagination__button"
              aria-label={t("setup.settings.s3.previousPage")}
              disabled={currentPage === 1}
              onClick={() => setCurrentPage((page) => Math.max(1, page - 1))}
            >
              <ChevronLeft size={16} aria-hidden="true" />
            </button>
            <span>{t("setup.settings.s3.pageIndicator", { page: currentPage, total: totalPages })}</span>
            <button
              type="button"
              className="settings-pagination__button"
              aria-label={t("setup.settings.s3.nextPage")}
              disabled={currentPage === totalPages}
              onClick={() => setCurrentPage((page) => Math.min(totalPages, page + 1))}
            >
              <ChevronRight size={16} aria-hidden="true" />
            </button>
          </div>
        </nav>
      )}
    </>
  );
}

function formatDate(value: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "short"
  }).format(new Date(value));
}
