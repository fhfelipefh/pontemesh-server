import { ButtonHTMLAttributes, ReactNode } from "react";

type IconButtonProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children"> & {
  label: string;
  icon: ReactNode;
  variant?: "primary" | "danger" | "neutral";
  className?: string;
};

export function IconButton({
  label,
  icon,
  variant = "primary",
  className,
  type = "button",
  ...buttonProps
}: IconButtonProps) {
  const classes = ["settings-icon-button", `settings-icon-button--${variant}`, className].filter(Boolean).join(" ");

  return (
    <button className={classes} type={type} title={label} aria-label={label} {...buttonProps}>
      {icon}
    </button>
  );
}
