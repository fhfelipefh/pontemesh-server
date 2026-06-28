import { FormEvent, useState } from "react";
import { CompleteSetupRequest, completeSetup } from "../api/setupApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";
import { PageShell } from "../components/PageShell";
import { TextInput } from "../components/TextInput";

type ConfigurePageProps = {
  onComplete: () => void;
};

export function ConfigurePage({ onComplete }: ConfigurePageProps) {
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
      onComplete();
    } catch (setupError) {
      setError(setupError instanceof Error ? setupError.message : "Setup failed.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <PageShell
      title="Configure Ponte Mesh"
      description="Create the first administrator and choose how this instance will operate."
    >
      <form className="form" onSubmit={handleSubmit}>
        <TextInput
          id="instanceName"
          label="Instance name"
          value={instanceName}
          onChange={setInstanceName}
          required
        />

        <label className="field" htmlFor="role">
          <span>Instance role</span>
          <select
            id="role"
            value={role}
            onChange={(event) => setRole(event.target.value as CompleteSetupRequest["role"])}
          >
            <option value="origin">origin</option>
            <option value="replica-edge">replica-edge</option>
          </select>
        </label>

        <TextInput
          id="adminUsername"
          label="Initial admin username"
          autoComplete="username"
          value={adminUsername}
          onChange={setAdminUsername}
          required
        />
        <TextInput
          id="adminPassword"
          label="Initial admin password"
          type="password"
          autoComplete="new-password"
          value={adminPassword}
          onChange={setAdminPassword}
          minLength={8}
          required
        />
        <TextInput
          id="httpPort"
          label="HTTP port"
          type="number"
          min={1}
          max={65535}
          value={httpPort}
          onChange={setHttpPort}
          required
        />
        <TextInput
          id="storageLocalPath"
          label="Local storage path"
          value={storageLocalPath}
          onChange={setStorageLocalPath}
          placeholder="Use backend default"
        />
        <ErrorMessage message={error} />
        <Button type="submit" disabled={submitting}>
          {submitting ? "Finishing..." : "Finish setup"}
        </Button>
      </form>
    </PageShell>
  );
}
