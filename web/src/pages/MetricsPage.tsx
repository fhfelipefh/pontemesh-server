import { ReactNode, useEffect, useState } from "react";
import { Activity, DownloadCloud, Gauge, SplitSquareHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  BucketTrafficMetric,
  getBucketTrafficMetrics,
  getOriginTrafficMetrics,
  getReplicaTrafficMetrics,
  OriginTrafficMetrics,
  ReplicaTrafficMetrics
} from "../api/dashboardApi";
import { ErrorMessage } from "../components/ErrorMessage";

type PeriodOption = "1h" | "24h" | "7d" | "30d" | "all";

const PERIOD_OPTIONS: { key: PeriodOption; label: (t: (k: string) => string) => string }[] = [
  { key: "1h", label: (t) => t("setup.metrics.periods.1h") },
  { key: "24h", label: (t) => t("setup.metrics.periods.24h") },
  { key: "7d", label: (t) => t("setup.metrics.periods.7d") },
  { key: "30d", label: (t) => t("setup.metrics.periods.30d") },
  { key: "all", label: (t) => t("setup.metrics.periods.all") }
];

export function MetricsPage() {
  const { t } = useTranslation();
  const [period, setPeriod] = useState<PeriodOption>("all");
  const [metrics, setMetrics] = useState<OriginTrafficMetrics | null>(null);
  const [replicaMetrics, setReplicaMetrics] = useState<ReplicaTrafficMetrics | null>(null);
  const [bucketMetrics, setBucketMetrics] = useState<BucketTrafficMetric[]>([]);
  const [error, setError] = useState("");

  useEffect(() => {
    Promise.all([
      getOriginTrafficMetrics(period),
      getReplicaTrafficMetrics(period),
      getBucketTrafficMetrics(period)
    ])
      .then(([origin, replicas, buckets]) => {
        setMetrics(origin);
        setReplicaMetrics(replicas);
        setBucketMetrics(buckets);
      })
      .catch((loadError) => {
        setError(loadError instanceof Error ? loadError.message : t("setup.metrics.loadFailed"));
      });
  }, [t, period]);

  if (error) {
    return <ErrorMessage message={error} />;
  }

  if (!metrics || !replicaMetrics) {
    return <div className="admin-loading">{t("setup.common.loading")}</div>;
  }

  const totals = bucketMetrics.reduce(
    (summary, bucket) => ({
      peerBytesServed: summary.peerBytesServed + bucket.peerBytesServed,
      fallbackEvents: summary.fallbackEvents + bucket.fallbackEvents,
      integrityFailures: summary.integrityFailures + bucket.integrityFailures,
      originOffloadBytes: summary.originOffloadBytes + bucket.originOffloadBytes
    }),
    { peerBytesServed: 0, fallbackEvents: 0, integrityFailures: 0, originOffloadBytes: 0 }
  );

  return (
    <div className="dashboard-grid">
      <section className="admin-hero admin-hero--with-controls">
        <div>
          <span>{t("setup.metrics.overview")}</span>
          <h1>{t("setup.metrics.title")}</h1>
        </div>
        <div className="metrics-period-filter" role="group" aria-label={t("setup.metrics.periodLabel")}>
          {PERIOD_OPTIONS.map((opt) => (
            <button
              key={opt.key}
              type="button"
              className={`metrics-period-pill ${period === opt.key ? "metrics-period-pill--active" : ""}`}
              onClick={() => setPeriod(opt.key)}
            >
              {opt.label(t)}
            </button>
          ))}
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
      <Metric icon={<DownloadCloud size={20} />} label={t("setup.metrics.peerBytesServed")} value={formatBytes(totals.peerBytesServed)} />
      <Metric icon={<Gauge size={20} />} label={t("setup.metrics.originOffloadBytes")} value={formatBytes(totals.originOffloadBytes)} />
      <Metric icon={<SplitSquareHorizontal size={20} />} label={t("setup.metrics.fallbackEvents")} value={String(totals.fallbackEvents)} />
      <Metric icon={<Activity size={20} />} label={t("setup.metrics.integrityFailures")} value={String(totals.integrityFailures)} />
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
