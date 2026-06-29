import { useCallback, useEffect, useState } from "react";
import { KeyRound, Plus, RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  CreatedS3AccessKey,
  S3AccessKeySummary,
  createS3AccessKey,
  listS3AccessKeys,
  revokeS3AccessKey
} from "../api/s3KeysApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";

export function SettingsPage() {
  const { t } = useTranslation();
  const [keys, setKeys] = useState<S3AccessKeySummary[]>([]);
  const [createdKey, setCreatedKey] = useState<CreatedS3AccessKey | null>(null);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [revoking, setRevoking] = useState<string | null>(null);
  const [error, setError] = useState("");

  const refreshKeys = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setKeys(await listS3AccessKeys());
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : t("setup.settings.s3.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshKeys();
  }, [refreshKeys]);

  async function handleCreateKey() {
    setCreating(true);
    setError("");
    try {
      const nextKey = await createS3AccessKey();
      setCreatedKey(nextKey);
      await refreshKeys();
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : t("setup.settings.s3.createFailed"));
    } finally {
      setCreating(false);
    }
  }

  async function handleRevokeKey(accessKeyId: string) {
    setRevoking(accessKeyId);
    setError("");
    try {
      await revokeS3AccessKey(accessKeyId);
      await refreshKeys();
    } catch (revokeError) {
      setError(revokeError instanceof Error ? revokeError.message : t("setup.settings.s3.revokeFailed"));
    } finally {
      setRevoking(null);
    }
  }

  return (
    <div className="settings-layout">
      <section className="admin-panel admin-panel--wide">
        <div className="admin-panel__header">
          <div>
            <h1>{t("setup.settings.title")}</h1>
            <p>{t("setup.settings.description")}</p>
          </div>
        </div>
      </section>

      <section className="admin-panel admin-panel--wide">
        <div className="admin-panel__header">
          <div>
            <h2>{t("setup.settings.s3.title")}</h2>
            <p>{t("setup.settings.s3.description")}</p>
          </div>
          <Button
            type="button"
            loading={creating}
            onClick={handleCreateKey}
            icon={<Plus size={17} aria-hidden="true" />}
          >
            {t("setup.settings.s3.create")}
          </Button>
        </div>

        <ErrorMessage message={error} />

        {createdKey && (
          <div className="secret-panel" role="status">
            <div>
              <KeyRound size={18} aria-hidden="true" />
              <strong>{t("setup.settings.s3.createdTitle")}</strong>
            </div>
            <dl>
              <div>
                <dt>{t("setup.settings.s3.accessKeyId")}</dt>
                <dd>{createdKey.key.accessKeyId}</dd>
              </div>
              <div>
                <dt>{t("setup.settings.s3.secretAccessKey")}</dt>
                <dd>{createdKey.secretAccessKey}</dd>
              </div>
            </dl>
            <p>{t("setup.settings.s3.createdHint")}</p>
          </div>
        )}

        {loading ? (
          <div className="admin-loading">{t("setup.common.loading")}</div>
        ) : keys.length === 0 ? (
          <div className="empty-state">
            <h3>{t("setup.settings.s3.emptyTitle")}</h3>
            <p>{t("setup.settings.s3.emptyDescription")}</p>
          </div>
        ) : (
          <div className="object-table s3-keys-table">
            <div className="object-table__head">
              <span>{t("setup.settings.s3.accessKeyId")}</span>
              <span>{t("setup.settings.s3.status")}</span>
              <span>{t("setup.settings.s3.lastUsed")}</span>
              <span>{t("setup.settings.s3.createdAt")}</span>
              <span />
            </div>
            {keys.map((key) => (
              <div className="object-table__row" key={key.id}>
                <span>{key.accessKeyId}</span>
                <span>{key.isActive ? t("setup.settings.s3.active") : t("setup.settings.s3.revoked")}</span>
                <span>{key.lastUsedAt ? formatDate(key.lastUsedAt) : t("setup.common.unavailable")}</span>
                <span>{formatDate(key.createdAt)}</span>
                <span>
                  {key.isActive && (
                    <button
                      className="icon-button"
                      type="button"
                      title={t("setup.settings.s3.revoke")}
                      aria-label={t("setup.settings.s3.revoke")}
                      disabled={revoking === key.accessKeyId}
                      onClick={() => handleRevokeKey(key.accessKeyId)}
                    >
                      <RotateCcw size={17} aria-hidden="true" />
                    </button>
                  )}
                </span>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "short",
    timeStyle: "short"
  }).format(new Date(value));
}
