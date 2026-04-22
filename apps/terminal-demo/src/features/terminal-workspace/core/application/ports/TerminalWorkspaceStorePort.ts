import type { TerminalWorkspaceViewState } from "../TerminalWorkspaceViewState.js";

export interface TerminalWorkspaceStorePort {
  getState(): TerminalWorkspaceViewState;
  patch(patch: Partial<TerminalWorkspaceViewState>): void;
}
