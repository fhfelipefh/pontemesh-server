import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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

  return (
    <div className="settings-page">
      <header className="settings-page__header">
        <div>
          <h1>{t("setup.settings.title")}</h1>
          <p>{t("setup.settings.description")}</p>
        </div>
      </header>

      <div className="settings-page__grid">
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
