import type { TerminalRuntimeBootstrapConfig } from "../contracts/index.js";

declare global {
  interface Window {
    terminalDemo?: {
      config: TerminalRuntimeBootstrapConfig;
    };
    terminalDemoDebug?: {
      controller: unknown;
      getState(): unknown;
      setInputDraft?(value: string): void;
    };
  }
}
