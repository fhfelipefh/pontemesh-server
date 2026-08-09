import { ReactNode } from "react";
import { BrandHeader } from "./BrandHeader";
import { HelpLink } from "./HelpLink";
import { LanguageSwitcher } from "./LanguageSwitcher";
import { SetupServerVersion } from "./SetupServerVersion";

type PageShellProps = {
  title: string;
  description: string;
  children?: ReactNode;
  compact?: boolean;
  serverVersion?: string | null;
};

export function PageShell({ title, description, children, compact = false, serverVersion }: PageShellProps) {
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

      {serverVersion ? <SetupServerVersion version={serverVersion} /> : null}
      <HelpLink />
    </main>
  );
}
