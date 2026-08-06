import { Info } from "lucide-react";
import { ReactNode } from "react";

type HelpCardProps = {
  title: string;
  children: ReactNode;
};

export function HelpCard({ title, children }: HelpCardProps) {
  return (
    <aside className="settings-help-card">
      <div className="settings-help-card__icon">
        <Info size={18} aria-hidden="true" />
      </div>
      <div>
        <h2>{title}</h2>
        <p>{children}</p>
      </div>
    </aside>
  );
}
