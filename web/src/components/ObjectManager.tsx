import { ChangeEvent, FormEvent, useCallback, useEffect, useRef, useState } from "react";
import { Download, Search, Trash2, UploadCloud } from "lucide-react";
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
  const [objectKey, setObjectKey] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

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
  }, [bucketName]);

  useEffect(() => {
    refreshObjects();
  }, [refreshObjects, refreshNonce]);

  async function handleUpload(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    if (!file) {
      return;
    }
    setUploading(true);
    setObjectError("");
    const uploadId = uploadProgress.addUpload(file.name);
    try {
      await uploadObjectWithProgress(bucketName, file, objectKey, (progress) => {
        uploadProgress.updateUpload(uploadId, progress);
      });
      uploadProgress.finishUpload(uploadId, "complete");
      setFile(null);
      setObjectKey("");
      form.reset();
      await refreshObjects();
      await onChanged();
    } catch (uploadError) {
      const message = uploadError instanceof Error ? uploadError.message : t("setup.objects.uploadFailed");
      uploadProgress.finishUpload(uploadId, "error", message);
      setObjectError(message);
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
    <div className="object-manager">
      <form className="bucket-upload" onSubmit={handleUpload}>
        <label>
          <span>{t("setup.objects.key")}</span>
          <input
            data-testid="object-key-input"
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
        <Button data-testid="upload-object-button" type="submit" loading={uploading} disabled={!file} icon={<UploadCloud size={17} aria-hidden="true" />}>
          {uploading ? t("setup.objects.uploading") : t("setup.objects.upload")}
        </Button>
      </form>

      <div className="bucket-drawer__tools">
        <form className="buckets-search" onSubmit={handleObjectSearch}>
          <label>
            <Search size={17} aria-hidden="true" />
            <input
              data-testid="object-search-input"
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
          <div className="object-table object-table--drawer" data-testid="object-list" role="table" aria-label={t("setup.objects.title")}>
            <div className="object-table__head" role="row">
              <span role="columnheader">{t("setup.objects.key")}</span>
              <span role="columnheader">{t("setup.objects.size")}</span>
              <span role="columnheader">{t("setup.objects.contentType")}</span>
              <span role="columnheader">{t("setup.objects.updatedAt")}</span>
              <span role="columnheader">{t("setup.common.actions")}</span>
            </div>
            {objectsPage.items.map((object) => (
              <div className="object-table__row" data-testid="object-row" role="row" key={object.key}>
                <span role="cell" title={object.key}>{object.key}</span>
                <span role="cell">{formatBytes(object.sizeBytes)}</span>
                <span role="cell" title={object.contentType}>{object.contentType}</span>
                <span role="cell">{formatDate(object.updatedAt ?? object.createdAt)}</span>
                <span className="buckets-table__actions" role="cell">
                  <a
                    className="table-action"
                    href={getObjectDownloadUrl(bucketName, object.key)}
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
    </div>
  );
}
