import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  resolveTerminalWorkspaceBootstrapConfig,
  TerminalWorkspaceApp,
  TerminalWorkspaceBootstrapErrorView,
} from "@features/terminal-workspace/renderer";

const root = createRoot(document.getElementById("root")!);
const bootstrap = resolveTerminalWorkspaceBootstrapConfig();

if (!bootstrap.config) {
  root.render(
    <StrictMode>
      <TerminalWorkspaceBootstrapErrorView error={bootstrap.error ?? "Unknown bootstrap error"} />
    </StrictMode>,
  );
} else {
  root.render(
    <StrictMode>
      <TerminalWorkspaceApp config={bootstrap.config} />
    </StrictMode>,
  );
}
