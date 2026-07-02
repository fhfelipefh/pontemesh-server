import { ReactNode, useEffect, useMemo, useState } from "react";
import { CheckCircle2, UploadCloud, XCircle, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatBytes } from "../utils/adminFormat";
import { UploadProgressContext, UploadProgressContextValue, UploadTask } from "./uploadProgressContext";

const RECENT_UPLOADS_STORAGE_KEY = "pontemesh.recentUploads";
const MAX_RECENT_UPLOADS = 10;

export function UploadProgressProvider({ children }: { children: ReactNode }) {
  const [uploads, setUploads] = useState<UploadTask[]>(() => loadRecentUploads());

  useEffect(() => {
    // Recent uploads are browser-local UI history, not backend operational history.
    // Keep only display-safe fields; do not persist backend error messages or paths.
    const recentUploads = uploads
      .filter((upload) => upload.status !== "uploading")
      .map(({ id, fileName, loadedBytes, totalBytes, percent, status, createdAt, finishedAt }) => ({
        id,
        fileName,
        loadedBytes,
        totalBytes,
        percent,
        status,
        createdAt,
        finishedAt
      }))
      .slice(0, MAX_RECENT_UPLOADS);
    localStorage.setItem(RECENT_UPLOADS_STORAGE_KEY, JSON.stringify(recentUploads));
  }, [uploads]);

  const value = useMemo<UploadProgressContextValue>(() => ({
    addUpload(fileName) {
      const id = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
      setUploads((current) => [
        {
          id,
          fileName,
          loadedBytes: 0,
          totalBytes: null,
          percent: null,
          status: "uploading",
          createdAt: new Date().toISOString()
        },
        ...current
      ]);
      return id;
    },
    updateUpload(id, progress) {
      setUploads((current) => current.map((upload) => (
        upload.id === id && upload.status === "uploading"
          ? { ...upload, ...progress }
          : upload
      )));
    },
    finishUpload(id, status, message) {
      setUploads((current) => current.map((upload) => (
        upload.id === id
          ? {
              ...upload,
              status,
              percent: status === "complete" ? 100 : upload.percent,
              message,
              finishedAt: new Date().toISOString()
            }
          : upload
      )).slice(0, MAX_RECENT_UPLOADS));
    }
  }), []);

  function dismissUpload(id: string) {
    setUploads((current) => current.filter((upload) => upload.id !== id));
  }

  return (
    <UploadProgressContext.Provider value={value}>
      {children}
      <UploadProgressPanel uploads={uploads} onDismiss={dismissUpload} />
    </UploadProgressContext.Provider>
  );
}

function loadRecentUploads(): UploadTask[] {
  try {
    const raw = localStorage.getItem(RECENT_UPLOADS_STORAGE_KEY);
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw) as UploadTask[];
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .filter((upload) => upload && upload.status !== "uploading" && typeof upload.id === "string" && typeof upload.fileName === "string")
      .slice(0, MAX_RECENT_UPLOADS);
  } catch {
    return [];
  }
}

function UploadProgressPanel({ uploads, onDismiss }: { uploads: UploadTask[]; onDismiss: (id: string) => void }) {
  const { t } = useTranslation();
  if (uploads.length === 0) {
    return null;
  }

  return (
    <aside className="upload-progress-panel" aria-label={t("setup.uploads.title")}>
      <header>
        <strong>{t("setup.uploads.title")}</strong>
      </header>
      <div className="upload-progress-panel__list">
        {uploads.map((upload) => (
          <article className="upload-progress-item" data-status={upload.status} key={upload.id}>
            <div className="upload-progress-item__icon" aria-hidden="true">
              {upload.status === "complete" ? <CheckCircle2 size={18} /> : upload.status === "error" ? <XCircle size={18} /> : <UploadCloud size={18} />}
            </div>
            <div className="upload-progress-item__body">
              <div className="upload-progress-item__row">
                <strong title={upload.fileName}>{upload.fileName}</strong>
                <span>{statusLabel(upload, t)}</span>
              </div>
              <div className="upload-progress-bar" aria-hidden="true">
                <span style={{ width: `${upload.percent ?? 8}%` }} />
              </div>
              <small>
                {upload.message ?? progressLabel(upload, t)}
              </small>
            </div>
            <button
              type="button"
              aria-label={t("setup.uploads.close")}
              title={t("setup.uploads.close")}
              onClick={() => onDismiss(upload.id)}
            >
              <X size={16} aria-hidden="true" />
            </button>
          </article>
        ))}
      </div>
    </aside>
  );
}

function statusLabel(upload: UploadTask, t: (key: string, options?: Record<string, unknown>) => string) {
  if (upload.status === "complete") {
    return t("setup.uploads.complete");
  }
  if (upload.status === "error") {
    return t("setup.uploads.failed");
  }
  return upload.percent === null
    ? t("setup.uploads.uploading")
    : t("setup.uploads.percent", { percent: upload.percent });
}

function progressLabel(upload: UploadTask, t: (key: string, options?: Record<string, unknown>) => string) {
  if (upload.totalBytes) {
    return `${formatBytes(upload.loadedBytes)} / ${formatBytes(upload.totalBytes)}`;
  }
  return upload.loadedBytes > 0 ? formatBytes(upload.loadedBytes) : t("setup.uploads.waiting");
}
