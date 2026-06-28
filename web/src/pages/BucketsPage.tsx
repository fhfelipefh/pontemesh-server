import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { Plus, Search, Trash2, UploadCloud } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  BucketSummary,
  ObjectSummary,
  createBucket,
  deleteBucket,
  deleteObject,
  listBuckets,
  listObjects,
  uploadObject
} from "../api/bucketsApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";

export function BucketsPage() {
  const { t } = useTranslation();
  const [buckets, setBuckets] = useState<BucketSummary[]>([]);
  const [selectedBucket, setSelectedBucket] = useState<string | null>(null);
  const [objects, setObjects] = useState<ObjectSummary[]>([]);
  const [bucketName, setBucketName] = useState("");
  const [query, setQuery] = useState("");
  const [objectKey, setObjectKey] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [loadingBuckets, setLoadingBuckets] = useState(true);
  const [loadingObjects, setLoadingObjects] = useState(false);
  const [submittingBucket, setSubmittingBucket] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState("");

  const filteredObjects = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) {
      return objects;
    }
    return objects.filter((object) => object.key.toLowerCase().includes(normalized));
  }, [objects, query]);

  const refreshBuckets = useCallback(async () => {
    setLoadingBuckets(true);
    setError("");
    try {
      const nextBuckets = await listBuckets();
      setBuckets(nextBuckets);
      setSelectedBucket((current) => current ?? nextBuckets[0]?.name ?? null);
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : t("setup.buckets.loadFailed"));
    } finally {
      setLoadingBuckets(false);
    }
  }, [t]);

  const refreshObjects = useCallback(async (bucket: string) => {
    setLoadingObjects(true);
    setError("");
    try {
      setObjects(await listObjects(bucket));
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : t("setup.objects.loadFailed"));
    } finally {
      setLoadingObjects(false);
    }
  }, [t]);

  useEffect(() => {
    refreshBuckets();
  }, [refreshBuckets]);

  useEffect(() => {
    if (!selectedBucket) {
      setObjects([]);
      return;
    }
    refreshObjects(selectedBucket);
  }, [refreshObjects, selectedBucket]);

  async function handleCreateBucket(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!bucketName.trim()) {
      return;
    }
    setSubmittingBucket(true);
    setError("");
    try {
      const created = await createBucket(bucketName.trim());
      setBucketName("");
      await refreshBuckets();
      setSelectedBucket(created.name);
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : t("setup.buckets.createFailed"));
    } finally {
      setSubmittingBucket(false);
    }
  }

  async function handleDeleteBucket(bucket: string) {
    setError("");
    try {
      await deleteBucket(bucket);
      if (selectedBucket === bucket) {
        setSelectedBucket(null);
      }
      await refreshBuckets();
    } catch (deleteError) {
      setError(deleteError instanceof Error ? deleteError.message : t("setup.buckets.deleteFailed"));
    }
  }

  async function handleUpload(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedBucket || !file) {
      return;
    }
    setUploading(true);
    setError("");
    try {
      await uploadObject(selectedBucket, file, objectKey);
      setFile(null);
      setObjectKey("");
      event.currentTarget.reset();
      await refreshObjects(selectedBucket);
      await refreshBuckets();
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : t("setup.objects.uploadFailed"));
    } finally {
      setUploading(false);
    }
  }

  async function handleDeleteObject(objectKeyToDelete: string) {
    if (!selectedBucket) {
      return;
    }
    setError("");
    try {
      await deleteObject(selectedBucket, objectKeyToDelete);
      await refreshObjects(selectedBucket);
      await refreshBuckets();
    } catch (deleteError) {
      setError(deleteError instanceof Error ? deleteError.message : t("setup.objects.deleteFailed"));
    }
  }

  return (
    <div className="buckets-layout">
      <section className="admin-panel bucket-sidebar">
        <div className="admin-panel__header">
          <div>
            <h1>{t("setup.buckets.title")}</h1>
            <p>{t("setup.buckets.description")}</p>
          </div>
        </div>

        <form className="inline-form" onSubmit={handleCreateBucket}>
          <input
            value={bucketName}
            onChange={(event) => setBucketName(event.target.value)}
            placeholder={t("setup.buckets.namePlaceholder")}
            aria-label={t("setup.buckets.name")}
          />
          <Button
            type="submit"
            loading={submittingBucket}
            disabled={!bucketName.trim()}
            icon={<Plus size={17} aria-hidden="true" />}
          >
            {t("setup.buckets.create")}
          </Button>
        </form>

        <ErrorMessage message={error} />

        {loadingBuckets ? (
          <div className="admin-loading">{t("setup.common.loading")}</div>
        ) : buckets.length === 0 ? (
          <EmptyState title={t("setup.buckets.emptyTitle")} description={t("setup.buckets.emptyDescription")} />
        ) : (
          <div className="bucket-list">
            {buckets.map((bucket) => (
              <button
                className="bucket-list__item"
                type="button"
                data-active={selectedBucket === bucket.name}
                key={bucket.name}
                onClick={() => setSelectedBucket(bucket.name)}
              >
                <span>{bucket.name}</span>
                <small>
                  {bucket.objectCount} {t("setup.objects.count")} · {formatBytes(bucket.totalBytes)}
                </small>
              </button>
            ))}
          </div>
        )}
      </section>

      <section className="admin-panel object-panel">
        {selectedBucket ? (
          <>
            <div className="admin-panel__header">
              <div>
                <h2>{selectedBucket}</h2>
                <p>{t("setup.objects.description")}</p>
              </div>
              <button
                className="icon-button"
                type="button"
                title={t("setup.buckets.delete")}
                aria-label={t("setup.buckets.delete")}
                onClick={() => handleDeleteBucket(selectedBucket)}
              >
                <Trash2 size={18} aria-hidden="true" />
              </button>
            </div>

            <form className="upload-form" onSubmit={handleUpload}>
              <input
                value={objectKey}
                onChange={(event) => setObjectKey(event.target.value)}
                placeholder={t("setup.objects.keyPlaceholder")}
                aria-label={t("setup.objects.key")}
              />
              <input
                type="file"
                onChange={(event) => setFile(event.target.files?.[0] ?? null)}
                aria-label={t("setup.objects.file")}
              />
              <Button
                type="submit"
                loading={uploading}
                disabled={!file}
                icon={<UploadCloud size={17} aria-hidden="true" />}
              >
                {t("setup.objects.upload")}
              </Button>
            </form>

            <label className="search-box">
              <Search size={17} aria-hidden="true" />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t("setup.objects.search")}
              />
            </label>

            {loadingObjects ? (
              <div className="admin-loading">{t("setup.common.loading")}</div>
            ) : objects.length === 0 ? (
              <EmptyState title={t("setup.objects.emptyTitle")} description={t("setup.objects.emptyDescription")} />
            ) : (
              <div className="object-table">
                <div className="object-table__head">
                  <span>{t("setup.objects.key")}</span>
                  <span>{t("setup.objects.size")}</span>
                  <span>{t("setup.objects.state")}</span>
                  <span>{t("setup.objects.createdAt")}</span>
                  <span />
                </div>
                {filteredObjects.map((object) => (
                  <div className="object-table__row" key={object.key}>
                    <span title={object.key}>{object.key}</span>
                    <span>{formatBytes(object.sizeBytes)}</span>
                    <span>{object.state}</span>
                    <span>{formatDate(object.createdAt)}</span>
                    <button
                      className="icon-button"
                      type="button"
                      title={t("setup.objects.delete")}
                      aria-label={t("setup.objects.delete")}
                      onClick={() => handleDeleteObject(object.key)}
                    >
                      <Trash2 size={17} aria-hidden="true" />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </>
        ) : (
          <EmptyState title={t("setup.buckets.emptyTitle")} description={t("setup.buckets.emptyDescription")} />
        )}
      </section>
    </div>
  );
}

function EmptyState({ title, description }: { title: string; description: string }) {
  return (
    <div className="empty-state">
      <strong>{title}</strong>
      <p>{description}</p>
    </div>
  );
}

function formatBytes(value: number): string {
  if (value === 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${(value / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(new Date(value));
}
