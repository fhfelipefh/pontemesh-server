import { Eye, EyeOff, LockKeyhole } from "lucide-react";
import { InputHTMLAttributes, useState } from "react";
import { useTranslation } from "react-i18next";
import { ErrorMessage } from "./ErrorMessage";

type TextInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> & {
  label: string;
  onChange: (value: string) => void;
  error?: string;
  revealable?: boolean;
};

export function TextInput({
  id,
  label,
  onChange,
  error = "",
  type = "text",
  revealable = false,
  ...props
}: TextInputProps) {
  const { t } = useTranslation();
  const [revealed, setRevealed] = useState(false);
  const isPassword = type === "password";
  const inputType = isPassword && revealed ? "text" : type;
  const errorId = error && id ? `${id}-error` : undefined;

  return (
    <label className="field" htmlFor={id}>
      <span>{label}</span>
      <span className={isPassword ? "input-wrap input-wrap--with-icon" : "input-wrap"}>
        {isPassword ? <LockKeyhole className="input-wrap__icon" size={18} aria-hidden="true" /> : null}
        <input
          id={id}
          type={inputType}
          aria-invalid={Boolean(error)}
          aria-describedby={errorId}
          onChange={(event) => onChange(event.target.value)}
          {...props}
        />
        {isPassword && revealable ? (
          <button
            className="input-wrap__reveal"
            type="button"
            aria-label={revealed ? t("setup.unlock.hideToken") : t("setup.unlock.showToken")}
            onClick={() => setRevealed((currentRevealed) => !currentRevealed)}
          >
            {revealed ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}
          </button>
        ) : null}
      </span>
      <ErrorMessage id={errorId} message={error} />
    </label>
  );
}
