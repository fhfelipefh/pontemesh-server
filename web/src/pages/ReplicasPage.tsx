import { FormEvent, useCallback, useEffect, useState } from "react";
import { Ban, Copy, Plus, ServerCog } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  CreatedReplicaCredential,
  ReplicaSummary,
  createReplicaCredential,
  listReplicas,
  revokeReplica
} from "../api/replicasApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";

export function ReplicasPage() {
  const { t, i18n } = useTranslation();
  const [replicas, setReplicas] = useState<ReplicaSummary[]>([]);
  const [created, setCreated] = useState<CreatedReplicaCredential | null>(null);
  const [name, setName] = useState("");
  const [allowedBuckets, setAllowedBuckets] = useState("");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [revoking, setRevoking] = useState<string | null>(null);
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
      await refreshReplicas();
    } catch (revokeError) {
      setError(revokeError instanceof Error ? revokeError.message : t("setup.replicas.revokeFailed"));
    } finally {
      setRevoking(null);
    }
  }

  return (
    <div className="settings-page">
      <header className="settings-page__header">
        <div>
          <h1>{t("setup.replicas.title")}</h1>
        </div>
      </header>
      <section className="settings-card">
        <div className="settings-card__header">
          <div className="settings-card__title-group">
            <div className="settings-card__title-icon">
              <ServerCog size={20} aria-hidden="true" />
            </div>
            <div>
              <h2>{t("setup.replicas.credentials")}</h2>
            </div>
          </div>
        </div>
        <form className="inline-form" onSubmit={handleSubmit}>
          <input value={name} onChange={(event) => setName(event.target.value)} placeholder={t("setup.replicas.namePlaceholder")} aria-label={t("setup.replicas.name")} />
          <input value={allowedBuckets} onChange={(event) => setAllowedBuckets(event.target.value)} placeholder={t("setup.replicas.allowedBucketsPlaceholder")} aria-label={t("setup.replicas.allowedBuckets")} />
          <Button type="submit" loading={submitting} disabled={!name.trim() || !allowedBuckets.trim()} icon={<Plus size={17} aria-hidden="true" />}>
            {t("setup.replicas.create")}
          </Button>
        </form>
        <ErrorMessage message={error} />
        {created ? (
          <section className="secret-panel" role="status">
            <strong>{t("setup.replicas.createdTitle")}</strong>
            <dl>
              <div>
                <dt>{t("setup.replicas.replicaId")}</dt>
                <dd><code>{created.replica.id}</code></dd>
              </div>
              <div>
                <dt>{t("setup.replicas.token")}</dt>
                <dd>
                  <code>{created.token}</code>
                  <button className="icon-button" type="button" title={t("setup.replicas.copyToken")} aria-label={t("setup.replicas.copyToken")} onClick={() => void navigator.clipboard?.writeText(created.token)}>
                    <Copy size={16} aria-hidden="true" />
                  </button>
                </dd>
              </div>
            </dl>
          </section>
        ) : null}
        {loading ? (
          <div className="settings-loading">{t("setup.common.loading")}</div>
        ) : replicas.length === 0 ? (
          <div className="settings-empty-state">
            <h3>{t("setup.replicas.emptyTitle")}</h3>
            <p>{t("setup.replicas.emptyDescription")}</p>
          </div>
        ) : (
          <div className="settings-table-wrap">
            <table className="settings-table">
              <thead>
                <tr>
                  <th>{t("setup.replicas.name")}</th>
                  <th>{t("setup.replicas.allowedBuckets")}</th>
                  <th>{t("setup.replicas.availableObjects")}</th>
                  <th>{t("setup.replicas.lastSeenAt")}</th>
                  <th>{t("setup.settings.s3.status")}</th>
                  <th>{t("setup.settings.s3.createdAt")}</th>
                  <th aria-label={t("setup.settings.s3.actions")} />
                </tr>
              </thead>
              <tbody>
                {replicas.map((replica) => (
                  <tr key={replica.id}>
                    <td className="settings-table__name">{replica.name}</td>
                    <td>{replica.allowedBuckets.join(", ")}</td>
                    <td>{replica.availableObjects}</td>
                    <td>{replica.lastSeenAt ? formatDate(replica.lastSeenAt, i18n.language) : t("setup.replicas.neverSeen")}</td>
                    <td>{replica.revoked ? t("setup.settings.s3.revoked") : t("setup.settings.s3.active")}</td>
                    <td>{formatDate(replica.createdAt, i18n.language)}</td>
                    <td className="settings-table__actions">
                      {!replica.revoked ? (
                        <button className="settings-revoke-button" type="button" title={t("setup.replicas.revoke")} aria-label={t("setup.replicas.revoke")} disabled={revoking === replica.id} onClick={() => handleRevoke(replica.id)}>
                          <Ban size={16} aria-hidden="true" />
                        </button>
                      ) : null}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}

function formatDate(value: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "short",
    timeStyle: "short"
  }).format(new Date(value));
}
