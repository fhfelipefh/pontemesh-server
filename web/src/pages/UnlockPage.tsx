import { FormEvent, useState } from "react";
import { UnlockKeyhole } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { unlockSetup } from "../api/setupApi";
import { Button } from "../components/Button";
import { PageShell } from "../components/PageShell";
import { TextInput } from "../components/TextInput";

export function UnlockPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [token, setToken] = useState("");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setSubmitting(true);

    try {
      await unlockSetup({ token });
      navigate("/setup/configure");
    } catch {
      setError(t("setup.unlock.invalidToken"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <PageShell title={t("setup.unlock.title")} description={t("setup.unlock.description")}>
      <form className="form" onSubmit={handleSubmit}>
        <TextInput
          id="token"
          label={t("setup.unlock.tokenLabel")}
          type="password"
          autoComplete="one-time-code"
          placeholder={t("setup.unlock.tokenPlaceholder")}
          value={token}
          onChange={setToken}
          error={error}
          revealable
          required
          autoFocus
        />
        <Button type="submit" disabled={!token.trim()} loading={submitting} icon={<UnlockKeyhole size={18} aria-hidden="true" />}>
          {submitting ? t("setup.unlock.checking") : t("setup.unlock.continue")}
        </Button>
        <p className="language-hint">{t("setup.unlock.languageHint")}</p>
      </form>
    </PageShell>
  );
}
