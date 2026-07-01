import { FormEvent, useCallback, useEffect, useState } from "react";
import {
  FolderOpen,
  Plus,
  Search,
  Trash2,
  X
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  BucketPolicy,
  BucketSummary,
  PaginatedResponse,
  createBucket,
  deleteBucket,
  deleteObject,
  getBucketPolicy,
  listBuckets,
  updateBucketPolicy
} from "../api/bucketsApi";
import { Button } from "../components/Button";
import { ConfirmDialog, EmptyState, PageSizeSelect, Pagination } from "../components/AdminListControls";
import { ErrorMessage } from "../components/ErrorMessage";
import { ObjectManager } from "../components/ObjectManager";
import { emptyPage, formatBytes, formatDate } from "../utils/adminFormat";

const BUCKET_PAGE_SIZE_OPTIONS = [10, 20, 50, 100];

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
      setActiveBucket((currentBucket) => {
        if (!currentBucket) {
          return currentBucket;
        }

        return nextPage.items.find((bucket) => bucket.name === currentBucket.name) ?? currentBucket;
      });
    } catch (loadError) {
      setBucketError(loadError instanceof Error ? loadError.message : t("setup.buckets.loadFailed"));
    } finally {
      setLoadingBuckets(false);
    }
  }, [bucketPageNumber, bucketPageSize, bucketSearch, t]);

  useEffect(() => {
    refreshBuckets();
  }, [refreshBuckets]);

  useEffect(() => {
    if (!createModalOpen) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setCreateModalOpen(false);
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [createModalOpen]);

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
          </div>
          <Button
            className="buckets-create-button"
            data-testid="create-bucket-button"
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
                data-testid="bucket-search-input"
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
            <div className="buckets-table" data-testid="bucket-list" role="table" aria-label={t("setup.buckets.title")}>
              <div className="buckets-table__head" role="row">
                <span role="columnheader">{t("setup.buckets.name")}</span>
                <span role="columnheader">{t("setup.buckets.objectCount")}</span>
                <span role="columnheader">{t("setup.buckets.totalSize")}</span>
                <span role="columnheader">{t("setup.buckets.createdAt")}</span>
                <span role="columnheader">{t("setup.common.actions")}</span>
              </div>
              {bucketPage.items.map((bucket) => (
                <div className="buckets-table__row" data-testid="bucket-row" role="row" key={bucket.name}>
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
        <div className="settings-modal-backdrop" data-testid="modal-backdrop" role="presentation">
          <form
            className="settings-modal"
            data-testid="create-bucket-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="create-bucket-title"
            onSubmit={handleCreateBucket}
          >
            <div className="settings-modal__header">
              <div>
                <h3 id="create-bucket-title">{t("setup.buckets.create")}</h3>
                <p>{t("setup.buckets.createDescription")}</p>
              </div>
              <button
                className="settings-modal__close"
                data-testid="create-bucket-close"
                type="button"
                aria-label={t("setup.common.close")}
                onClick={() => setCreateModalOpen(false)}
              >
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
  const [policy, setPolicy] = useState<BucketPolicy | null>(null);
  const [policyError, setPolicyError] = useState("");
  const [policySaving, setPolicySaving] = useState(false);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  useEffect(() => {
    let active = true;
    setPolicy(null);
    setPolicyError("");
    getBucketPolicy(bucket.name)
      .then((nextPolicy) => {
        if (active) {
          setPolicy(nextPolicy);
        }
      })
      .catch((loadError) => {
        if (active) {
          setPolicyError(loadError instanceof Error ? loadError.message : t("setup.buckets.policyLoadFailed"));
        }
      });
    return () => {
      active = false;
    };
  }, [bucket.name, t]);

  async function savePolicy(nextPolicy: BucketPolicy) {
    setPolicySaving(true);
    setPolicyError("");
    try {
      const saved = await updateBucketPolicy(bucket.name, {
        accessPackageTtlSeconds: nextPolicy.accessPackageTtlSeconds,
        fragmentSizeBytes: nextPolicy.fragmentSizeBytes,
        allowReplicaEdge: nextPolicy.allowReplicaEdge,
        allowPeerSharing: nextPolicy.allowPeerSharing,
        sourceSelectionStrategy: nextPolicy.sourceSelectionStrategy,
        fragmentPriorityStrategy: nextPolicy.fragmentPriorityStrategy,
        failureThreshold: nextPolicy.failureThreshold,
        fallbackMode: nextPolicy.fallbackMode
      });
      setPolicy(saved);
    } catch (saveError) {
      setPolicyError(saveError instanceof Error ? saveError.message : t("setup.buckets.policySaveFailed"));
    } finally {
      setPolicySaving(false);
    }
  }

  return (
    <div className="bucket-drawer-backdrop" data-testid="modal-backdrop" role="presentation">
      <aside className="bucket-drawer" data-testid="bucket-details-dialog" role="dialog" aria-modal="true" aria-labelledby="bucket-drawer-title">
        <header className="bucket-drawer__header">
          <div>
            <h2 id="bucket-drawer-title">{bucket.name}</h2>
            <p>
              {bucket.objectCount} {t("setup.objects.count")} · {formatBytes(bucket.totalBytes)}
            </p>
          </div>
          <button
            className="settings-modal__close"
            data-testid="bucket-details-close"
            type="button"
            aria-label={t("setup.common.close")}
            onClick={onClose}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <section className="bucket-policy-panel">
          <div className="bucket-policy-panel__header">
            <h3>{t("setup.buckets.policyTitle")}</h3>
            {policy ? (
              <Button
                type="button"
                loading={policySaving}
                onClick={() => void savePolicy(policy)}
              >
                {t("setup.common.save")}
              </Button>
            ) : null}
          </div>
          <ErrorMessage message={policyError} />
          {!policy && !policyError ? (
            <div className="admin-loading">{t("setup.common.loading")}</div>
          ) : policy ? (
            <div className="bucket-policy-grid">
              <label>
                <span>{t("setup.buckets.accessPackageTtl")}</span>
                <input
                  type="number"
                  min={60}
                  max={3600}
                  value={policy.accessPackageTtlSeconds}
                  onChange={(event) => setPolicy({ ...policy, accessPackageTtlSeconds: Number(event.target.value) })}
                />
              </label>
              <label>
                <span>{t("setup.buckets.fragmentSize")}</span>
                <input
                  type="number"
                  min={1024}
                  max={134217728}
                  value={policy.fragmentSizeBytes}
                  onChange={(event) => setPolicy({ ...policy, fragmentSizeBytes: Number(event.target.value) })}
                />
              </label>
              <label>
                <span>{t("setup.buckets.sourceSelection")}</span>
                <select
                  value={policy.sourceSelectionStrategy}
                  onChange={(event) => setPolicy({ ...policy, sourceSelectionStrategy: event.target.value })}
                >
                  <option value="ORIGIN_REPLICA_EDGE">{t("setup.buckets.sourceOriginReplica")}</option>
                  <option value="ORIGIN_ONLY">{t("setup.buckets.sourceOriginOnly")}</option>
                  <option value="REPLICA_EDGE_FIRST">{t("setup.buckets.sourceReplicaFirst")}</option>
                  <option value="PEER_FIRST">{t("setup.buckets.sourcePeerFirst")}</option>
                </select>
              </label>
              <label>
                <span>{t("setup.buckets.fragmentPriority")}</span>
                <select
                  value={policy.fragmentPriorityStrategy}
                  onChange={(event) => setPolicy({ ...policy, fragmentPriorityStrategy: event.target.value })}
                >
                  <option value="MANIFEST_ORDER">{t("setup.buckets.priorityManifest")}</option>
                  <option value="INITIAL_FIRST">{t("setup.buckets.priorityInitial")}</option>
                  <option value="RAREST_FIRST">{t("setup.buckets.priorityRarest")}</option>
                </select>
              </label>
              <label>
                <span>{t("setup.buckets.failureThreshold")}</span>
                <input
                  type="number"
                  min={1}
                  max={20}
                  value={policy.failureThreshold}
                  onChange={(event) => setPolicy({ ...policy, failureThreshold: Number(event.target.value) })}
                />
              </label>
              <label>
                <span>{t("setup.buckets.fallbackMode")}</span>
                <select
                  value={policy.fallbackMode}
                  onChange={(event) => setPolicy({ ...policy, fallbackMode: event.target.value })}
                >
                  <option value="ORIGIN_RANGE">{t("setup.buckets.fallbackRange")}</option>
                  <option value="ORIGIN_FULL_OBJECT">{t("setup.buckets.fallbackFull")}</option>
                  <option value="DISABLED">{t("setup.buckets.fallbackDisabled")}</option>
                </select>
              </label>
              <label className="bucket-policy-toggle">
                <input
                  type="checkbox"
                  checked={policy.allowReplicaEdge}
                  onChange={(event) => setPolicy({ ...policy, allowReplicaEdge: event.target.checked })}
                />
                <span>{t("setup.buckets.allowReplicaEdge")}</span>
              </label>
              <label className="bucket-policy-toggle">
                <input
                  type="checkbox"
                  checked={policy.allowPeerSharing}
                  onChange={(event) => setPolicy({ ...policy, allowPeerSharing: event.target.checked })}
                />
                <span>{t("setup.buckets.allowPeerSharing")}</span>
              </label>
            </div>
          ) : null}
        </section>

        <ObjectManager
          bucketName={bucket.name}
          refreshNonce={refreshNonce}
          externalError={externalError}
          onChanged={onChanged}
          onConfirmDeleteObject={onConfirmDeleteObject}
        />
      </aside>
    </div>
  );
}
