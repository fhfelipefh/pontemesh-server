import { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "type"> & {
  children: ReactNode;
  className?: string;
  icon?: ReactNode;
  type?: "button" | "submit";
  variant?: "primary" | "secondary" | "ghost" | "danger";
  disabled?: boolean;
  loading?: boolean;
};

export function Button({
  children,
  className,
  icon,
  type = "button",
  variant = "primary",
  disabled = false,
  loading = false,
  onClick,
  ...buttonProps
}: ButtonProps) {
  const classes = ["button", `button--${variant}`, className].filter(Boolean).join(" ");

  return (
    <button
      {...buttonProps}
      className={classes}
      type={type}
      disabled={disabled || loading}
      aria-busy={loading}
      onClick={onClick}
    >
      {loading ? <span className="button__spinner" aria-hidden="true" /> : icon}
      <span>{children}</span>
    </button>
  );
}
