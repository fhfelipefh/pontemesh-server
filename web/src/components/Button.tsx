import { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "type"> & {
  children: ReactNode;
  className?: string;
  fullWidth?: boolean;
  icon?: ReactNode;
  size?: "sm" | "md" | "lg";
  type?: "button" | "submit";
  variant?: "primary" | "secondary" | "ghost" | "danger";
  disabled?: boolean;
  loading?: boolean;
};

export function Button({
  children,
  className,
  fullWidth = false,
  icon,
  size = "md",
  type = "button",
  variant = "primary",
  disabled = false,
  loading = false,
  onClick,
  ...buttonProps
}: ButtonProps) {
  const classes = ["button", `button--${variant}`, `button--${size}`, fullWidth ? "button--full-width" : null, className].filter(Boolean).join(" ");

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
