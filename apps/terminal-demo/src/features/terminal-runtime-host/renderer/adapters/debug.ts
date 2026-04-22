import type { TerminalRuntimeWorkspaceState } from "@features/terminal-workspace-kernel/contracts";
import type { TerminalRuntimeWorkspaceController } from "../../core/application/index.js";

export function installTerminalRuntimeDebug(options: {
  controller: TerminalRuntimeWorkspaceController;
  getState(): TerminalRuntimeWorkspaceState;
}): () => void {
  if (!import.meta.env.DEV) {
    return () => undefined;
  }

  window.terminalDemoDebug = {
    controller: options.controller,
    getState: options.getState,
  };

  return () => {
    delete window.terminalDemoDebug;
  };
}
