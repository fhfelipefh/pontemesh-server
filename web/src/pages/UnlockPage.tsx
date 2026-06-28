import { FormEvent, useState } from "react";
import { unlockSetup } from "../api/setupApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";
import { PageShell } from "../components/PageShell";
import { TextInput } from "../components/TextInput";

type UnlockPageProps = {
  onUnlocked: () => void;
};

export function UnlockPage({ onUnlocked }: UnlockPageProps) {
  const [token, setToken] = useState("");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setSubmitting(true);

    try {
      await unlockSetup({ token });
      onUnlocked();
    } catch {
      setError("Invalid initial admin token.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <PageShell
      title="Unlock Ponte Mesh"
      description="This Ponte Mesh instance needs the initial admin token before it can be configured."
    >
      <form className="form" onSubmit={handleSubmit}>
        <TextInput
          id="token"
          label="Initial admin token"
          type="password"
          autoComplete="one-time-code"
          value={token}
          onChange={setToken}
          required
          autoFocus
        />
        <ErrorMessage message={error} />
        <Button type="submit" disabled={submitting}>
          {submitting ? "Checking..." : "Continue"}
        </Button>
      </form>
    </PageShell>
  );
}
