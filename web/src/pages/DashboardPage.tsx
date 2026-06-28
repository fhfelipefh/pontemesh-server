import { ReactNode, useEffect, useState } from "react";
import { Boxes, CheckCircle2, Cpu, Database, HardDrive, Server, ShieldCheck } from "lucide-react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { DashboardSummary, getDashboardSummary } from "../api/dashboardApi";
import { ErrorMessage } from "../components/ErrorMessage";

export function DashboardPage() {
  const { t } = useTranslation();
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    getDashboardSummary()
      .then(setSummary)
      .catch((loadError) => setError(loadError instanceof Error ? loadError.message : t("setup.dashboard.loadFailed")));
  }, [t]);

  if (error) {
    return <ErrorMessage message={error} />;
  }

  if (!summary) {
    return <div className="admin-loading">{t("setup.common.loading")}</div>;
  }

  const healthItems = [
    { label: t("setup.dashboard.health.database"), ok: summary.health.databaseConnected },
    { label: t("setup.dashboard.health.storage"), ok: summary.health.storageWritable },
    { label: t("setup.dashboard.health.setup"), ok: summary.health.setupCompleted },
    { label: t("setup.dashboard.health.authenticated"), ok: summary.health.authenticated }
  ];

  return (
    <div className="dashboard-grid">
      <section className="admin-hero">
        <div>
          <span>{t("setup.dashboard.overview")}</span>
          <h1>{summary.instance.name}</h1>
          <p>
            {t("setup.dashboard.role")} {t(`setup.roles.${summary.instance.role}`)} · {t("setup.dashboard.environment")}{" "}
            {t(`setup.environment.${summary.instance.environment}`)}
          </p>
        </div>
        <div className="admin-hero__meta">
          <strong>{summary.instance.version}</strong>
          <span>{t("setup.dashboard.version")}</span>
        </div>
      </section>

      <MetricCard
        icon={<Server size={20} />}
        label={t("setup.dashboard.cards.instance")}
        value={formatDuration(summary.instance.uptimeSeconds, t)}
        detail={t("setup.dashboard.uptime")}
      />
      <MetricCard
        icon={<HardDrive size={20} />}
        label={t("setup.dashboard.cards.storage")}
        value={formatPercent(summary.storage.usedPercent, t)}
        detail={summary.storage.path}
      />
      <MetricCard
        icon={<Boxes size={20} />}
        label={t("setup.dashboard.cards.buckets")}
        value={String(summary.objects.totalBuckets)}
        detail={t("setup.dashboard.totalBuckets")}
      />
      <MetricCard
        icon={<Database size={20} />}
        label={t("setup.dashboard.cards.objects")}
        value={String(summary.objects.totalObjects)}
        detail={formatBytes(summary.objects.totalObjectBytes, t)}
      />
      <MetricCard
        icon={<Cpu size={20} />}
        label={t("setup.dashboard.cards.resources")}
        value={formatPercent(summary.resources.cpuUsagePercent, t)}
        detail={`${t("setup.dashboard.memory")} ${formatPercent(summary.resources.memoryUsagePercent, t)}`}
      />
      <MetricCard
        icon={<ShieldCheck size={20} />}
        label={t("setup.dashboard.cards.health")}
        value={`${healthItems.filter((item) => item.ok).length}/${healthItems.length}`}
        detail={t("setup.dashboard.realChecks")}
      />

      <section className="admin-panel admin-panel--wide">
        <div className="admin-panel__header">
          <div>
            <h2>{t("setup.dashboard.storage.title")}</h2>
            <p>{summary.storage.path}</p>
          </div>
        </div>
        <UsageBar value={summary.storage.usedPercent} />
        <div className="detail-grid">
          <Detail label={t("setup.dashboard.storage.used")} value={formatBytes(summary.storage.usedBytes, t)} />
          <Detail label={t("setup.dashboard.storage.available")} value={formatBytes(summary.storage.availableBytes, t)} />
          <Detail label={t("setup.dashboard.storage.total")} value={formatBytes(summary.storage.totalBytes, t)} />
          <Detail label={t("setup.dashboard.storage.writable")} value={summary.storage.writable ? t("setup.common.yes") : t("setup.common.no")} />
        </div>
      </section>

      <section className="admin-panel">
        <div className="admin-panel__header">
          <div>
            <h2>{t("setup.dashboard.buckets.title")}</h2>
            <p>{t("setup.dashboard.buckets.description")}</p>
          </div>
          <Link className="admin-link-button" to="/buckets">{t("setup.dashboard.buckets.open")}</Link>
        </div>
        <div className="detail-grid detail-grid--single">
          <Detail label={t("setup.dashboard.totalBuckets")} value={String(summary.objects.totalBuckets)} />
          <Detail label={t("setup.dashboard.totalObjects")} value={String(summary.objects.totalObjects)} />
          <Detail label={t("setup.dashboard.totalBytes")} value={formatBytes(summary.objects.totalObjectBytes, t)} />
        </div>
      </section>

      <section className="admin-panel">
        <div className="admin-panel__header">
          <div>
            <h2>{t("setup.dashboard.health.title")}</h2>
            <p>{t("setup.dashboard.health.description")}</p>
          </div>
        </div>
        <div className="health-list">
          {healthItems.map((item) => (
            <div className="health-list__item" key={item.label}>
              <CheckCircle2 size={17} aria-hidden="true" data-ok={item.ok} />
              <span>{item.label}</span>
              <strong>{item.ok ? t("setup.common.ok") : t("setup.common.failed")}</strong>
            </div>
          ))}
        </div>
      </section>

      <Warnings warnings={[...summary.storage.warnings, ...summary.resources.warnings]} />
    </div>
  );
}

function MetricCard({ icon, label, value, detail }: {
  icon: ReactNode;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <section className="metric-card">
      <div className="metric-card__icon">{icon}</div>
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </section>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-item">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function UsageBar({ value }: { value: number | null }) {
  const width = value === null ? 0 : Math.max(0, Math.min(100, value));
  return (
    <div className="usage-bar" aria-hidden="true">
      <span style={{ width: `${width}%` }} />
    </div>
  );
}

function Warnings({ warnings }: { warnings: string[] }) {
  const { t } = useTranslation();
  if (warnings.length === 0) {
    return null;
  }
  return (
    <section className="admin-panel admin-panel--wide">
      <h2>{t("setup.dashboard.warnings")}</h2>
      <ul className="warning-list">
        {warnings.map((warning) => <li key={warning}>{warning}</li>)}
      </ul>
    </section>
  );
}

function formatBytes(value: number | null, t: (key: string) => string): string {
  if (value === null) {
    return t("setup.common.unavailable");
  }
  if (value === 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${(value / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

function formatPercent(value: number | null, t: (key: string) => string): string {
  return value === null ? t("setup.common.unavailable") : `${value.toFixed(1)}%`;
}

function formatDuration(seconds: number, t: (key: string) => string): string {
  if (seconds < 60) {
    return t("setup.dashboard.duration.seconds").replace("{{count}}", String(seconds));
  }
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return t("setup.dashboard.duration.minutes").replace("{{count}}", String(minutes));
  }
  const hours = Math.floor(minutes / 60);
  return t("setup.dashboard.duration.hours").replace("{{count}}", String(hours));
}
