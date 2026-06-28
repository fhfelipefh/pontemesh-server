import { ReactNode } from "react";
import { BrandHeader } from "./BrandHeader";
import { HelpLink } from "./HelpLink";
import { LanguageSwitcher } from "./LanguageSwitcher";

type PageShellProps = {
  title: string;
  description: string;
  children?: ReactNode;
  compact?: boolean;
};

export function PageShell({ title, description, children, compact = false }: PageShellProps) {
  return (
    <main className="setup-page">
      <div className="setup-page__language">
        <LanguageSwitcher />
      </div>

      <section
        className={compact ? "setup-card setup-card--compact" : "setup-card"}
        aria-labelledby="page-title"
      >
        <BrandHeader />
        <div className="setup-card__divider" />
        <div className="setup-card__content">
          <h1 id="page-title">{title}</h1>
          <p>{description}</p>
          {children}
        </div>
      </section>

      <HelpLink />
    </main>
  );
}
