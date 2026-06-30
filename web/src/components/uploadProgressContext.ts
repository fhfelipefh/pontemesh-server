import { createContext, useContext } from "react";

export type UploadStatus = "uploading" | "complete" | "error";

export type UploadTask = {
  id: string;
  fileName: string;
  loadedBytes: number;
  totalBytes: number | null;
  percent: number | null;
  status: UploadStatus;
  message?: string;
};

export type UploadProgressContextValue = {
  addUpload: (fileName: string) => string;
  updateUpload: (id: string, progress: Pick<UploadTask, "loadedBytes" | "totalBytes" | "percent">) => void;
  finishUpload: (id: string, status: Exclude<UploadStatus, "uploading">, message?: string) => void;
};

export const UploadProgressContext = createContext<UploadProgressContextValue | null>(null);

export function useUploadProgress() {
  const context = useContext(UploadProgressContext);
  if (!context) {
    throw new Error("useUploadProgress must be used inside UploadProgressProvider");
  }
  return context;
}
