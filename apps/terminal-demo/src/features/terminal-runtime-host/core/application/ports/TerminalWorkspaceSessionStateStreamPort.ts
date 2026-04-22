import type {
  TerminalSessionState,
  TerminalWorkspaceSessionStreamHealth,
} from "@features/terminal-workspace-kernel/contracts";

export interface TerminalWorkspaceSessionStateSubscription {
  readonly subscriptionId: string;
  dispose(): Promise<void>;
}

export interface TerminalWorkspaceSessionStateStreamPort {
  subscribeSessionState(
    sessionId: string,
    handlers: {
      onState(state: TerminalSessionState): void;
      onStatusChange?(health: TerminalWorkspaceSessionStreamHealth): void;
      onError?(error: Error): void;
      onClosed?(): void;
    },
  ): Promise<TerminalWorkspaceSessionStateSubscription>;
  dispose(): void;
}
