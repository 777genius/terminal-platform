import type { TerminalDemoBootstrapConfig } from "../contracts/index.js";
import type { TerminalWorkspaceController, TerminalWorkspaceViewState } from "../core/application/index.js";

declare global {
  interface Window {
    terminalDemo?: {
      config: TerminalDemoBootstrapConfig;
    };
    terminalDemoDebug?: {
      controller: TerminalWorkspaceController;
      getState(): TerminalWorkspaceViewState;
      setInputDraft(value: string): void;
    };
  }
}
