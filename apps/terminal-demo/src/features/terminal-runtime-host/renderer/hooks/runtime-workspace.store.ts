import { create } from "zustand";
import {
  initialTerminalRuntimeWorkspaceState,
  type TerminalRuntimeWorkspaceState,
} from "@features/terminal-workspace-kernel/contracts";

export interface TerminalRuntimeWorkspaceStoreState extends TerminalRuntimeWorkspaceState {
  reset(): void;
}

export const useTerminalRuntimeWorkspaceStore = create<TerminalRuntimeWorkspaceStoreState>((set) => ({
  ...initialTerminalRuntimeWorkspaceState,
  reset: () => {
    set(initialTerminalRuntimeWorkspaceState);
  },
}));
