import { ChangeEvent, FormEvent, useCallback, useEffect, useRef, useState } from "react";
import { Download, Search, Trash2, UploadCloud, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  ObjectSummary,
  PaginatedResponse,
  getObjectDownloadUrl,
  listObjects,
  uploadObjectWithProgress
} from "../api/bucketsApi";
import { Button } from "./Button";
import { EmptyState, PageSizeSelect, Pagination } from "./AdminListControls";
import { ErrorMessage } from "./ErrorMessage";
import { useUploadProgress } from "./uploadProgressContext";
import { emptyPage, formatBytes, formatDate } from "../utils/adminFormat";

const OBJECT_PAGE_SIZE_OPTIONS = [10, 20, 50, 100];

type ObjectManagerProps = {
  bucketName: string;
  refreshNonce?: number;
  externalError?: string;
  onChanged: () => Promise<void> | void;
  onConfirmDeleteObject: (objectKey: string) => void;
};

export function ObjectManager({
  bucketName,
  refreshNonce = 0,
  externalError = "",
  onChanged,
  onConfirmDeleteObject
}: ObjectManagerProps) {
  const { t } = useTranslation();
  const uploadProgress = useUploadProgress();
  const [objectsPage, setObjectsPage] = useState<PaginatedResponse<ObjectSummary>>(emptyPage(20));
  const [objectQuery, setObjectQuery] = useState("");
  const [objectSearch, setObjectSearch] = useState("");
  const [objectPageNumber, setObjectPageNumber] = useState(1);
  const [objectPageSize, setObjectPageSize] = useState(20);
  const [loadingObjects, setLoadingObjects] = useState(true);
  const [objectError, setObjectError] = useState("");
  const [uploadModalOpen, setUploadModalOpen] = useState(false);
  const [objectKey, setObjectKey] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState("");
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const resetUploadForm = useCallback(() => {
    setFile(null);
    setObjectKey("");
    setUploadError("");
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  }, []);

  const refreshObjects = useCallback(async () => {
    setLoadingObjects(true);
    setObjectError("");
    try {
      setObjectsPage(await listObjects(bucketName, {
        query: objectSearch,
        page: objectPageNumber,
        pageSize: objectPageSize
      }));
    } catch (loadError) {
      setObjectError(loadError instanceof Error ? loadError.message : t("setup.objects.loadFailed"));
    } finally {
      setLoadingObjects(false);
    }
  }, [bucketName, objectPageNumber, objectPageSize, objectSearch, t]);

  useEffect(() => {
    setObjectPageNumber(1);
    setObjectQuery("");
    setObjectSearch("");
    setUploadModalOpen(false);
    resetUploadForm();
  }, [bucketName, resetUploadForm]);

  useEffect(() => {
    refreshObjects();
  }, [refreshObjects, refreshNonce]);

  async function handleUpload(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!file) {
      return;
    }
    setUploading(true);
    setUploadError("");
    setObjectError("");
    const uploadId = uploadProgress.addUpload(file.name);
    try {
      await uploadObjectWithProgress(bucketName, file, objectKey, (progress) => {
        uploadProgress.updateUpload(uploadId, progress);
      });
      uploadProgress.finishUpload(uploadId, "complete");
      setUploadModalOpen(false);
      resetUploadForm();
      await refreshObjects();
      await onChanged();
    } catch (uploadErrorValue) {
      const message = translatedUploadError(uploadErrorValue, t);
      uploadProgress.finishUpload(uploadId, "error", message);
      setUploadError(message);
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

  function openUploadModal() {
    setUploadError("");
    setUploadModalOpen(true);
  }

  function closeUploadModal() {
    if (uploading) {
      return;
    }
    setUploadModalOpen(false);
    resetUploadForm();
  }

  const objectSearchActive = objectSearch.trim().length > 0;
  const hasObjects = objectsPage.items.length > 0;

  return (
    <div className="object-manager">
      <div className="objects-toolbar">
        <form className="objects-search" onSubmit={handleObjectSearch}>
          <label data-testid="object-search-control">
            <Search size={17} aria-hidden="true" />
            <input
              data-testid="object-search-input"
              value={objectQuery}
              onChange={(event) => setObjectQuery(event.target.value)}
              placeholder={t("setup.objects.search")}
              aria-label={t("setup.objects.search")}
            />
          </label>
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
        <Button
          className="objects-upload-button"
          data-testid="open-upload-object-button"
          type="button"
          icon={<UploadCloud size={17} aria-hidden="true" />}
          onClick={openUploadModal}
        >
          {t("setup.objects.upload")}
        </Button>
      </div>

      <ErrorMessage message={objectError || externalError} />

      <section className="objects-table-card" data-testid="objects-table-card">
        <div className="objects-table-container">
          {loadingObjects ? (
            <div className="admin-loading">{t("setup.objects.loading")}</div>
          ) : !hasObjects ? (
            <EmptyState
              title={objectSearchActive ? t("setup.objects.noResultsTitle") : t("setup.objects.emptyTitle")}
              description={objectSearchActive ? t("setup.objects.noResultsDescription") : t("setup.objects.emptyDescription")}
            >
              {!objectSearchActive ? (
                <Button
                  className="objects-empty-upload-button"
                  type="button"
                  icon={<UploadCloud size={17} aria-hidden="true" />}
                  onClick={openUploadModal}
                >
                  {t("setup.objects.upload")}
                </Button>
              ) : null}
            </EmptyState>
          ) : (
            <div className="objects-table" data-testid="object-list" role="table" aria-label={t("setup.objects.title")}>
              <div className="objects-table-row objects-table-row--head" role="row">
                <span role="columnheader">{t("setup.objects.key")}</span>
                <span role="columnheader">{t("setup.objects.size")}</span>
                <span role="columnheader">{t("setup.objects.contentType")}</span>
                <span role="columnheader">{t("setup.objects.updatedAt")}</span>
                <span role="columnheader">{t("setup.common.actions")}</span>
              </div>
              {objectsPage.items.map((object) => (
                <div className="objects-table-row" data-testid="object-row" role="row" key={object.key}>
                  <span role="cell" title={object.key}>{object.key}</span>
                  <span role="cell">{formatBytes(object.sizeBytes)}</span>
                  <span role="cell" title={object.contentType}>{object.contentType}</span>
                  <span role="cell">{formatDate(object.updatedAt ?? object.createdAt)}</span>
                  <span className="objects-table-actions" role="cell">
                    <a
                      className="table-action table-action-button"
                      href={getObjectDownloadUrl(bucketName, object.key)}
                      title={t("setup.objects.downloadObject")}
                      download
                    >
                      <Download size={15} aria-hidden="true" />
                      <span>{t("setup.objects.download")}</span>
                    </a>
                    <button
                      className="table-action table-action-button table-action--danger"
                      type="button"
                      title={t("setup.objects.deleteObject")}
                      onClick={() => onConfirmDeleteObject(object.key)}
                    >
                      <Trash2 size={15} aria-hidden="true" />
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
      </section>

      {uploadModalOpen ? (
        <div className="settings-modal-backdrop" data-testid="upload-object-dialog-backdrop" role="presentation">
          <form className="settings-modal upload-object-dialog" data-testid="upload-object-dialog" role="dialog" aria-modal="true" aria-labelledby="upload-object-title" onSubmit={handleUpload}>
            <div className="settings-modal__header">
              <div>
                <h3 id="upload-object-title">{t("setup.objects.upload")}</h3>
                <p>{t("setup.objects.uploadBucket", { bucket: bucketName })}</p>
              </div>
              <button
                className="settings-modal__close"
                type="button"
                aria-label={t("setup.common.close")}
                onClick={closeUploadModal}
              >
                <X size={18} aria-hidden="true" />
              </button>
            </div>

            <ErrorMessage message={uploadError} />

            <label className="upload-object-field">
              <span>{t("setup.objects.key")}</span>
              <input
                data-testid="object-key-input"
                value={objectKey}
                onChange={(event) => setObjectKey(event.target.value)}
                placeholder={t("setup.objects.keyPlaceholder")}
              />
            </label>

            <div className="upload-object-file">
              <span>{t("setup.objects.file")}</span>
              <input ref={fileInputRef} data-testid="object-file-input" type="file" onChange={handleFileChange} />
              <button type="button" onClick={() => fileInputRef.current?.click()}>
                <UploadCloud size={17} aria-hidden="true" />
                <span>{t("setup.objects.chooseFile")}</span>
              </button>
              <small>{file?.name ?? t("setup.objects.noFileChosen")}</small>
            </div>

            <div className="settings-modal__actions">
              <button className="settings-secondary-button" type="button" onClick={closeUploadModal}>
                {t("setup.common.cancel")}
              </button>
              <Button className="settings-modal__primary" data-testid="upload-object-button" type="submit" loading={uploading} disabled={!file} icon={<UploadCloud size={17} aria-hidden="true" />}>
                {uploading ? t("setup.objects.uploading") : t("setup.objects.upload")}
              </Button>
            </div>
          </form>
        </div>
      ) : null}
    </div>
  );
}

function translatedUploadError(
  uploadError: unknown,
  t: (key: string, options?: Record<string, unknown>) => string
): string {
  const message = uploadError instanceof Error ? uploadError.message : "";
  if (message.toLowerCase().includes("failed to read uploaded file")) {
    return t("setup.objects.readUploadedFileFailed");
  }
  if (message.toLowerCase().includes("active object already exists in bucket")) {
    return t("setup.objects.activeObjectExists");
  }
  return message || t("setup.objects.uploadFailed");
}
