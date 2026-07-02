import { AlertCircle, AlertTriangle, CheckCircle2, Info } from "lucide-react";
import { ReactNode } from "react";

type InfoBoxProps = {
  children: ReactNode;
  variant?: "info" | "warning" | "success" | "danger";
  className?: string;
};

const icons = {
  info: Info,
  warning: AlertTriangle,
  success: CheckCircle2,
  danger: AlertCircle
};

export function InfoBox({ children, variant = "info", className }: InfoBoxProps) {
  const Icon = icons[variant];
  const classes = ["settings-info-box", `settings-info-box--${variant}`, className].filter(Boolean).join(" ");

  return (
    <div className={classes}>
      <Icon size={18} aria-hidden="true" />
      <div>{children}</div>
    </div>
  );
}
