import type { ReactElement } from "react";

export function TerminalWorkspaceBootstrapErrorView(props: {
  error: string;
}): ReactElement {
  return (
    <main className="shell shell--error">
      <section className="panel panel--surface panel--error">
        <div className="section__eyebrow">Bootstrap Error</div>
        <h1 className="section__title">Terminal Platform Demo</h1>
        <p className="section__copy">{props.error}</p>
      </section>
    </main>
  );
}
