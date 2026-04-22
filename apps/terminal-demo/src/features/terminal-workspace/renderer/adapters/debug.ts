import type { TerminalWorkspaceController, TerminalWorkspaceViewState } from "../../core/application/index.js";

export function installTerminalWorkspaceDebug(options: {
  controller: TerminalWorkspaceController;
  getState(): TerminalWorkspaceViewState;
  setInputDraft(value: string): void;
}): () => void {
  if (!import.meta.env.DEV) {
    return () => undefined;
  }

  window.terminalDemoDebug = {
    controller: options.controller,
    getState: options.getState,
    setInputDraft: options.setInputDraft,
  };

  return () => {
    delete window.terminalDemoDebug;
  };
}
