import type { TerminalSessionState } from "@features/terminal-workspace-kernel/contracts";

export interface TerminalWorkspaceRuntimeWatchHandle {
  readonly sessionId: string;
  dispose(): Promise<void>;
}

export interface TerminalWorkspaceSessionStateRuntimePort {
  watchSessionState(
    sessionId: string,
    handlers: {
      onState(state: TerminalSessionState): void;
      onError(error: unknown): void;
      onClosed(): void;
    },
  ): Promise<TerminalWorkspaceRuntimeWatchHandle>;
}
