import { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "type"> & {
  children: ReactNode;
  className?: string;
  icon?: ReactNode;
  type?: "button" | "submit";
  disabled?: boolean;
  loading?: boolean;
};

export function Button({
  children,
  className,
  icon,
  type = "button",
  disabled = false,
  loading = false,
  onClick,
  ...buttonProps
}: ButtonProps) {
  return (
    <button
      {...buttonProps}
      className={className ? `button ${className}` : "button"}
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
