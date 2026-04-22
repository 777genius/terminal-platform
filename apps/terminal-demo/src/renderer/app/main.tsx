import "./styles.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  resolveTerminalRuntimeBootstrapConfig,
  TerminalRuntimeBootstrapErrorView,
} from "@features/terminal-runtime-host/renderer";
import { TerminalDemoWorkspaceApp } from "./TerminalDemoWorkspaceApp.js";

const root = createRoot(document.getElementById("root")!);

root.render(
  <StrictMode>
    <main className="shell shell--error">
      <section className="panel panel--surface panel--error">
        <div className="section__eyebrow">SDK Bootstrap</div>
        <h1 className="section__title">Terminal Platform Demo</h1>
        <p className="section__copy">Resolving runtime host and latest workspace gateway...</p>
      </section>
    </main>
  </StrictMode>,
);

const bootstrap = await resolveTerminalRuntimeBootstrapConfig();

if (!bootstrap.config) {
  root.render(
    <StrictMode>
      <TerminalRuntimeBootstrapErrorView error={bootstrap.error ?? "Unknown bootstrap error"} />
    </StrictMode>,
  );
} else {
  root.render(
    <StrictMode>
      <TerminalDemoWorkspaceApp config={bootstrap.config} />
    </StrictMode>,
  );
}
