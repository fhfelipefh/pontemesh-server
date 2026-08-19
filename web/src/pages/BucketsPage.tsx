import { FormEvent, useCallback, useEffect, useRef, useState } from "react";
import {
  FolderOpen,
  Plus,
  Save,
  Search,
  Settings2,
  Trash2,
  X
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { HttpError } from "../api/http";
import {
  BucketPolicyDefaultsInput,
  BucketPolicy,
  BucketSummary,
  PaginatedResponse,
  bulkUpdateBucketPolicies,
  createBucket,
  deleteBucket,
  deleteObject,
  getBucketPolicy,
  getBucketPolicyDefaults,
  listBuckets,
  updateBucketPolicy,
  updateBucketPolicyDefaults
} from "../api/bucketsApi";
import { Button } from "../components/Button";
import { ConfirmDialog, EmptyState, PageSizeSelect, Pagination } from "../components/AdminListControls";
import { ErrorMessage } from "../components/ErrorMessage";
import { CheckboxField, CheckboxGrid, FormField, FormGrid, FormSection, FormSectionHeader } from "../components/layout";
import { ObjectManager } from "../components/ObjectManager";
import { emptyPage, formatBytes, formatDate } from "../utils/adminFormat";

const BUCKET_PAGE_SIZE_OPTIONS = [10, 20, 50, 100];
const BUCKET_AUTO_REFRESH_MS = 5000;

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
  const [selectedBuckets, setSelectedBuckets] = useState<Set<string>>(new Set());
  const [policyManagerOpen, setPolicyManagerOpen] = useState(false);
  const bucketRefreshInFlight = useRef(false);

  const refreshBuckets = useCallback(async (silent = false) => {
    if (bucketRefreshInFlight.current) {
      return;
    }
    bucketRefreshInFlight.current = true;
    if (!silent) {
      setLoadingBuckets(true);
      setBucketError("");
    }
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
      if (!silent) {
        setLoadingBuckets(false);
      }
      bucketRefreshInFlight.current = false;
    }
  }, [bucketPageNumber, bucketPageSize, bucketSearch, t]);

  useEffect(() => {
    refreshBuckets();
  }, [refreshBuckets]);

  useEffect(() => {
    const interval = window.setInterval(() => {
      if (document.visibilityState === "visible") {
        void refreshBuckets(true);
      }
    }, BUCKET_AUTO_REFRESH_MS);
    return () => window.clearInterval(interval);
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
      if (createError instanceof HttpError && createError.status === 409) {
        setBucketError(t("setup.buckets.createConflict"));
      } else {
        setBucketError(createError instanceof Error ? createError.message : t("setup.buckets.createFailed"));
      }
    } finally {
      setSubmittingBucket(false);
    }
  }

  async function handleDeleteBucket(bucket: string) {
    setBucketError("");
    try {
      await deleteBucket(bucket);
      setSelectedBuckets((current) => {
        const next = new Set(current);
        next.delete(bucket);
        return next;
      });
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
  const visibleBucketNames = bucketPage.items.map((bucket) => bucket.name);
  const allVisibleSelected = visibleBucketNames.length > 0 && visibleBucketNames.every((name) => selectedBuckets.has(name));

  return (
    <div className="buckets-page">
      <section className="admin-panel buckets-card">
        <div className="buckets-card__header">
          <div>
            <h1>{t("setup.buckets.title")}</h1>
          </div>
          <div className="buckets-header-actions">
            <Button
              className="buckets-policy-button"
              data-testid="bucket-policy-manager-button"
              type="button"
              icon={<Settings2 size={17} aria-hidden="true" />}
              onClick={() => setPolicyManagerOpen(true)}
            >
              {t("setup.buckets.managePolicies")}
            </Button>
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
            <Button type="submit" icon={<Search size={17} aria-hidden="true" />}>{t("setup.common.search")}</Button>
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
            />
          ) : (
            <div className="buckets-table" data-testid="bucket-list" role="table" aria-label={t("setup.buckets.title")}>
              <div className="buckets-table__head" role="row">
                <span role="columnheader" className="buckets-table__select">
                  <input
                    type="checkbox"
                    aria-label={t("setup.buckets.selectVisible")}
                    checked={allVisibleSelected}
                    onChange={(event) => {
                      setSelectedBuckets((current) => {
                        const next = new Set(current);
                        for (const name of visibleBucketNames) {
                          if (event.target.checked) next.add(name);
                          else next.delete(name);
                        }
                        return next;
                      });
                    }}
                  />
                </span>
                <span role="columnheader">{t("setup.buckets.name")}</span>
                <span role="columnheader">{t("setup.buckets.owner")}</span>
                <span role="columnheader">{t("setup.buckets.objectCount")}</span>
                <span role="columnheader">{t("setup.buckets.totalSize")}</span>
                <span role="columnheader">{t("setup.buckets.createdAt")}</span>
                <span role="columnheader">{t("setup.common.actions")}</span>
              </div>
              {bucketPage.items.map((bucket) => (
                <div className="buckets-table__row" data-testid="bucket-row" role="row" key={bucket.name}>
                  <span role="cell" className="buckets-table__select">
                    <input
                      type="checkbox"
                      aria-label={t("setup.buckets.selectBucket", { name: bucket.name })}
                      checked={selectedBuckets.has(bucket.name)}
                      onChange={(event) => {
                        setSelectedBuckets((current) => {
                          const next = new Set(current);
                          if (event.target.checked) next.add(bucket.name);
                          else next.delete(bucket.name);
                          return next;
                        });
                      }}
                    />
                  </span>
                  <span role="cell" title={bucket.name}>{bucket.name}</span>
                  <span role="cell" title={bucket.ownerUsername} className="buckets-table__owner">{bucket.ownerUsername}</span>
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

      {policyManagerOpen ? (
        <BucketPolicyManager
          selectedBuckets={[...selectedBuckets].sort()}
          onClose={() => setPolicyManagerOpen(false)}
          onApplied={async () => {
            setPolicyManagerOpen(false);
            await refreshBuckets();
          }}
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
                <X size={16} aria-hidden="true" />
                {t("setup.common.cancel")}
              </button>
              <Button className="settings-modal__primary" type="submit" loading={submittingBucket} disabled={!bucketName.trim()} icon={<Plus size={17} aria-hidden="true" />}>
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

type BucketPolicyManagerProps = {
  selectedBuckets: string[];
  onClose: () => void;
  onApplied: () => Promise<void>;
};

function BucketPolicyManager({ selectedBuckets, onClose, onApplied }: BucketPolicyManagerProps) {
  const { t } = useTranslation();
  const [policy, setPolicy] = useState<BucketPolicyDefaultsInput | null>(null);
  const [scope, setScope] = useState<"selected" | "all">(selectedBuckets.length > 0 ? "selected" : "all");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState<"defaults" | "buckets" | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  useEffect(() => {
    let active = true;
    getBucketPolicyDefaults()
      .then((defaults) => {
        if (active) setPolicy(defaults);
      })
      .catch((loadError) => {
        if (active) setError(loadError instanceof Error ? loadError.message : t("setup.buckets.defaultsLoadFailed"));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [t]);

  async function saveDefaults() {
    if (!policy) return;
    setSubmitting("defaults");
    setError("");
    setNotice("");
    try {
      const saved = await updateBucketPolicyDefaults(policy);
      setPolicy(saved);
      setNotice(t("setup.buckets.defaultsSaved"));
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : t("setup.buckets.defaultsSaveFailed"));
    } finally {
      setSubmitting(null);
    }
  }

  async function applyToBuckets(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!policy || (scope === "selected" && selectedBuckets.length === 0)) return;
    setSubmitting("buckets");
    setError("");
    setNotice("");
    try {
      await bulkUpdateBucketPolicies(
        { allBuckets: scope === "all", bucketNames: scope === "selected" ? selectedBuckets : [] },
        policy
      );
      await onApplied();
    } catch (applyError) {
      setError(applyError instanceof Error ? applyError.message : t("setup.buckets.bulkSaveFailed"));
    } finally {
      setSubmitting(null);
    }
  }

  return (
    <div className="settings-modal-backdrop" data-testid="modal-backdrop" role="presentation">
      <form
        className="settings-modal bucket-policy-manager"
        data-testid="bucket-policy-manager-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="bucket-policy-manager-title"
        onSubmit={applyToBuckets}
      >
        <div className="settings-modal__header">
          <div>
            <h3 id="bucket-policy-manager-title">{t("setup.buckets.managePolicies")}</h3>
          </div>
          <button className="settings-modal__close" type="button" aria-label={t("setup.common.close")} onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </div>

        {loading ? <div className="admin-loading">{t("setup.buckets.loadingDefaults")}</div> : null}
        <ErrorMessage message={error} />
        {notice ? <p className="settings-success" role="status">{notice}</p> : null}

        {policy ? (
          <>
            <section className="bucket-policy-manager__section">
              <h4>{t("setup.buckets.defaultsTitle")}</h4>
              <p>{t("setup.buckets.defaultsDescription")}</p>
              <HybridPolicyFields policy={policy} onChange={setPolicy} idPrefix="bucket-defaults" />
            </section>

            <section className="bucket-policy-manager__section">
              <h4>{t("setup.buckets.applyTitle")}</h4>
              <div className="bucket-policy-manager__scope">
                <label>
                  <input
                    type="radio"
                    name="bucket-policy-scope"
                    value="selected"
                    checked={scope === "selected"}
                    disabled={selectedBuckets.length === 0}
                    onChange={() => setScope("selected")}
                  />
                  <span>{t("setup.buckets.applySelected", { count: selectedBuckets.length })}</span>
                </label>
                <label>
                  <input
                    type="radio"
                    name="bucket-policy-scope"
                    value="all"
                    checked={scope === "all"}
                    onChange={() => setScope("all")}
                  />
                  <span>{t("setup.buckets.applyAll")}</span>
                </label>
              </div>
            </section>

            <div className="settings-modal__actions bucket-policy-manager__actions">
              <button className="settings-secondary-button" type="button" onClick={onClose}>
                <X size={16} aria-hidden="true" />
                {t("setup.common.cancel")}
              </button>
              <Button
                type="button"
                loading={submitting === "defaults"}
                disabled={submitting !== null}
                icon={<Save size={17} aria-hidden="true" />}
                onClick={() => void saveDefaults()}
              >
                {t("setup.buckets.saveDefaults")}
              </Button>
              <Button
                type="submit"
                loading={submitting === "buckets"}
                disabled={submitting !== null || (scope === "selected" && selectedBuckets.length === 0)}
                icon={<Settings2 size={17} aria-hidden="true" />}
              >
                {t("setup.buckets.applyPolicy")}
              </Button>
            </div>
          </>
        ) : null}
      </form>
    </div>
  );
}

type HybridPolicyFieldsProps = {
  policy: BucketPolicyDefaultsInput;
  onChange: (policy: BucketPolicyDefaultsInput) => void;
  idPrefix: string;
};

function HybridPolicyFields({ policy, onChange, idPrefix }: HybridPolicyFieldsProps) {
  const { t } = useTranslation();
  return (
    <>
      <FormGrid columns={4}>
        <FormField label={t("setup.buckets.accessPackageTtl")} htmlFor={`${idPrefix}-ttl`}>
          <input id={`${idPrefix}-ttl`} type="number" min={60} max={3600} required value={policy.accessPackageTtlSeconds} onChange={(event) => onChange({ ...policy, accessPackageTtlSeconds: Number(event.target.value) })} />
        </FormField>
        <FormField label={t("setup.buckets.fragmentSize")} htmlFor={`${idPrefix}-fragment-size`}>
          <input id={`${idPrefix}-fragment-size`} type="number" min={1024} max={134217728} required value={policy.fragmentSizeBytes} onChange={(event) => onChange({ ...policy, fragmentSizeBytes: Number(event.target.value) })} />
        </FormField>
        <FormField label={t("setup.buckets.sourceSelection")} htmlFor={`${idPrefix}-source`}>
          <select id={`${idPrefix}-source`} value={policy.sourceSelectionStrategy} onChange={(event) => onChange({ ...policy, sourceSelectionStrategy: event.target.value })}>
            <option value="ORIGIN_REPLICA_EDGE">{t("setup.buckets.sourceOriginReplica")}</option>
            <option value="ORIGIN_ONLY">{t("setup.buckets.sourceOriginOnly")}</option>
            <option value="REPLICA_EDGE_FIRST">{t("setup.buckets.sourceReplicaFirst")}</option>
            <option value="PEER_FIRST">{t("setup.buckets.sourcePeerFirst")}</option>
          </select>
        </FormField>
        <FormField label={t("setup.buckets.fragmentPriority")} htmlFor={`${idPrefix}-priority`}>
          <select id={`${idPrefix}-priority`} value={policy.fragmentPriorityStrategy} onChange={(event) => onChange({ ...policy, fragmentPriorityStrategy: event.target.value })}>
            <option value="MANIFEST_ORDER">{t("setup.buckets.priorityManifest")}</option>
            <option value="INITIAL_FIRST">{t("setup.buckets.priorityInitial")}</option>
            <option value="RAREST_FIRST">{t("setup.buckets.priorityRarest")}</option>
          </select>
        </FormField>
        <FormField label={t("setup.buckets.failureThreshold")} htmlFor={`${idPrefix}-failure-threshold`}>
          <input id={`${idPrefix}-failure-threshold`} type="number" min={1} max={20} required value={policy.failureThreshold} onChange={(event) => onChange({ ...policy, failureThreshold: Number(event.target.value) })} />
        </FormField>
        <FormField label={t("setup.buckets.fallbackMode")} htmlFor={`${idPrefix}-fallback`}>
          <select id={`${idPrefix}-fallback`} value={policy.fallbackMode} onChange={(event) => onChange({ ...policy, fallbackMode: event.target.value })}>
            <option value="ORIGIN_RANGE">{t("setup.buckets.fallbackRange")}</option>
            <option value="ORIGIN_FULL_OBJECT">{t("setup.buckets.fallbackFull")}</option>
            <option value="DISABLED">{t("setup.buckets.fallbackDisabled")}</option>
          </select>
        </FormField>
      </FormGrid>
      <CheckboxGrid columns={2}>
        <CheckboxField label={t("setup.buckets.allowReplicaEdge")}>
          <input type="checkbox" checked={policy.allowReplicaEdge} onChange={(event) => onChange({ ...policy, allowReplicaEdge: event.target.checked })} />
        </CheckboxField>
        <CheckboxField label={t("setup.buckets.allowPeerSharing")}>
          <input type="checkbox" checked={policy.allowPeerSharing} onChange={(event) => onChange({ ...policy, allowPeerSharing: event.target.checked })} />
        </CheckboxField>
      </CheckboxGrid>
    </>
  );
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
  const [s3Json, setS3Json] = useState({
    lifecycle: "",
    resourcePolicy: "",
    notifications: ""
  });
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
          setS3Json(policyJsonState(nextPolicy));
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
    const parsedJson = parseAdvancedS3Json(s3Json, t);
    if ("error" in parsedJson) {
      setPolicySaving(false);
      setPolicyError(parsedJson.error);
      return;
    }
    try {
      const saved = await updateBucketPolicy(bucket.name, {
        accessPackageTtlSeconds: nextPolicy.accessPackageTtlSeconds,
        fragmentSizeBytes: nextPolicy.fragmentSizeBytes,
        allowReplicaEdge: nextPolicy.allowReplicaEdge,
        allowPeerSharing: nextPolicy.allowPeerSharing,
        sourceSelectionStrategy: nextPolicy.sourceSelectionStrategy,
        fragmentPriorityStrategy: nextPolicy.fragmentPriorityStrategy,
        failureThreshold: nextPolicy.failureThreshold,
        fallbackMode: nextPolicy.fallbackMode,
        s3ListDefaultMaxKeys: nextPolicy.s3ListDefaultMaxKeys,
        s3ListMaxKeysLimit: nextPolicy.s3ListMaxKeysLimit,
        s3ListAllowDelimiter: nextPolicy.s3ListAllowDelimiter,
        s3VersioningEnabled: nextPolicy.s3VersioningEnabled,
        s3ObjectTaggingEnabled: nextPolicy.s3ObjectTaggingEnabled,
        s3ChecksumAlgorithm: nextPolicy.s3ChecksumAlgorithm,
        s3MultipartAbortDays: nextPolicy.s3MultipartAbortDays,
        s3DefaultEncryptionAlgorithm: nextPolicy.s3DefaultEncryptionAlgorithm,
        s3DefaultEncryptionKeyId: emptyStringToNull(nextPolicy.s3DefaultEncryptionKeyId),
        s3ObjectLockEnabled: nextPolicy.s3ObjectLockEnabled,
        s3ObjectLockDefaultMode: nextPolicy.s3ObjectLockDefaultMode,
        s3ObjectLockDefaultRetainDays: nextPolicy.s3ObjectLockDefaultRetainDays,
        s3LifecycleRules: parsedJson.lifecycle,
        s3ResourcePolicy: parsedJson.resourcePolicy,
        s3EventNotifications: parsedJson.notifications
      });
      setPolicy(saved);
      setS3Json(policyJsonState(saved));
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

        <FormSection data-testid="hybrid-policy-section">
          <FormSectionHeader
            title={t("setup.buckets.policyTitle")}
            actions={policy ? (
              <Button
                data-testid="hybrid-policy-save-button"
                type="button"
                loading={policySaving}
                icon={<Save size={17} aria-hidden="true" />}
                onClick={() => void savePolicy(policy)}
              >
                {t("setup.common.save")}
              </Button>
            ) : null}
          />
          <ErrorMessage message={policyError} />
          {!policy && !policyError ? (
            <div className="admin-loading">{t("setup.common.loading")}</div>
          ) : policy ? (
            <>
              <FormGrid columns={4}>
                <FormField label={t("setup.buckets.accessPackageTtl")} htmlFor="bucket-policy-access-package-ttl">
                <input
                  id="bucket-policy-access-package-ttl"
                  type="number"
                  min={60}
                  max={3600}
                  value={policy.accessPackageTtlSeconds}
                  onChange={(event) => setPolicy({ ...policy, accessPackageTtlSeconds: Number(event.target.value) })}
                />
                </FormField>
                <FormField label={t("setup.buckets.fragmentSize")} htmlFor="bucket-policy-fragment-size">
                <input
                  id="bucket-policy-fragment-size"
                  type="number"
                  min={1024}
                  max={134217728}
                  value={policy.fragmentSizeBytes}
                  onChange={(event) => setPolicy({ ...policy, fragmentSizeBytes: Number(event.target.value) })}
                />
                </FormField>
                <FormField label={t("setup.buckets.sourceSelection")} htmlFor="bucket-policy-source-selection">
                <select
                  id="bucket-policy-source-selection"
                  value={policy.sourceSelectionStrategy}
                  onChange={(event) => setPolicy({ ...policy, sourceSelectionStrategy: event.target.value })}
                >
                  <option value="ORIGIN_REPLICA_EDGE">{t("setup.buckets.sourceOriginReplica")}</option>
                  <option value="ORIGIN_ONLY">{t("setup.buckets.sourceOriginOnly")}</option>
                  <option value="REPLICA_EDGE_FIRST">{t("setup.buckets.sourceReplicaFirst")}</option>
                  <option value="PEER_FIRST">{t("setup.buckets.sourcePeerFirst")}</option>
                </select>
                </FormField>
                <FormField label={t("setup.buckets.fragmentPriority")} htmlFor="bucket-policy-fragment-priority">
                <select
                  id="bucket-policy-fragment-priority"
                  value={policy.fragmentPriorityStrategy}
                  onChange={(event) => setPolicy({ ...policy, fragmentPriorityStrategy: event.target.value })}
                >
                  <option value="MANIFEST_ORDER">{t("setup.buckets.priorityManifest")}</option>
                  <option value="INITIAL_FIRST">{t("setup.buckets.priorityInitial")}</option>
                  <option value="RAREST_FIRST">{t("setup.buckets.priorityRarest")}</option>
                </select>
                </FormField>
                <FormField label={t("setup.buckets.failureThreshold")} htmlFor="bucket-policy-failure-threshold">
                <input
                  id="bucket-policy-failure-threshold"
                  type="number"
                  min={1}
                  max={20}
                  value={policy.failureThreshold}
                  onChange={(event) => setPolicy({ ...policy, failureThreshold: Number(event.target.value) })}
                />
                </FormField>
                <FormField label={t("setup.buckets.fallbackMode")} htmlFor="bucket-policy-fallback-mode">
                <select
                  id="bucket-policy-fallback-mode"
                  value={policy.fallbackMode}
                  onChange={(event) => setPolicy({ ...policy, fallbackMode: event.target.value })}
                >
                  <option value="ORIGIN_RANGE">{t("setup.buckets.fallbackRange")}</option>
                  <option value="ORIGIN_FULL_OBJECT">{t("setup.buckets.fallbackFull")}</option>
                  <option value="DISABLED">{t("setup.buckets.fallbackDisabled")}</option>
                </select>
                </FormField>
                <FormField label={t("setup.buckets.s3ListDefaultMaxKeys")} htmlFor="bucket-policy-s3-default-max-keys">
                <input
                  id="bucket-policy-s3-default-max-keys"
                  type="number"
                  min={1}
                  max={10000}
                  value={policy.s3ListDefaultMaxKeys}
                  onChange={(event) => setPolicy({ ...policy, s3ListDefaultMaxKeys: Number(event.target.value) })}
                />
                </FormField>
                <FormField label={t("setup.buckets.s3ListMaxKeysLimit")} htmlFor="bucket-policy-s3-max-keys-limit">
                <input
                  id="bucket-policy-s3-max-keys-limit"
                  type="number"
                  min={1}
                  max={100000}
                  value={policy.s3ListMaxKeysLimit}
                  onChange={(event) => setPolicy({ ...policy, s3ListMaxKeysLimit: Number(event.target.value) })}
                />
                </FormField>
                <FormField label={t("setup.buckets.s3ChecksumAlgorithm")} htmlFor="bucket-policy-s3-checksum">
                <select
                  id="bucket-policy-s3-checksum"
                  value={policy.s3ChecksumAlgorithm}
                  onChange={(event) => setPolicy({ ...policy, s3ChecksumAlgorithm: event.target.value })}
                >
                  <option value="SHA256">SHA256</option>
                  <option value="ETAG_MD5_COMPATIBLE">ETag MD5</option>
                  <option value="NONE">{t("setup.common.disabled")}</option>
                </select>
                </FormField>
                <FormField label={t("setup.buckets.s3MultipartAbortDays")} htmlFor="bucket-policy-s3-multipart-abort-days">
                <input
                  id="bucket-policy-s3-multipart-abort-days"
                  type="number"
                  min={1}
                  max={365}
                  value={policy.s3MultipartAbortDays}
                  onChange={(event) => setPolicy({ ...policy, s3MultipartAbortDays: Number(event.target.value) })}
                />
                </FormField>
              </FormGrid>
              <CheckboxGrid columns={3} data-testid="hybrid-policy-checkbox-grid">
                <CheckboxField label={t("setup.buckets.s3ListAllowDelimiter")}>
                  <input
                    type="checkbox"
                    checked={policy.s3ListAllowDelimiter}
                    onChange={(event) => setPolicy({ ...policy, s3ListAllowDelimiter: event.target.checked })}
                  />
                </CheckboxField>
                <CheckboxField label={t("setup.buckets.s3VersioningEnabled")}>
                  <input
                    type="checkbox"
                    checked={policy.s3VersioningEnabled}
                    onChange={(event) => setPolicy({ ...policy, s3VersioningEnabled: event.target.checked })}
                  />
                </CheckboxField>
                <CheckboxField label={t("setup.buckets.s3ObjectTaggingEnabled")}>
                  <input
                    type="checkbox"
                    checked={policy.s3ObjectTaggingEnabled}
                    onChange={(event) => setPolicy({ ...policy, s3ObjectTaggingEnabled: event.target.checked })}
                  />
                </CheckboxField>
                <CheckboxField label={t("setup.buckets.allowReplicaEdge")}>
                  <input
                    type="checkbox"
                    checked={policy.allowReplicaEdge}
                    onChange={(event) => setPolicy({ ...policy, allowReplicaEdge: event.target.checked })}
                  />
                </CheckboxField>
                <CheckboxField label={t("setup.buckets.allowPeerSharing")}>
                  <input
                    type="checkbox"
                    checked={policy.allowPeerSharing}
                    onChange={(event) => setPolicy({ ...policy, allowPeerSharing: event.target.checked })}
                  />
                </CheckboxField>
              </CheckboxGrid>
              <div className="bucket-policy-advanced" data-testid="s3-advanced-policy-section">
                <h4>{t("setup.buckets.s3AdvancedTitle")}</h4>
                <FormGrid columns={4}>
                  <FormField label={t("setup.buckets.s3DefaultEncryptionAlgorithm")} htmlFor="bucket-policy-s3-encryption">
                    <select
                      id="bucket-policy-s3-encryption"
                      value={policy.s3DefaultEncryptionAlgorithm}
                      onChange={(event) => setPolicy({ ...policy, s3DefaultEncryptionAlgorithm: event.target.value })}
                    >
                      <option value="NONE">{t("setup.common.disabled")}</option>
                      <option value="AES256">AES256</option>
                      <option value="aws:kms">aws:kms</option>
                    </select>
                  </FormField>
                  <FormField label={t("setup.buckets.s3DefaultEncryptionKeyId")} htmlFor="bucket-policy-s3-encryption-key">
                    <input
                      id="bucket-policy-s3-encryption-key"
                      value={policy.s3DefaultEncryptionKeyId ?? ""}
                      onChange={(event) => setPolicy({ ...policy, s3DefaultEncryptionKeyId: event.target.value })}
                    />
                  </FormField>
                  <FormField label={t("setup.buckets.s3ObjectLockDefaultMode")} htmlFor="bucket-policy-s3-object-lock-mode">
                    <select
                      id="bucket-policy-s3-object-lock-mode"
                      value={policy.s3ObjectLockDefaultMode ?? ""}
                      onChange={(event) => setPolicy({ ...policy, s3ObjectLockDefaultMode: emptyStringToNull(event.target.value) })}
                    >
                      <option value="">{t("setup.common.disabled")}</option>
                      <option value="GOVERNANCE">GOVERNANCE</option>
                      <option value="COMPLIANCE">COMPLIANCE</option>
                    </select>
                  </FormField>
                  <FormField label={t("setup.buckets.s3ObjectLockDefaultRetainDays")} htmlFor="bucket-policy-s3-object-lock-days">
                    <input
                      id="bucket-policy-s3-object-lock-days"
                      type="number"
                      min={1}
                      value={policy.s3ObjectLockDefaultRetainDays ?? ""}
                      onChange={(event) => setPolicy({ ...policy, s3ObjectLockDefaultRetainDays: event.target.value ? Number(event.target.value) : null })}
                    />
                  </FormField>
                </FormGrid>
                <CheckboxGrid columns={2}>
                  <CheckboxField label={t("setup.buckets.s3ObjectLockEnabled")}>
                    <input
                      type="checkbox"
                      checked={policy.s3ObjectLockEnabled}
                      onChange={(event) => setPolicy({ ...policy, s3ObjectLockEnabled: event.target.checked })}
                    />
                  </CheckboxField>
                </CheckboxGrid>
                <FormGrid columns={1}>
                  <FormField label={t("setup.buckets.s3LifecycleRules")} htmlFor="bucket-policy-s3-lifecycle-rules">
                    <textarea
                      id="bucket-policy-s3-lifecycle-rules"
                      rows={5}
                      spellCheck={false}
                      value={s3Json.lifecycle}
                      onChange={(event) => setS3Json({ ...s3Json, lifecycle: event.target.value })}
                    />
                  </FormField>
                  <FormField label={t("setup.buckets.s3ResourcePolicy")} htmlFor="bucket-policy-s3-resource-policy">
                    <textarea
                      id="bucket-policy-s3-resource-policy"
                      rows={5}
                      spellCheck={false}
                      value={s3Json.resourcePolicy}
                      onChange={(event) => setS3Json({ ...s3Json, resourcePolicy: event.target.value })}
                    />
                  </FormField>
                  <FormField label={t("setup.buckets.s3EventNotifications")} htmlFor="bucket-policy-s3-event-notifications">
                    <textarea
                      id="bucket-policy-s3-event-notifications"
                      rows={5}
                      spellCheck={false}
                      value={s3Json.notifications}
                      onChange={(event) => setS3Json({ ...s3Json, notifications: event.target.value })}
                    />
                  </FormField>
                </FormGrid>
              </div>
            </>
          ) : null}
        </FormSection>

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

function policyJsonState(policy: BucketPolicy) {
  return {
    lifecycle: stableJson(policy.s3LifecycleRules ?? []),
    resourcePolicy: stableJson(policy.s3ResourcePolicy ?? { Version: "2012-10-17", Statement: [] }),
    notifications: stableJson(policy.s3EventNotifications ?? { EventBridgeEnabled: false, Rules: [] })
  };
}

function stableJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

function parseAdvancedS3Json(
  fields: { lifecycle: string; resourcePolicy: string; notifications: string },
  t: (key: string, options?: Record<string, string>) => string
):
  | { lifecycle: unknown; resourcePolicy: unknown; notifications: unknown }
  | { error: string } {
  const lifecycle = parseJsonField(fields.lifecycle, t("setup.buckets.s3LifecycleRules"), t("setup.buckets.invalidJson"));
  if ("error" in lifecycle) {
    return lifecycle;
  }
  const resourcePolicy = parseJsonField(fields.resourcePolicy, t("setup.buckets.s3ResourcePolicy"), t("setup.buckets.invalidJson"));
  if ("error" in resourcePolicy) {
    return resourcePolicy;
  }
  const notifications = parseJsonField(fields.notifications, t("setup.buckets.s3EventNotifications"), t("setup.buckets.invalidJson"));
  if ("error" in notifications) {
    return notifications;
  }
  return {
    lifecycle: lifecycle.value,
    resourcePolicy: resourcePolicy.value,
    notifications: notifications.value
  };
}

function parseJsonField(value: string, label: string, invalidJson: string): { value: unknown } | { error: string } {
  try {
    return { value: JSON.parse(value) };
  } catch {
    return { error: `${label}: ${invalidJson}` };
  }
}

function emptyStringToNull(value: string | null): string | null {
  const trimmed = value?.trim() ?? "";
  return trimmed ? trimmed : null;
}
