import { Ban, Check, ChevronLeft, ChevronRight, KeyRound, Plus, X } from "lucide-react";
import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { CreatedS3AccessKey, S3AccessKeySummary } from "../../api/s3KeysApi";
import { Button } from "../Button";
import { ErrorMessage } from "../ErrorMessage";
import { CredentialTable } from "./CredentialTable";
import { CopyButton } from "./CopyButton";
import { EmptyState } from "./EmptyState";
import { IconButton } from "./IconButton";
import { InfoBox } from "./InfoBox";
import { SettingsSection } from "./SettingsSection";
import { StatusBadge } from "./StatusBadge";

type S3CredentialsCardProps = {
  keys: S3AccessKeySummary[];
  createdKey: CreatedS3AccessKey | null;
  keyName: string;
  loading: boolean;
  creating: boolean;
  revoking: string | null;
  error: string;
  currentPage: number;
  pageSize: number;
  totalKeys: number;
  totalPages: number;
  onKeyNameChange: (value: string) => void;
  onCreateKey: () => void;
  onDismissCreatedKey: () => void;
  onPageChange: (page: number) => void;
  onRevokeKey: (id: string, name: string) => void;
};

export function S3CredentialsCard({
  keys,
  createdKey,
  keyName,
  loading,
  creating,
  revoking,
  error,
  currentPage,
  pageSize,
  totalKeys,
  totalPages,
  onKeyNameChange,
  onCreateKey,
  onDismissCreatedKey,
  onPageChange,
  onRevokeKey
}: S3CredentialsCardProps) {
  const { t, i18n } = useTranslation();
  const [createDialogOpen, setCreateDialogOpen] = useState(false);

  function handleCreateKey() {
    onCreateKey();
  }

  useEffect(() => {
    if (createdKey) {
      setCreateDialogOpen(false);
    }
  }, [createdKey]);

  return (
    <SettingsSection
      title={t("setup.settings.s3.title")}
      icon={<KeyRound size={20} />}
      actions={
        <Button
          className="settings-create-key-button"
          type="button"
          loading={creating}
          onClick={() => setCreateDialogOpen(true)}
          icon={<Plus size={17} aria-hidden="true" />}
        >
          {t("setup.settings.s3.create")}
        </Button>
      }
    >

      <ErrorMessage message={error} />

      {createdKey && (
        <S3SecretModal createdKey={createdKey} onClose={onDismissCreatedKey} />
      )}

      {loading ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : keys.length === 0 ? (
        <EmptyState
          icon={<KeyRound size={22} />}
          title={t("setup.settings.s3.emptyTitle")}
        />
      ) : (
        <S3CredentialsTable
          keys={keys}
          locale={i18n.language}
          currentPage={currentPage}
          pageSize={pageSize}
          totalKeys={totalKeys}
          totalPages={totalPages}
          revoking={revoking}
          onPageChange={onPageChange}
          onRevokeKey={onRevokeKey}
        />
      )}

      <InfoBox>
        <p>{t("setup.settings.s3.helpText")}</p>
      </InfoBox>

      {createDialogOpen && (
        <CreateS3KeyModal
          keyName={keyName}
          creating={creating}
          onKeyNameChange={onKeyNameChange}
          onCreateKey={handleCreateKey}
          onClose={() => setCreateDialogOpen(false)}
        />
      )}
    </SettingsSection>
  );
}

type CreateS3KeyModalProps = {
  keyName: string;
  creating: boolean;
  onKeyNameChange: (value: string) => void;
  onCreateKey: () => void;
  onClose: () => void;
};

function CreateS3KeyModal({
  keyName,
  creating,
  onKeyNameChange,
  onCreateKey,
  onClose
}: CreateS3KeyModalProps) {
  const { t } = useTranslation();

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onCreateKey();
  }

  return (
    <div className="settings-modal-backdrop" role="presentation">
      <form
        className="settings-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="s3-create-key-title"
        onSubmit={handleSubmit}
      >
        <div className="settings-modal__header">
          <div>
            <h3 id="s3-create-key-title">{t("setup.settings.s3.createModalTitle")}</h3>
          </div>
          <button
            className="settings-modal__close"
            type="button"
            title={t("setup.common.cancel")}
            aria-label={t("setup.common.cancel")}
            onClick={onClose}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </div>
        <label className="settings-modal-field" htmlFor="s3-key-name">
          <span>{t("setup.settings.s3.name")}</span>
          <input
            id="s3-key-name"
            type="text"
            value={keyName}
            onChange={(event) => onKeyNameChange(event.target.value)}
            placeholder={t("setup.settings.s3.namePlaceholder")}
            autoFocus
          />
        </label>
        <div className="settings-modal__actions">
          <button className="settings-secondary-button" type="button" onClick={onClose}>
            <X size={16} aria-hidden="true" />
            {t("setup.common.cancel")}
          </button>
          <Button
            className="settings-modal__primary"
            type="submit"
            loading={creating}
            icon={<Plus size={17} aria-hidden="true" />}
          >
            {t("setup.settings.s3.create")}
          </Button>
        </div>
      </form>
    </div>
  );
}

