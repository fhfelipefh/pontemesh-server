import { FormEvent, ReactNode, useCallback, useEffect, useState } from "react";
import { Activity, Ban, Boxes, KeyRound, Plus, ServerCog, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  CreatedReplicaCredential,
  ReplicaSummary,
  createReplicaCredential,
  listReplicas,
  revokeReplica
} from "../api/replicasApi";
import { Button } from "../components/Button";
import { ConfirmDialog } from "../components/AdminListControls";
import { ErrorMessage } from "../components/ErrorMessage";
import { CopyButton } from "../components/settings/CopyButton";
import { EmptyState } from "../components/settings/EmptyState";
import { StatusBadge } from "../components/settings/StatusBadge";

type RevokeConfirmation = { id: string; name: string } | null;

export function ReplicasPage() {
  const { t, i18n } = useTranslation();
  const [replicas, setReplicas] = useState<ReplicaSummary[]>([]);
  const [created, setCreated] = useState<CreatedReplicaCredential | null>(null);
  const [name, setName] = useState("");
  const [allowedBuckets, setAllowedBuckets] = useState("");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [revoking, setRevoking] = useState<string | null>(null);
  const [revokeConfirmation, setRevokeConfirmation] = useState<RevokeConfirmation>(null);
  const [error, setError] = useState("");

  const refreshReplicas = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setReplicas(await listReplicas());
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : t("setup.replicas.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refreshReplicas();
  }, [refreshReplicas]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const buckets = allowedBuckets.split(",").map((bucket) => bucket.trim()).filter(Boolean);
    if (!name.trim() || buckets.length === 0) {
      return;
    }
    setSubmitting(true);
    setError("");
    try {
      setCreated(await createReplicaCredential(name.trim(), buckets));
      setName("");
      setAllowedBuckets("");
      await refreshReplicas();
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : t("setup.replicas.createFailed"));
    } finally {
      setSubmitting(false);
    }
  }

  async function handleRevoke(replicaId: string) {
    setRevoking(replicaId);
    setError("");
    try {
      await revokeReplica(replicaId);
      setRevokeConfirmation(null);
      await refreshReplicas();
    } catch (revokeError) {
      setError(revokeError instanceof Error ? revokeError.message : t("setup.replicas.revokeFailed"));
    } finally {
      setRevoking(null);
    }
  }

  return (
    <div className="settings-page replicas-page">
      <header className="settings-page__header replicas-page__header">
        <div>
          <h1>{t("setup.replicas.title")}</h1>
          <p>{t("setup.replicas.description")}</p>
        </div>
        <div className="replicas-page__header-icon" aria-hidden="true"><ServerCog size={28} /></div>
      </header>
      <section className="replica-summary-grid" aria-label={t("setup.replicas.summaryLabel")}>
        <ReplicaStat icon={<ServerCog size={18} />} label={t("setup.replicas.total")} value={String(replicas.length)} />
        <ReplicaStat icon={<ShieldCheck size={18} />} label={t("setup.replicas.active")} value={String(replicas.filter((replica) => !replica.revoked).length)} tone="success" />
        <ReplicaStat icon={<Boxes size={18} />} label={t("setup.replicas.availableObjects")} value={String(replicas.reduce((total, replica) => total + replica.availableObjects, 0))} />
      </section>

      <div className="replicas-page__layout">
        <section className="settings-card replica-create-card">
          <div className="settings-card__header">
            <div className="settings-card__title-group">
              <div className="settings-card__title-icon"><KeyRound size={20} aria-hidden="true" /></div>
              <div>
                <h2>{t("setup.replicas.credentials")}</h2>
                <p>{t("setup.replicas.credentialsDescription")}</p>
              </div>
            </div>
          </div>
          <form className="replica-create-form" onSubmit={handleSubmit}>
            <label>
              <span>{t("setup.replicas.name")}</span>
              <input value={name} onChange={(event) => setName(event.target.value)} placeholder={t("setup.replicas.namePlaceholder")} aria-label={t("setup.replicas.name")} />
            </label>
            <label>
              <span>{t("setup.replicas.allowedBuckets")}</span>
              <input value={allowedBuckets} onChange={(event) => setAllowedBuckets(event.target.value)} placeholder={t("setup.replicas.allowedBucketsPlaceholder")} aria-label={t("setup.replicas.allowedBuckets")} />
            </label>
            <p className="replica-create-form__hint">{t("setup.replicas.allowedBucketsHint")}</p>
            <Button type="submit" loading={submitting} disabled={!name.trim() || !allowedBuckets.trim()} icon={<Plus size={17} aria-hidden="true" />}>
              {t("setup.replicas.create")}
            </Button>
          </form>
          <ErrorMessage message={error} />
          {created ? <CreatedReplicaToken created={created} /> : null}
        </section>

        <section className="settings-card replica-list-card">
          <div className="settings-card__header">
            <div className="settings-card__title-group">
              <div className="settings-card__title-icon"><Activity size={20} aria-hidden="true" /></div>
              <div>
                <h2>{t("setup.replicas.registeredTitle")}</h2>
                <p>{t("setup.replicas.registeredDescription")}</p>
              </div>
            </div>
          </div>
          {loading ? (
            <div className="settings-loading">{t("setup.common.loading")}</div>
          ) : replicas.length === 0 ? (
            <EmptyState icon={<ServerCog size={22} />} title={t("setup.replicas.emptyTitle")} description={t("setup.replicas.emptyDescription")} />
          ) : (
            <div className="replica-list">
              {replicas.map((replica) => (
                <article className="replica-card" data-revoked={replica.revoked} key={replica.id}>
                  <div className="replica-card__heading">
                    <div className="replica-card__identity">
                      <span className="replica-card__icon" aria-hidden="true"><ServerCog size={18} /></span>
                      <div>
                        <h3>{replica.name}</h3>
                        <code>{replica.id}</code>
                      </div>
                    </div>
                    <StatusBadge active={!replica.revoked} activeLabel={t("setup.settings.s3.active")} revokedLabel={t("setup.settings.s3.revoked")} />
                  </div>
                  <div className="replica-card__metrics">
                    <ReplicaDetail label={t("setup.replicas.availableObjects")} value={String(replica.availableObjects)} />
                    <ReplicaDetail label={t("setup.replicas.lastSeenAt")} value={replica.lastSeenAt ? formatDate(replica.lastSeenAt, i18n.language) : t("setup.replicas.neverSeen")} />
                    <ReplicaDetail label={t("setup.replicas.createdAt")} value={formatDate(replica.createdAt, i18n.language)} />
                  </div>
                  <div className="replica-card__footer">
                    <div className="replica-bucket-tags" aria-label={t("setup.replicas.allowedBuckets")}>
                      {replica.allowedBuckets.map((bucket) => <span key={bucket}>{bucket}</span>)}
                    </div>
                    {!replica.revoked ? (
                      <button className="settings-revoke-button" type="button" title={t("setup.replicas.revoke")} aria-label={t("setup.replicas.revoke")} disabled={revoking === replica.id} onClick={() => setRevokeConfirmation({ id: replica.id, name: replica.name })}>
                        <Ban size={16} aria-hidden="true" />
                      </button>
                    ) : null}
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      </div>
      {revokeConfirmation ? (
        <ConfirmDialog
          title={t("setup.replicas.confirmRevokeTitle")}
          description={t("setup.replicas.confirmRevokeDescription", { name: revokeConfirmation.name })}
          onCancel={() => setRevokeConfirmation(null)}
          onConfirm={() => void handleRevoke(revokeConfirmation.id)}
        />
      ) : null}
    </div>
  );
}

function ReplicaStat({ icon, label, value, tone = "default" }: { icon: ReactNode; label: string; value: string; tone?: "default" | "success" }) {
  return <div className="replica-summary" data-tone={tone}>{icon}<span>{label}</span><strong>{value}</strong></div>;
}

function ReplicaDetail({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong>{value}</strong></div>;
}

function CreatedReplicaToken({ created }: { created: CreatedReplicaCredential }) {
  const { t } = useTranslation();
  return (
    <section className="secret-panel" role="status">
      <strong>{t("setup.replicas.createdTitle")}</strong>
      <dl>
        <div><dt>{t("setup.replicas.replicaId")}</dt><dd><code>{created.replica.id}</code></dd></div>
        <div><dt>{t("setup.replicas.token")}</dt><dd><code>{created.token}</code><CopyButton value={created.token} label={t("setup.replicas.copyToken")} /></dd></div>
      </dl>
    </section>
  );
}

function formatDate(value: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "short"
  }).format(new Date(value));
}
