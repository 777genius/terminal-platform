import "./styles.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  resolveTerminalRuntimeBootstrapConfig,
  TerminalRuntimeBootstrapErrorView,
} from "@features/terminal-runtime-host/renderer";
import { TerminalDemoWorkspaceApp } from "./TerminalDemoWorkspaceApp.js";

const root = createRoot(document.getElementById("root")!);
const bootstrap = resolveTerminalRuntimeBootstrapConfig();

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
