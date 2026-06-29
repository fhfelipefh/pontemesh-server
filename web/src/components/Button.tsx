import { ReactNode } from "react";

type ButtonProps = {
  children: ReactNode;
  icon?: ReactNode;
  type?: "button" | "submit";
  disabled?: boolean;
  loading?: boolean;
  onClick?: () => void;
};

export function Button({
  children,
  icon,
  type = "button",
  disabled = false,
  loading = false,
  onClick
}: ButtonProps) {
  return (
    <button className="button" type={type} disabled={disabled || loading} aria-busy={loading} onClick={onClick}>
      {loading ? <span className="button__spinner" aria-hidden="true" /> : icon}
      <span>{children}</span>
    </button>
  );
}
