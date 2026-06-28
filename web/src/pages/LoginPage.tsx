import { FormEvent, useState } from "react";
import { LogIn } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AuthUser, login } from "../api/authApi";
import { Button } from "../components/Button";
import { ErrorMessage } from "../components/ErrorMessage";
import { PageShell } from "../components/PageShell";
import { TextInput } from "../components/TextInput";

type LoginPageProps = {
  onAuthenticated: (user: AuthUser) => void;
};

export function LoginPage({ onAuthenticated }: LoginPageProps) {
  const { t } = useTranslation();
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setSubmitting(true);
    try {
      const user = await login({ username, password });
      onAuthenticated(user);
    } catch (loginError) {
      setError(loginError instanceof Error ? loginError.message : t("setup.auth.loginFailed"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <PageShell title={t("setup.auth.loginTitle")} description={t("setup.auth.loginDescription")}>
      <form className="form" onSubmit={handleSubmit}>
        <TextInput
          id="loginUsername"
          label={t("setup.auth.username")}
          autoComplete="username"
          value={username}
          onChange={setUsername}
          required
          autoFocus
        />
        <TextInput
          id="loginPassword"
          label={t("setup.auth.password")}
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={setPassword}
          revealable
          required
        />
        <ErrorMessage message={error} />
        <Button
          type="submit"
          loading={submitting}
          disabled={!username.trim() || !password}
          icon={<LogIn size={18} aria-hidden="true" />}
        >
          {submitting ? t("setup.auth.signingIn") : t("setup.auth.signIn")}
        </Button>
      </form>
    </PageShell>
  );
}