type S3SecretModalProps = {
  createdKey: CreatedS3AccessKey;
  onClose: () => void;
};

function S3SecretModal({ createdKey, onClose }: S3SecretModalProps) {
  const { t } = useTranslation();

  return (
    <div className="settings-modal-backdrop" role="presentation">
      <section
        className="settings-modal settings-modal--secret"
        role="dialog"
        aria-modal="true"
        aria-labelledby="s3-secret-title"
      >
        <div className="settings-modal__header">
          <div>
            <h3 id="s3-secret-title">{t("setup.settings.s3.createdTitle")}</h3>
            <p>{t("setup.settings.s3.createdHint")}</p>
          </div>
          <button
            className="settings-modal__close"
            type="button"
            title={t("setup.common.ok")}
            aria-label={t("setup.common.ok")}
            onClick={onClose}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </div>
        <dl className="settings-secret-list">
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
        <div className="settings-modal__actions">
          <Button className="settings-modal__primary" type="button" onClick={onClose} icon={<Check size={17} aria-hidden="true" />}>
            {t("setup.common.ok")}
          </Button>
        </div>
      </section>
    </div>
  );
}

type S3CredentialsTableProps = {
  keys: S3AccessKeySummary[];
  locale: string;
  currentPage: number;
  pageSize: number;
  totalKeys: number;
  totalPages: number;
  revoking: string | null;
  onPageChange: (page: number) => void;
  onRevokeKey: (id: string, name: string) => void;
};

function S3CredentialsTable({
  keys,
  locale,
  currentPage,
  pageSize,
  totalKeys,
  totalPages,
  revoking,
  onPageChange,
  onRevokeKey
}: S3CredentialsTableProps) {
  const { t } = useTranslation();
  const firstVisibleKey = totalKeys === 0 ? 0 : (currentPage - 1) * pageSize + 1;
  const lastVisibleKey = Math.min(currentPage * pageSize, totalKeys);

  return (
    <>
      <CredentialTable
        columns={[
          { key: "name", label: t("setup.settings.s3.name"), className: "settings-table__col-name" },
          { key: "accessKeyId", label: t("setup.settings.s3.accessKeyId"), className: "settings-table__col-key" },
          { key: "status", label: t("setup.settings.s3.status"), className: "settings-table__col-status" },
          { key: "lastUsed", label: t("setup.settings.s3.lastUsed"), className: "settings-table__col-last-used" },
          { key: "createdAt", label: t("setup.settings.s3.createdAt"), className: "settings-table__col-created" },
          { key: "actions", ariaLabel: t("setup.settings.s3.actions"), className: "settings-table__col-actions" }
        ]}
        minWidth={940}
      >
            {keys.map((key) => (
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
                <td className="settings-table__actions">
                  {key.isActive && (
                    <IconButton
                      variant="danger"
                      label={t("setup.settings.s3.revoke")}
                      icon={<Ban size={16} aria-hidden="true" />}
                      disabled={revoking === key.id}
                      onClick={() => onRevokeKey(key.id, key.name ?? key.accessKeyId)}
                    />
                  )}
                </td>
              </tr>
            ))}
      </CredentialTable>
      {totalKeys > pageSize && (
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
              onClick={() => onPageChange(Math.max(1, currentPage - 1))}
            >
              <ChevronLeft size={16} aria-hidden="true" />
            </button>
            <span>{t("setup.settings.s3.pageIndicator", { page: currentPage, total: totalPages })}</span>
            <button
              type="button"
              className="settings-pagination__button"
              aria-label={t("setup.settings.s3.nextPage")}
              disabled={currentPage === totalPages}
              onClick={() => onPageChange(Math.min(totalPages, currentPage + 1))}
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
