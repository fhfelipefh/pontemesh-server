import { ChangeEvent, FormEvent, useCallback, useEffect, useRef, useState } from "react";
import {
  ChevronLeft,
  ChevronRight,
  Download,
  FolderOpen,
  Plus,
  Search,
  Trash2,
  UploadCloud,
  X
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  BucketSummary,
  ObjectSummary,
  PaginatedResponse,
  createBucket,
  deleteBucket,
  deleteObject,
  listBuckets,
  listObjects,
  uploadObject
} from "../api/bucketsApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";

const BUCKET_PAGE_SIZE_OPTIONS = [10, 20, 50, 100];
const OBJECT_PAGE_SIZE_OPTIONS = [10, 20, 50, 100];

type ConfirmationState =
  | { kind: "bucket"; bucket: string }
  | { kind: "object"; bucket: string; objectKey: string }
  | null;

export function BucketsPage() {
  const { t } = useTranslation();
  const [bucketPage, setBucketPage] = useState<PaginatedResponse<BucketSummary>>(emptyPage(20));
  const [bucketQuery, setBucketQuery] = useState("");
  const [bucketSearch, setBucketSearch] = useState("");
  const [bucketPageNumber, setBucketPageNumber] = useState(1);
  const [bucketPageSize, setBucketPageSize] = useState(20);
  const [loadingBuckets, setLoadingBuckets] = useState(true);
  const [bucketError, setBucketError] = useState("");
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [bucketName, setBucketName] = useState("");
  const [submittingBucket, setSubmittingBucket] = useState(false);
  const [activeBucket, setActiveBucket] = useState<BucketSummary | null>(null);
  const [confirmation, setConfirmation] = useState<ConfirmationState>(null);
  const [objectRefreshNonce, setObjectRefreshNonce] = useState(0);
  const [drawerActionError, setDrawerActionError] = useState("");

  const refreshBuckets = useCallback(async () => {
    setLoadingBuckets(true);
    setBucketError("");
    try {
      const nextPage = await listBuckets({
        query: bucketSearch,
        page: bucketPageNumber,
        pageSize: bucketPageSize
      });
      setBucketPage(nextPage);
      if (activeBucket) {
        const refreshed = nextPage.items.find((bucket) => bucket.name === activeBucket.name);
        if (refreshed) {
          setActiveBucket(refreshed);
        }
      }
    } catch (loadError) {
      setBucketError(loadError instanceof Error ? loadError.message : t("setup.buckets.loadFailed"));
    } finally {
      setLoadingBuckets(false);
    }
  }, [activeBucket, bucketPageNumber, bucketPageSize, bucketSearch, t]);

  useEffect(() => {
    refreshBuckets();
  }, [refreshBuckets]);

  function handleBucketSearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBucketPageNumber(1);
    setBucketSearch(bucketQuery);
  }

  async function handleCreateBucket(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!bucketName.trim()) {
      return;
    }
    setSubmittingBucket(true);
    setBucketError("");
    try {
      const created = await createBucket(bucketName.trim());
      setBucketName("");
      setCreateModalOpen(false);
      setBucketPageNumber(1);
      setBucketSearch("");
      setBucketQuery("");
      setActiveBucket(created);
      await refreshBuckets();
    } catch (createError) {
      setBucketError(createError instanceof Error ? createError.message : t("setup.buckets.createFailed"));
    } finally {
      setSubmittingBucket(false);
    }
  }

  async function handleDeleteBucket(bucket: string) {
    setBucketError("");
    try {
      await deleteBucket(bucket);
      setConfirmation(null);
      if (activeBucket?.name === bucket) {
        setActiveBucket(null);
      }
      await refreshBuckets();
    } catch (deleteError) {
      setBucketError(deleteError instanceof Error ? deleteError.message : t("setup.buckets.deleteFailed"));
    }
  }

  const bucketSearchActive = bucketSearch.trim().length > 0;
  const hasBuckets = bucketPage.items.length > 0;

  return (
    <div className="buckets-page">
      <section className="admin-panel buckets-card">
        <div className="buckets-card__header">
          <div>
            <h1>{t("setup.buckets.title")}</h1>
            <p>{t("setup.buckets.description")}</p>
          </div>
          <Button
            className="buckets-create-button"
            type="button"
            icon={<Plus size={17} aria-hidden="true" />}
            onClick={() => setCreateModalOpen(true)}
          >
            {t("setup.buckets.create")}
          </Button>
        </div>

        <div className="buckets-toolbar">
          <form className="buckets-search" onSubmit={handleBucketSearch}>
            <label>
              <Search size={17} aria-hidden="true" />
              <input
                value={bucketQuery}
                onChange={(event) => setBucketQuery(event.target.value)}
                placeholder={t("setup.buckets.search")}
                aria-label={t("setup.buckets.search")}
              />
            </label>
            <Button type="submit">{t("setup.common.search")}</Button>
          </form>
          <PageSizeSelect
            label={t("setup.common.itemsPerPage")}
            value={bucketPageSize}
            options={BUCKET_PAGE_SIZE_OPTIONS}
            onChange={(nextPageSize) => {
              setBucketPageNumber(1);
              setBucketPageSize(nextPageSize);
            }}
          />
        </div>

        <ErrorMessage message={bucketError} />

        <div className="buckets-table-wrap">
          {loadingBuckets ? (
            <div className="admin-loading">{t("setup.buckets.loading")}</div>
          ) : !hasBuckets ? (
            <EmptyState
              title={bucketSearchActive ? t("setup.buckets.noResultsTitle") : t("setup.buckets.emptyTitle")}
              description={bucketSearchActive ? t("setup.buckets.noResultsDescription") : t("setup.buckets.emptyDescription")}
            />
          ) : (
            <div className="buckets-table" role="table" aria-label={t("setup.buckets.title")}>
              <div className="buckets-table__head" role="row">
                <span role="columnheader">{t("setup.buckets.name")}</span>
                <span role="columnheader">{t("setup.buckets.objectCount")}</span>
                <span role="columnheader">{t("setup.buckets.totalSize")}</span>
                <span role="columnheader">{t("setup.buckets.createdAt")}</span>
                <span role="columnheader">{t("setup.common.actions")}</span>
              </div>
              {bucketPage.items.map((bucket) => (
                <div className="buckets-table__row" role="row" key={bucket.name}>
                  <span role="cell" title={bucket.name}>{bucket.name}</span>
                  <span role="cell">{bucket.objectCount}</span>
                  <span role="cell">{formatBytes(bucket.totalBytes)}</span>
                  <span role="cell">{formatDate(bucket.createdAt)}</span>
                  <span className="buckets-table__actions" role="cell">
                    <button
                      className="table-action"
                      type="button"
                      onClick={() => {
                        setDrawerActionError("");
                        setActiveBucket(bucket);
                      }}
                    >
                      <FolderOpen size={16} aria-hidden="true" />
                      <span>{t("setup.buckets.open")}</span>
                    </button>
                    <button
                      className="table-action table-action--danger"
                      type="button"
                      onClick={() => setConfirmation({ kind: "bucket", bucket: bucket.name })}
                    >
                      <Trash2 size={16} aria-hidden="true" />
                      <span>{t("setup.buckets.delete")}</span>
                    </button>
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        <Pagination
          page={bucketPage.page}
          totalPages={bucketPage.totalPages}
          totalItems={bucketPage.totalItems}
          onPrevious={() => setBucketPageNumber((page) => Math.max(1, page - 1))}
          onNext={() => setBucketPageNumber((page) => Math.min(bucketPage.totalPages, page + 1))}
        />
      </section>

      {activeBucket ? (
        <BucketDrawer
          bucket={activeBucket}
          onClose={() => setActiveBucket(null)}
          onChanged={refreshBuckets}
          refreshNonce={objectRefreshNonce}
          externalError={drawerActionError}
          onConfirmDeleteObject={(objectKey) => setConfirmation({ kind: "object", bucket: activeBucket.name, objectKey })}
        />
      ) : null}

      {createModalOpen ? (
        <div className="settings-modal-backdrop" role="presentation">
          <form className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="create-bucket-title" onSubmit={handleCreateBucket}>
            <div className="settings-modal__header">
              <div>
                <h3 id="create-bucket-title">{t("setup.buckets.create")}</h3>
                <p>{t("setup.buckets.createDescription")}</p>
              </div>
              <button className="settings-modal__close" type="button" aria-label={t("setup.common.close")} onClick={() => setCreateModalOpen(false)}>
                <X size={18} aria-hidden="true" />
              </button>
            </div>
            <label className="settings-modal-field" htmlFor="bucket-name">
              <span>{t("setup.buckets.name")}</span>
              <input
                id="bucket-name"
                value={bucketName}
                onChange={(event) => setBucketName(event.target.value)}
                placeholder={t("setup.buckets.namePlaceholder")}
                autoFocus
              />
            </label>
            <div className="settings-modal__actions">
              <button className="settings-secondary-button" type="button" onClick={() => setCreateModalOpen(false)}>
                {t("setup.common.cancel")}
              </button>
              <Button className="settings-modal__primary" type="submit" loading={submittingBucket} disabled={!bucketName.trim()}>
                {t("setup.buckets.create")}
              </Button>
            </div>
          </form>
        </div>
      ) : null}

      {confirmation ? (
        <ConfirmDialog
          title={confirmation.kind === "bucket" ? t("setup.buckets.confirmDeleteTitle") : t("setup.objects.confirmDeleteTitle")}
          description={
            confirmation.kind === "bucket"
              ? t("setup.buckets.confirmDeleteDescription", { name: confirmation.bucket })
              : t("setup.objects.confirmDeleteDescription", { key: confirmation.objectKey })
          }
          onCancel={() => setConfirmation(null)}
          onConfirm={() => {
            if (confirmation.kind === "bucket") {
              void handleDeleteBucket(confirmation.bucket);
              return;
            }
            void deleteObjectAndRefresh(confirmation.bucket, confirmation.objectKey);
          }}
        />
      ) : null}
    </div>
  );

  async function deleteObjectAndRefresh(bucket: string, objectKey: string) {
    setBucketError("");
    try {
      await deleteObject(bucket, objectKey);
      setConfirmation(null);
      setDrawerActionError("");
      setObjectRefreshNonce((nonce) => nonce + 1);
      await refreshBuckets();
    } catch (deleteError) {
      setConfirmation(null);
      setDrawerActionError(deleteError instanceof Error ? deleteError.message : t("setup.objects.deleteFailed"));
    }
  }
}

type BucketDrawerProps = {
  bucket: BucketSummary;
  onClose: () => void;
  onChanged: () => Promise<void>;
  refreshNonce: number;
  externalError: string;
  onConfirmDeleteObject: (objectKey: string) => void;
};

function BucketDrawer({ bucket, onClose, onChanged, refreshNonce, externalError, onConfirmDeleteObject }: BucketDrawerProps) {
  const { t } = useTranslation();
  const [objectsPage, setObjectsPage] = useState<PaginatedResponse<ObjectSummary>>(emptyPage(20));
  const [objectQuery, setObjectQuery] = useState("");
  const [objectSearch, setObjectSearch] = useState("");
  const [objectPageNumber, setObjectPageNumber] = useState(1);
  const [objectPageSize, setObjectPageSize] = useState(20);
  const [loadingObjects, setLoadingObjects] = useState(true);
  const [objectError, setObjectError] = useState("");
  const [objectKey, setObjectKey] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const refreshObjects = useCallback(async () => {
    setLoadingObjects(true);
    setObjectError("");
    try {
      setObjectsPage(await listObjects(bucket.name, {
        query: objectSearch,
        page: objectPageNumber,
        pageSize: objectPageSize
      }));
    } catch (loadError) {
      setObjectError(loadError instanceof Error ? loadError.message : t("setup.objects.loadFailed"));
    } finally {
      setLoadingObjects(false);
    }
  }, [bucket.name, objectPageNumber, objectPageSize, objectSearch, t]);

  useEffect(() => {
    refreshObjects();
  }, [refreshObjects, refreshNonce]);

  async function handleUpload(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!file) {
      return;
    }
    setUploading(true);
    setObjectError("");
    try {
      await uploadObject(bucket.name, file, objectKey);
      setFile(null);
      setObjectKey("");
      event.currentTarget.reset();
      await refreshObjects();
      await onChanged();
    } catch (uploadError) {
      setObjectError(uploadError instanceof Error ? uploadError.message : t("setup.objects.uploadFailed"));
    } finally {
      setUploading(false);
    }
  }

  function handleObjectSearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setObjectPageNumber(1);
    setObjectSearch(objectQuery);
  }

  function handleFileChange(event: ChangeEvent<HTMLInputElement>) {
    const nextFile = event.target.files?.[0] ?? null;
    setFile(nextFile);
  }

  const objectSearchActive = objectSearch.trim().length > 0;
  const hasObjects = objectsPage.items.length > 0;

  return (
    <div className="bucket-drawer-backdrop" role="presentation">
      <aside className="bucket-drawer" role="dialog" aria-modal="true" aria-labelledby="bucket-drawer-title">
        <header className="bucket-drawer__header">
          <div>
            <h2 id="bucket-drawer-title">{bucket.name}</h2>
            <p>
              {bucket.objectCount} {t("setup.objects.count")} · {formatBytes(bucket.totalBytes)}
            </p>
          </div>
          <button className="settings-modal__close" type="button" aria-label={t("setup.common.close")} onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <form className="bucket-upload" onSubmit={handleUpload}>
          <label>
            <span>{t("setup.objects.key")}</span>
            <input
              value={objectKey}
              onChange={(event) => setObjectKey(event.target.value)}
              placeholder={t("setup.objects.keyPlaceholder")}
            />
          </label>
          <div className="bucket-file-picker">
            <span>{t("setup.objects.file")}</span>
            <input ref={fileInputRef} type="file" onChange={handleFileChange} />
            <button type="button" onClick={() => fileInputRef.current?.click()}>
              <UploadCloud size={17} aria-hidden="true" />
              <span>{t("setup.objects.chooseFile")}</span>
            </button>
            <small>{file?.name ?? t("setup.objects.noFileChosen")}</small>
          </div>
          <Button type="submit" loading={uploading} disabled={!file} icon={<UploadCloud size={17} aria-hidden="true" />}>
            {uploading ? t("setup.objects.uploading") : t("setup.objects.upload")}
          </Button>
        </form>

        <div className="bucket-drawer__tools">
          <form className="buckets-search" onSubmit={handleObjectSearch}>
            <label>
              <Search size={17} aria-hidden="true" />
              <input
                value={objectQuery}
                onChange={(event) => setObjectQuery(event.target.value)}
                placeholder={t("setup.objects.search")}
                aria-label={t("setup.objects.search")}
              />
            </label>
            <Button type="submit">{t("setup.common.search")}</Button>
          </form>
          <PageSizeSelect
            label={t("setup.common.itemsPerPage")}
            value={objectPageSize}
            options={OBJECT_PAGE_SIZE_OPTIONS}
            onChange={(nextPageSize) => {
              setObjectPageNumber(1);
              setObjectPageSize(nextPageSize);
            }}
          />
        </div>

        <ErrorMessage message={objectError || externalError} />

        <div className="bucket-drawer__table-wrap">
          {loadingObjects ? (
            <div className="admin-loading">{t("setup.objects.loading")}</div>
          ) : !hasObjects ? (
            <EmptyState
              title={objectSearchActive ? t("setup.objects.noResultsTitle") : t("setup.objects.emptyTitle")}
              description={objectSearchActive ? t("setup.objects.noResultsDescription") : t("setup.objects.emptyDescription")}
            />
          ) : (
            <div className="object-table object-table--drawer">
              <div className="object-table__head">
                <span>{t("setup.objects.key")}</span>
                <span>{t("setup.objects.size")}</span>
                <span>{t("setup.objects.contentType")}</span>
                <span>{t("setup.objects.updatedAt")}</span>
                <span>{t("setup.common.actions")}</span>
              </div>
              {objectsPage.items.map((object) => (
                <div className="object-table__row" key={object.key}>
                  <span title={object.key}>{object.key}</span>
                  <span>{formatBytes(object.sizeBytes)}</span>
                  <span title={object.contentType}>{object.contentType}</span>
                  <span>{formatDate(object.updatedAt ?? object.createdAt)}</span>
                  <span className="buckets-table__actions">
                    <a
                      className="table-action"
                      href={`/api/admin/buckets/${encodeURIComponent(bucket.name)}/objects/${encodePathKey(object.key)}`}
                      download
                    >
                      <Download size={16} aria-hidden="true" />
                      <span>{t("setup.objects.download")}</span>
                    </a>
                    <button
                      className="table-action table-action--danger"
                      type="button"
                      onClick={() => onConfirmDeleteObject(object.key)}
                    >
                      <Trash2 size={16} aria-hidden="true" />
                      <span>{t("setup.objects.delete")}</span>
                    </button>
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        <Pagination
          page={objectsPage.page}
          totalPages={objectsPage.totalPages}
          totalItems={objectsPage.totalItems}
          onPrevious={() => setObjectPageNumber((page) => Math.max(1, page - 1))}
          onNext={() => setObjectPageNumber((page) => Math.min(objectsPage.totalPages, page + 1))}
        />
      </aside>
    </div>
  );
}

function ConfirmDialog({
  title,
  description,
  onCancel,
  onConfirm
}: {
  title: string;
  description: string;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="settings-modal-backdrop" role="presentation">
      <div className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
        <div className="settings-modal__header">
          <div>
            <h3 id="confirm-title">{title}</h3>
            <p>{description}</p>
          </div>
          <button className="settings-modal__close" type="button" aria-label={t("setup.common.close")} onClick={onCancel}>
            <X size={18} aria-hidden="true" />
          </button>
        </div>
        <div className="settings-modal__actions">
          <button className="settings-secondary-button" type="button" onClick={onCancel}>
            {t("setup.common.cancel")}
          </button>
          <Button className="settings-modal__primary button--danger" type="button" onClick={onConfirm}>
            {t("setup.common.confirm")}
          </Button>
        </div>
      </div>
    </div>
  );
}

function PageSizeSelect({
  label,
  value,
  options,
  onChange
}: {
  label: string;
  value: number;
  options: number[];
  onChange: (value: number) => void;
}) {
  return (
    <label className="page-size-select">
      <span>{label}</span>
      <select value={value} onChange={(event) => onChange(Number(event.target.value))}>
        {options.map((option) => (
          <option value={option} key={option}>
            {option}
          </option>
        ))}
      </select>
    </label>
  );
}

function Pagination({
  page,
  totalPages,
  totalItems,
  onPrevious,
  onNext
}: {
  page: number;
  totalPages: number;
  totalItems: number;
  onPrevious: () => void;
  onNext: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="buckets-pagination">
      <span>{t("setup.common.totalItems", { count: totalItems })}</span>
      <div className="buckets-pagination__actions">
        <button type="button" disabled={page <= 1} onClick={onPrevious}>
          <ChevronLeft size={16} aria-hidden="true" />
          <span>{t("setup.common.previous")}</span>
        </button>
        <strong>{t("setup.common.pageIndicator", { page, totalPages })}</strong>
        <button type="button" disabled={page >= totalPages} onClick={onNext}>
          <span>{t("setup.common.next")}</span>
          <ChevronRight size={16} aria-hidden="true" />
        </button>
      </div>
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

function emptyPage<T>(pageSize: number): PaginatedResponse<T> {
  return {
    items: [],
    page: 1,
    pageSize,
    totalItems: 0,
    totalPages: 1
  };
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

function encodePathKey(key: string): string {
  return key.split("/").map(encodeURIComponent).join("/");
}
