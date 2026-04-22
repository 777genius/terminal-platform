import type { TerminalRuntimeWorkspaceState } from "@features/terminal-workspace-kernel/contracts";

export interface TerminalWorkspaceStorePort {
  getState(): TerminalRuntimeWorkspaceState;
  patch(patch: Partial<TerminalRuntimeWorkspaceState>): void;
}
