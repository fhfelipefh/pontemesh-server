import { FormEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { CompleteSetupRequest, completeSetup } from "../api/setupApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";
import { PageShell } from "../components/PageShell";
import { TextInput } from "../components/TextInput";

export function ConfigurePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [instanceName, setInstanceName] = useState("Ponte Mesh Local");
  const [role, setRole] = useState<CompleteSetupRequest["role"]>("origin");
  const [adminUsername, setAdminUsername] = useState("admin");
  const [adminPassword, setAdminPassword] = useState("");
  const [httpPort, setHttpPort] = useState("8080");
  const [storageLocalPath, setStorageLocalPath] = useState("");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);

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

    if (storageLocalPath.trim()) {
      payload.storageLocalPath = storageLocalPath.trim();
    }

    try {
      await completeSetup(payload);
      navigate("/");
    } catch (setupError) {
      setError(setupError instanceof Error ? setupError.message : t("setup.configure.setupFailed"));
    } finally {
      setSubmitting(false);
    }
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
        <TextInput
          id="storageLocalPath"
          label={t("setup.configure.storageLocalPath")}
          value={storageLocalPath}
          onChange={setStorageLocalPath}
          placeholder={t("setup.configure.storagePlaceholder")}
        />
        <div className="form__footer">
          <ErrorMessage message={error} />
          <Button type="submit" loading={submitting}>
            {submitting ? t("setup.configure.finishing") : t("setup.configure.finish")}
          </Button>
        </div>
      </form>
    </PageShell>
  );
}
