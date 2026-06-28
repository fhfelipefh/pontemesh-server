import { ReactNode } from "react";

type PageShellProps = {
  title: string;
  description: string;
  children?: ReactNode;
};

export function PageShell({ title, description, children }: PageShellProps) {
  return (
    <main className="page">
      <section className="panel" aria-labelledby="page-title">
        <h1 id="page-title">{title}</h1>
        <p>{description}</p>
        {children}
      </section>
    </main>
  );
}
