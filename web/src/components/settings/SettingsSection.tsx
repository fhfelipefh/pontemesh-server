import { ReactNode } from "react";

type SettingsSectionProps = {
  title: string;
  description?: string;
  icon: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  id?: string;
};

export function SettingsSection({
  title,
  description,
  icon,
  actions,
  children,
  className,
  id
}: SettingsSectionProps) {
  const classes = ["settings-card", className].filter(Boolean).join(" ");

  return (
    <section className={classes} id={id}>
      <div className="settings-card__header">
        <div className="settings-card__title-group">
          <div className="settings-card__title-icon" aria-hidden="true">
            {icon}
          </div>
          <div>
            <h2>{title}</h2>
            {description ? <p>{description}</p> : null}
          </div>
        </div>
        {actions ? <div className="settings-card__actions">{actions}</div> : null}
      </div>
      <div className="settings-card__body">{children}</div>
    </section>
  );
}
