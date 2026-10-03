import { useState } from "react";
import { useTranslation } from "react-i18next";
import { HardDrive, Plus, Trash2 } from "lucide-react";
import { StoragePoolStatus } from "../../api/storageApi";
import { formatBytes } from "../../utils/adminFormat";
import { Button } from "../Button";
import { ConfirmDialog } from "../AdminListControls";
import { SettingsSection } from "./SettingsSection";

export type StoragePoolCardProps = {
  pool: StoragePoolStatus | null;
  loading: boolean;
  error: string;
  actionMessage: string | null;
  isAdmin: boolean;
  onAddDrive: (path: string) => Promise<void>;
  onDrainDrive: (path: string) => Promise<void>;
  onStrategyChange: (strategy: string) => Promise<void>;
};

export function StoragePoolCard({
  pool,
  loading,
  error,
  actionMessage,
  isAdmin,
  onAddDrive,
  onDrainDrive,
  onStrategyChange,
}: StoragePoolCardProps) {
  const { t } = useTranslation();
  const [newDrivePath, setNewDrivePath] = useState("");
  const [adding, setAdding] = useState(false);
  const [drainConfirmDrive, setDrainConfirmDrive] = useState<string | null>(null);
  const [draining, setDraining] = useState(false);

  async function handleAddDrive() {
    if (!newDrivePath.trim()) return;
    try {
      setAdding(true);
      await onAddDrive(newDrivePath.trim());
      setNewDrivePath("");
    } finally {
      setAdding(false);
    }
  }

  async function handleConfirmDrain() {
    if (!drainConfirmDrive) return;
    try {
      setDraining(true);
      await onDrainDrive(drainConfirmDrive);
      setDrainConfirmDrive(null);
    } finally {
      setDraining(false);
    }
  }

  return (
    <SettingsSection
      className="settings-card--wide"
      title={t("setup.settings.storagePool.title")}
      description={t("setup.settings.storagePool.description")}
      icon={<HardDrive size={20} />}
    >
      {error ? <p className="error-message">{error}</p> : null}
      {actionMessage ? (
        <p className="settings-success" role="status">
          {actionMessage}
        </p>
      ) : null}

      {loading || !pool ? (
        <div className="settings-loading">{t("setup.common.loading")}</div>
      ) : (
        <>
          <div className="storage-capacity-form__thresholds">
            <label htmlFor="storage-allocation-strategy">
              <span>{t("setup.settings.storagePool.strategy")}</span>
              <div>
                <select
                  id="storage-allocation-strategy"
                  value={pool.allocationStrategy}
                  disabled={!isAdmin}
                  onChange={event => void onStrategyChange(event.target.value)}
                >
                  <option value="MOST_AVAILABLE_FREE_SPACE">
                    {t("setup.settings.storagePool.strategyMostFree")}
                  </option>
                  <option value="ROUND_ROBIN">
                    {t("setup.settings.storagePool.strategyRoundRobin")}
                  </option>
                </select>
              </div>
            </label>
          </div>

          {isAdmin ? (
            <form
              className="inline-form"
              onSubmit={event => {
                event.preventDefault();
                void handleAddDrive();
              }}
            >
              <input
                id="new-storage-drive-path"
                type="text"
                placeholder={t("setup.settings.storagePool.drivePathPlaceholder")}
                value={newDrivePath}
                disabled={adding}
                onChange={event => setNewDrivePath(event.target.value)}
                aria-label={t("setup.settings.storagePool.path")}
              />
              <button
                className="settings-create-key-button"
                type="submit"
                disabled={adding || !newDrivePath.trim()}
              >
                <Plus size={17} aria-hidden="true" />
                {t("setup.settings.storagePool.addDrive")}
              </button>
            </form>
          ) : null}

          <div className="table-container">
            <table className="buckets-table" role="table">
              <thead>
                <tr role="row">
                  <th role="columnheader">{t("setup.settings.storagePool.path")}</th>
                  <th role="columnheader">{t("setup.settings.storagePool.status")}</th>
                  <th role="columnheader">{t("setup.settings.storagePool.usage")}</th>
                  <th role="columnheader">{t("setup.settings.storagePool.available")}</th>
                  {isAdmin ? <th role="columnheader">{t("setup.common.actions")}</th> : null}
                </tr>
              </thead>
              <tbody>
                {pool.drives.map(drive => (
                  <tr key={drive.path} role="row">
                    <td role="cell">
                      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                        <code>{drive.path}</code>
                        <span
                          className="settings-status-badge"
                          data-active={drive.isPrimary}
                        >
                          {drive.isPrimary
                            ? t("setup.settings.storagePool.primary")
                            : t("setup.settings.storagePool.auxiliary")}
                        </span>
                      </div>
                    </td>
                    <td role="cell">
                      <span>{drive.level}</span>
                    </td>
                    <td role="cell">
                      {drive.usedPercent !== null ? `${drive.usedPercent.toFixed(1)}%` : "-"}
                    </td>
                    <td role="cell">
                      {drive.availableBytes !== null ? formatBytes(drive.availableBytes) : "-"}
                    </td>
                    {isAdmin ? (
                      <td role="cell">
                        {!drive.isPrimary ? (
                          <Button
                            className="button--danger"
                            type="button"
                            icon={<Trash2 size={15} aria-hidden="true" />}
                            onClick={() => setDrainConfirmDrive(drive.path)}
                          >
                            {t("setup.settings.storagePool.drainDrive")}
                          </Button>
                        ) : null}
                      </td>
                    ) : null}
                  </tr>
                ))}
                {pool.drives.length <= 1 ? (
                  <tr role="row">
                    <td colSpan={isAdmin ? 5 : 4} style={{ textAlign: "center", padding: "0.75rem" }}>
                      {t("setup.settings.storagePool.emptyPool")}
                    </td>
                  </tr>
                ) : null}
              </tbody>
            </table>
          </div>

          {drainConfirmDrive ? (
            <ConfirmDialog
              title={t("setup.settings.storagePool.confirmDrainTitle")}
              description={t("setup.settings.storagePool.confirmDrainText")}
              confirmLabel={
                draining
                  ? t("setup.settings.storagePool.draining")
                  : t("setup.settings.storagePool.drainDrive")
              }
              onCancel={() => setDrainConfirmDrive(null)}
              onConfirm={() => void handleConfirmDrain()}
            />
          ) : null}
        </>
      )}
    </SettingsSection>
  );
}
