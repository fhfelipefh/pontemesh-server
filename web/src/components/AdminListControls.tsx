import { ChevronLeft, ChevronRight, Trash2, X } from "lucide-react";
import { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./Button";

export function ConfirmDialog({
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
    <div className="settings-modal-backdrop" data-testid="modal-backdrop" role="presentation">
      <div className="settings-modal" data-testid="confirm-dialog" role="dialog" aria-modal="true" aria-labelledby="confirm-title">
        <div className="settings-modal__header">
          <div>
            <h3 id="confirm-title">{title}</h3>
            <p>{description}</p>
          </div>
          <button
            className="settings-modal__close"
            data-testid="confirm-dialog-close"
            type="button"
            aria-label={t("setup.common.close")}
            onClick={onCancel}
          >
            <X size={18} aria-hidden="true" />
          </button>
        </div>
        <div className="settings-modal__actions">
          <button className="settings-secondary-button" type="button" onClick={onCancel}>
            <X size={16} aria-hidden="true" />
            {t("setup.common.cancel")}
          </button>
          <Button className="settings-modal__primary button--danger" type="button" onClick={onConfirm} icon={<Trash2 size={17} aria-hidden="true" />}>
            {t("setup.common.confirm")}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function PageSizeSelect({
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

export function Pagination({
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

export function EmptyState({ title, description, children }: { title: string; description: string; children?: ReactNode }) {
  return (
    <div className="empty-state">
      <strong>{title}</strong>
      <p>{description}</p>
      {children}
    </div>
  );
}
