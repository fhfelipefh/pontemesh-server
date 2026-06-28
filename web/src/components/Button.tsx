import { ReactNode } from "react";

type ButtonProps = {
  children: ReactNode;
  type?: "button" | "submit";
  disabled?: boolean;
  loading?: boolean;
};

export function Button({ children, type = "button", disabled = false, loading = false }: ButtonProps) {
  return (
    <button className="button" type={type} disabled={disabled || loading} aria-busy={loading}>
      {loading ? <span className="button__spinner" aria-hidden="true" /> : null}
      <span>{children}</span>
    </button>
  );
}
