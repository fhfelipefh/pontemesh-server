import "../styles/replicas-modern.css";
import { FormEvent, useCallback, useEffect, useState } from "react";
import { Ban, Plus, ServerCog } from "lucide-react";
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
    <div className="replicas-modern-page">
      <header className="replicas-modern__header">
        <h1>{t("setup.replicas.title")}</h1>
        <p>{t("setup.replicas.description")}</p>
      </header>
      <section className="replicas-modern-stats" aria-label={t("setup.replicas.summaryLabel")}>
        <div className="rm-stat">
          <span>{t("setup.replicas.total")}</span>
          <strong>{replicas.length}</strong>
        </div>
        <div className="rm-stat" data-tone="success">
          <span>{t("setup.replicas.active")}</span>
          <strong>{replicas.filter((replica) => !replica.revoked).length}</strong>
        </div>
        <div className="rm-stat">
          <span>{t("setup.replicas.availableObjects")}</span>
          <strong>{replicas.reduce((total, replica) => total + replica.availableObjects, 0)}</strong>
        </div>
      </section>

      <div className="replicas-modern-layout">
        <section className="rm-panel">
          <div className="rm-panel__header">
            <h2>{t("setup.replicas.credentials")}</h2>
            <p>{t("setup.replicas.credentialsDescription")}</p>
          </div>
          <div className="rm-panel__body">
            <form className="rm-form" onSubmit={handleSubmit}>
              <div className="rm-form-group">
                <label>{t("setup.replicas.name")}</label>
                <input value={name} onChange={(event) => setName(event.target.value)} placeholder={t("setup.replicas.namePlaceholder")} aria-label={t("setup.replicas.name")} />
              </div>
              <div className="rm-form-group">
                <label>{t("setup.replicas.allowedBuckets")}</label>
                <input value={allowedBuckets} onChange={(event) => setAllowedBuckets(event.target.value)} placeholder={t("setup.replicas.allowedBucketsPlaceholder")} aria-label={t("setup.replicas.allowedBuckets")} />
                <span className="rm-form-hint">{t("setup.replicas.allowedBucketsHint")}</span>
              </div>
              <div className="rm-form-submit">
                <Button type="submit" loading={submitting} disabled={!name.trim() || !allowedBuckets.trim()} icon={<Plus size={17} aria-hidden="true" />}>
                  {t("setup.replicas.create")}
                </Button>
              </div>
            </form>
            <ErrorMessage message={error} />
            {created ? <CreatedReplicaToken created={created} /> : null}
          </div>
        </section>

        <section className="rm-panel">
          <div className="rm-panel__header">
            <h2>{t("setup.replicas.registeredTitle")}</h2>
            <p>{t("setup.replicas.registeredDescription")}</p>
          </div>
          <div className="rm-panel__body" style={{ padding: 0 }}>
            {loading ? (
              <div className="settings-loading" style={{ padding: 24 }}>{t("setup.common.loading")}</div>
            ) : replicas.length === 0 ? (
              <EmptyState icon={<ServerCog size={22} />} title={t("setup.replicas.emptyTitle")} description={t("setup.replicas.emptyDescription")} />
            ) : (
              <div className="rm-list">
                {replicas.map((replica) => (
                  <article className="rm-list-item" data-revoked={replica.revoked} key={replica.id}>
                    <div className="rm-list-item__header">
                      <div className="rm-list-item__title">
                        <h3>{replica.name}</h3>
                        <code>{replica.id}</code>
                      </div>
                      <StatusBadge active={!replica.revoked} activeLabel={t("setup.settings.s3.active")} revokedLabel={t("setup.settings.s3.revoked")} />
                    </div>
                    <div className="rm-list-item__metrics">
                      <div className="rm-metric">
                        <span>{t("setup.replicas.availableObjects")}</span>
                        <strong>{replica.availableObjects}</strong>
                      </div>
                      <div className="rm-metric">
                        <span>{t("setup.replicas.lastSeenAt")}</span>
                        <strong>{replica.lastSeenAt ? formatDate(replica.lastSeenAt, i18n.language) : t("setup.replicas.neverSeen")}</strong>
                      </div>
                      <div className="rm-metric">
                        <span>{t("setup.replicas.createdAt")}</span>
                        <strong>{formatDate(replica.createdAt, i18n.language)}</strong>
                      </div>
                    </div>
                    <div className="rm-list-item__footer">
                      <div className="rm-tags" aria-label={t("setup.replicas.allowedBuckets")}>
                        {replica.allowedBuckets.map((bucket) => <span key={bucket}>{bucket}</span>)}
                      </div>
                      {!replica.revoked ? (
                        <button className="rm-revoke-btn" type="button" title={t("setup.replicas.revoke")} aria-label={t("setup.replicas.revoke")} disabled={revoking === replica.id} onClick={() => setRevokeConfirmation({ id: replica.id, name: replica.name })}>
                          <Ban size={16} aria-hidden="true" />
                        </button>
                      ) : null}
                    </div>
                  </article>
                ))}
              </div>
            )}
          </div>
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
