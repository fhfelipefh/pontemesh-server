import { FormEvent, useState } from "react";
import { CheckCircle2, ChevronDown, Clipboard, KeyRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { CompleteSetupRequest, CompleteSetupResponse, completeSetup } from "../api/setupApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";
import { PageShell } from "../components/PageShell";
import { TextInput } from "../components/TextInput";

const DEFAULT_STORAGE_PATH = "/var/pontemesh_home/data/storage";

export function ConfigurePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [instanceName, setInstanceName] = useState("Ponte Mesh Local");
  const [role, setRole] = useState<CompleteSetupRequest["role"]>("origin");
  const [adminUsername, setAdminUsername] = useState("admin");
  const [adminPassword, setAdminPassword] = useState("");
  const [httpPort, setHttpPort] = useState("8080");
  const [internalStoragePath, setInternalStoragePath] = useState("");
  const [originBaseUrl, setOriginBaseUrl] = useState("");
  const [replicaId, setReplicaId] = useState("");
  const [replicaToken, setReplicaToken] = useState("");
  const [replicaPublicEndpoint, setReplicaPublicEndpoint] = useState("");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [setupResult, setSetupResult] = useState<CompleteSetupResponse | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setSubmitting(true);

    const payload: CompleteSetupRequest = {
      instanceName,
      role,
      adminUsername,
      adminPassword,
      httpPort: Number(httpPort)
    };

    if (internalStoragePath.trim()) {
      payload.internalStoragePath = internalStoragePath.trim();
    }

    if (role === "replica-edge") {
      payload.originBaseUrl = originBaseUrl.trim();
      payload.replicaId = replicaId.trim();
      payload.replicaToken = replicaToken.trim();
      payload.replicaPublicEndpoint = replicaPublicEndpoint.trim();
    }

    try {
      setSetupResult(await completeSetup(payload));
    } catch (setupError) {
      setError(setupError instanceof Error ? setupError.message : t("setup.configure.setupFailed"));
    } finally {
      setSubmitting(false);
    }
  }

  if (setupResult?.initialS3AccessKey) {
    return (
      <PageShell
        title={t("setup.configure.s3InitialTitle")}
        description={t("setup.configure.s3InitialDescription")}
        compact
      >
        <div className="secret-panel" role="status">
          <div>
            <KeyRound size={18} aria-hidden="true" />
            <strong>{t("setup.configure.s3InitialWarning")}</strong>
          </div>
          <dl>
            <div>
              <dt>{t("setup.settings.s3.accessKeyId")}</dt>
              <dd>
                <code>{setupResult.initialS3AccessKey.accessKeyId}</code>
                <CopyButton value={setupResult.initialS3AccessKey.accessKeyId} label={t("setup.settings.s3.copyAccessKeyId")} />
              </dd>
            </div>
            <div>
              <dt>{t("setup.settings.s3.secretAccessKey")}</dt>
              <dd>
                <code>{setupResult.initialS3AccessKey.secretAccessKey}</code>
                <CopyButton value={setupResult.initialS3AccessKey.secretAccessKey} label={t("setup.settings.s3.copySecretAccessKey")} />
              </dd>
            </div>
          </dl>
          <p>{t("setup.settings.s3.createdHint")}</p>
        </div>
        <div className="form__footer">
          <Button type="button" onClick={() => navigate("/login")} icon={<CheckCircle2 size={18} aria-hidden="true" />}>
            {t("setup.configure.goToLogin")}
          </Button>
        </div>
      </PageShell>
    );
  }

  if (setupResult) {
    return (
      <PageShell
        title={t("setup.configure.replicaReadyTitle")}
        description={t("setup.configure.replicaReadyDescription")}
        compact
      >
        <div className="form__footer">
          <Button type="button" onClick={() => navigate("/login")} icon={<CheckCircle2 size={18} aria-hidden="true" />}>
            {t("setup.configure.goToLogin")}
          </Button>
        </div>
      </PageShell>
    );
  }

  return (
    <PageShell
      title={t("setup.configure.title")}
      description={t("setup.configure.description")}
      compact
    >
      <form className="form form--two-column" onSubmit={handleSubmit}>
        <TextInput
          id="instanceName"
          label={t("setup.configure.instanceName")}
          value={instanceName}
          onChange={setInstanceName}
          required
        />

        <label className="field" htmlFor="role">
          <span>{t("setup.configure.role")}</span>
          <span className="input-wrap">
            <select
              id="role"
              value={role}
              onChange={(event) => setRole(event.target.value as CompleteSetupRequest["role"])}
            >
              <option value="origin">{t("setup.configure.origin")}</option>
              <option value="replica-edge">{t("setup.configure.replicaEdge")}</option>
            </select>
          </span>
        </label>

        <TextInput
          id="adminUsername"
          label={t("setup.configure.adminUsername")}
          autoComplete="username"
          value={adminUsername}
          onChange={setAdminUsername}
          required
        />
        <TextInput
          id="adminPassword"
          label={t("setup.configure.adminPassword")}
          type="password"
          autoComplete="new-password"
          value={adminPassword}
          onChange={setAdminPassword}
          minLength={8}
          revealable
          required
        />
        <TextInput
          id="httpPort"
          label={t("setup.configure.httpPort")}
          type="number"
          min={1}
          max={65535}
          value={httpPort}
          onChange={setHttpPort}
          required
        />

        {role === "replica-edge" ? (
          <section className="storage-summary" aria-labelledby="replica-config-title">
            <div>
              <h2 id="replica-config-title">{t("setup.configure.replica.title")}</h2>
              <p>{t("setup.configure.replica.description")}</p>
            </div>
            <TextInput
              id="originBaseUrl"
              label={t("setup.configure.replica.originBaseUrl")}
              value={originBaseUrl}
              onChange={setOriginBaseUrl}
              placeholder="https://origin.example.com"
              required
            />
            <TextInput
              id="replicaPublicEndpoint"
              label={t("setup.configure.replica.publicEndpoint")}
              value={replicaPublicEndpoint}
              onChange={setReplicaPublicEndpoint}
              placeholder="https://edge.example.com"
              required
            />
            <TextInput
              id="replicaId"
              label={t("setup.configure.replica.replicaId")}
              value={replicaId}
              onChange={setReplicaId}
              required
            />
            <TextInput
              id="replicaToken"
              label={t("setup.configure.replica.replicaToken")}
              type="password"
              value={replicaToken}
              onChange={setReplicaToken}
              revealable
              required
            />
          </section>
        ) : null}

        <section className="storage-summary" aria-labelledby="storage-summary-title">
          <div>
            <h2 id="storage-summary-title">{t("setup.configure.storage.title")}</h2>
            <p>{t("setup.configure.storage.description")}</p>
          </div>
          <div className="storage-summary__path">
            <span>{t("setup.configure.storage.defaultPathLabel")}</span>
            <code>{DEFAULT_STORAGE_PATH}</code>
          </div>
        </section>

        <section className="advanced-storage">
          <button
            className="advanced-storage__toggle"
            type="button"
            aria-expanded={advancedOpen}
            aria-controls="advanced-storage-panel"
            onClick={() => setAdvancedOpen((current) => !current)}
          >
            <span>{t("setup.configure.storage.advancedTitle")}</span>
            <ChevronDown size={18} aria-hidden="true" />
          </button>

          {advancedOpen ? (
            <div id="advanced-storage-panel" className="advanced-storage__panel">
              <p>{t("setup.configure.storage.advancedDescription")}</p>
              <TextInput
                id="internalStoragePath"
                label={t("setup.configure.storage.internalPathLabel")}
                value={internalStoragePath}
                onChange={setInternalStoragePath}
                placeholder={t("setup.configure.storage.internalPathPlaceholder")}
              />
              <p className="advanced-storage__help">
                {t("setup.configure.storage.internalPathHelp")}
              </p>
            </div>
          ) : null}

        </section>
        <div className="form__footer">
          <ErrorMessage message={error} />
          <Button type="submit" loading={submitting} icon={<CheckCircle2 size={18} aria-hidden="true" />}>
            {submitting ? t("setup.configure.finishing") : t("setup.configure.finish")}
          </Button>
        </div>
      </form>
    </PageShell>
  );
}

function CopyButton({ value, label }: { value: string; label: string }) {
  async function handleCopy() {
    await navigator.clipboard?.writeText(value);
  }

  return (
    <button className="icon-button" type="button" title={label} aria-label={label} onClick={handleCopy}>
      <Clipboard size={16} aria-hidden="true" />
    </button>
  );
}
