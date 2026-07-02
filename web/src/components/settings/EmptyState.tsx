import { ReactNode } from "react";

type EmptyStateProps = {
  title: string;
  description?: string;
  icon?: ReactNode;
};

export function EmptyState({ title, description, icon }: EmptyStateProps) {
  return (
    <div className="settings-empty-state">
      {icon ? <div className="settings-empty-state__icon" aria-hidden="true">{icon}</div> : null}
      <h3>{title}</h3>
      {description ? <p>{description}</p> : null}
    </div>
  );
}
