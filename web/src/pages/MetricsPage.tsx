import { ReactNode, useEffect, useState } from "react";
import { Activity, DownloadCloud, Gauge, SplitSquareHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  getOriginTrafficMetrics,
  getReplicaTrafficMetrics,
  OriginTrafficMetrics,
  ReplicaTrafficMetrics
} from "../api/dashboardApi";
import { ErrorMessage } from "../components/ErrorMessage";

export function MetricsPage() {
  const { t } = useTranslation();
  const [metrics, setMetrics] = useState<OriginTrafficMetrics | null>(null);
  const [replicaMetrics, setReplicaMetrics] = useState<ReplicaTrafficMetrics | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    Promise.all([getOriginTrafficMetrics(), getReplicaTrafficMetrics()])
      .then(([origin, replicas]) => {
        setMetrics(origin);
        setReplicaMetrics(replicas);
      })
      .catch((loadError) => {
        setError(loadError instanceof Error ? loadError.message : t("setup.metrics.loadFailed"));
      });
  }, [t]);

  if (error) {
    return <ErrorMessage message={error} />;
  }

  if (!metrics || !replicaMetrics) {
    return <div className="admin-loading">{t("setup.common.loading")}</div>;
  }

  return (
    <div className="dashboard-grid">
      <section className="admin-hero">
        <div>
          <span>{t("setup.metrics.overview")}</span>
          <h1>{t("setup.metrics.title")}</h1>
          <p>{t("setup.metrics.description")}</p>
        </div>
      </section>
      <Metric icon={<Activity size={20} />} label={t("setup.metrics.totalRequests")} value={String(metrics.totalRequests)} />
      <Metric icon={<DownloadCloud size={20} />} label={t("setup.metrics.fullObjectRequests")} value={String(metrics.fullObjectRequests)} />
      <Metric icon={<SplitSquareHorizontal size={20} />} label={t("setup.metrics.rangeRequests")} value={String(metrics.rangeRequests)} />
      <Metric icon={<Gauge size={20} />} label={t("setup.metrics.totalBytesServed")} value={formatBytes(metrics.totalBytesServed)} />
      <Metric icon={<Activity size={20} />} label={t("setup.metrics.activeReplicas")} value={String(replicaMetrics.activeReplicas)} />
      <Metric icon={<DownloadCloud size={20} />} label={t("setup.metrics.replicaBytesSynced")} value={formatBytes(replicaMetrics.totalBytesSynced)} />
      <Metric icon={<SplitSquareHorizontal size={20} />} label={t("setup.metrics.replicaFragmentsSynced")} value={String(replicaMetrics.totalFragmentsSynced)} />
      <Metric icon={<Gauge size={20} />} label={t("setup.metrics.replicaFailures")} value={String(replicaMetrics.syncFailures + replicaMetrics.authFailures)} />
    </div>
  );
}

function Metric({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <section className="metric-card">
      <div className="metric-card__icon">{icon}</div>
      <span>{label}</span>
      <strong>{value}</strong>
    </section>
  );
}

function formatBytes(value: number): string {
  if (value < 1024) {
    return `${value} B`;
  }
  const units = ["KB", "MB", "GB", "TB"];
  let size = value / 1024;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${size.toFixed(1)} ${units[unit]}`;
}
