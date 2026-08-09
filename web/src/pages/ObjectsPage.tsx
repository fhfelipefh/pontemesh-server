import { useCallback, useEffect, useMemo, useState } from "react";
import { Database, RefreshCw } from "lucide-react";
import { useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { BucketSummary, deleteObject, listBuckets } from "../api/bucketsApi";
import { Button } from "../components/Button";
import { ConfirmDialog, EmptyState } from "../components/AdminListControls";
import { ErrorMessage } from "../components/ErrorMessage";
import { ObjectManager } from "../components/ObjectManager";
import { formatBytes } from "../utils/adminFormat";

type ConfirmationState = { bucket: string; objectKey: string } | null;

export function ObjectsPage() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const bucketParam = searchParams.get("bucket")?.trim() ?? "";
  const [buckets, setBuckets] = useState<BucketSummary[]>([]);
  const [selectedBucketName, setSelectedBucketName] = useState(bucketParam);
  const [loadingBuckets, setLoadingBuckets] = useState(true);
  const [bucketError, setBucketError] = useState("");
  const [objectActionError, setObjectActionError] = useState("");
  const [refreshNonce, setRefreshNonce] = useState(0);
  const [confirmation, setConfirmation] = useState<ConfirmationState>(null);

  const selectedBucket = useMemo(
    () => buckets.find((bucket) => bucket.name === selectedBucketName) ?? null,
    [buckets, selectedBucketName]
  );

  const refreshBuckets = useCallback(async () => {
    setLoadingBuckets(true);
    setBucketError("");
    try {
      const nextPage = await listBuckets({ page: 1, pageSize: 100 });
      setBuckets(nextPage.items);
      setSelectedBucketName((current) => {
        if (current && nextPage.items.some((bucket) => bucket.name === current)) {
          return current;
        }
        if (bucketParam && nextPage.items.some((bucket) => bucket.name === bucketParam)) {
          return bucketParam;
        }
        return nextPage.items[0]?.name ?? "";
      });
    } catch (loadError) {
      setBucketError(loadError instanceof Error ? loadError.message : t("setup.buckets.loadFailed"));
    } finally {
      setLoadingBuckets(false);
    }
  }, [bucketParam, t]);

  useEffect(() => {
    refreshBuckets();
  }, [refreshBuckets]);

  useEffect(() => {
    if (!bucketParam) {
      return;
    }
    setSelectedBucketName(bucketParam);
  }, [bucketParam]);

  useEffect(() => {
    if (!selectedBucketName) {
      return;
    }
    if (selectedBucketName !== bucketParam) {
      setSearchParams({ bucket: selectedBucketName }, { replace: true });
    }
  }, [bucketParam, selectedBucketName, setSearchParams]);

  async function handleDeleteObject(bucket: string, objectKey: string) {
    setObjectActionError("");
    try {
      await deleteObject(bucket, objectKey);
      setConfirmation(null);
      setRefreshNonce((nonce) => nonce + 1);
      await refreshBuckets();
    } catch (deleteError) {
      setConfirmation(null);
      setObjectActionError(deleteError instanceof Error ? deleteError.message : t("setup.objects.deleteFailed"));
    }
  }

  function handleBucketChange(nextBucketName: string) {
    setObjectActionError("");
    setSelectedBucketName(nextBucketName);
  }

  const hasBuckets = buckets.length > 0;
  const selectedBucketObjectLabel = selectedBucket?.objectCount === 1
    ? t("setup.objects.objectSingular")
    : t("setup.objects.objectPlural");

  return (
    <div className="objects-page">
      <div className="objects-page-header">
        <h1>{t("setup.objects.title")}</h1>
        <Button
          className="refresh-button"
          type="button"
          icon={<RefreshCw size={17} aria-hidden="true" />}
          onClick={() => {
            setRefreshNonce((nonce) => nonce + 1);
            void refreshBuckets();
          }}
        >
          {t("setup.objects.refresh")}
        </Button>
      </div>

      <ErrorMessage message={bucketError} />

      {loadingBuckets ? (
        <div className="admin-loading">{t("setup.buckets.loading")}</div>
      ) : !hasBuckets ? (
        <EmptyState
          title={t("setup.objects.noBucketsTitle")}
        />
      ) : (
        <>
          <section className="objects-summary-card">
            <label className="bucket-context-row">
              <span>{t("setup.objects.bucket")}</span>
              <select
                data-testid="objects-bucket-select"
                value={selectedBucketName}
                onChange={(event) => handleBucketChange(event.target.value)}
              >
                {buckets.map((bucket) => (
                  <option value={bucket.name} key={bucket.name}>
                    {bucket.name}
                  </option>
                ))}
              </select>
            </label>
            {selectedBucket ? (
              <div className="objects-bucket-summary" data-testid="objects-bucket-summary">
                <Database size={17} aria-hidden="true" />
                <span>
                  {t("setup.objects.bucketSummary", {
                    count: selectedBucket.objectCount,
                    label: selectedBucketObjectLabel,
                    bytes: formatBytes(selectedBucket.totalBytes)
                  })}
                </span>
              </div>
            ) : null}
          </section>

          {selectedBucketName ? (
            <ObjectManager
              bucketName={selectedBucketName}
              refreshNonce={refreshNonce}
              externalError={objectActionError}
              onChanged={refreshBuckets}
              onConfirmDeleteObject={(objectKey) => setConfirmation({ bucket: selectedBucketName, objectKey })}
            />
          ) : null}
        </>
      )}

      {confirmation ? (
        <ConfirmDialog
          title={t("setup.objects.confirmDeleteTitle")}
          description={t("setup.objects.confirmDeleteDescription", { key: confirmation.objectKey })}
          onCancel={() => setConfirmation(null)}
          onConfirm={() => void handleDeleteObject(confirmation.bucket, confirmation.objectKey)}
        />
      ) : null}
    </div>
  );
}
