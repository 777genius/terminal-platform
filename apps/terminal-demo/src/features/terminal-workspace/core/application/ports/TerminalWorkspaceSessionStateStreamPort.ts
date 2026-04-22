import type { TerminalSessionState } from "../../../contracts/terminal-workspace-contracts.js";
import type {
  TerminalWorkspaceSessionStreamHealth,
} from "../TerminalWorkspaceSessionStreamHealth.js";

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
