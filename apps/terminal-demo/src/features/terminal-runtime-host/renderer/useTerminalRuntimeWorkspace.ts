import { createContext, useContext } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";

export const TerminalRuntimeWorkspaceContext = createContext<TerminalRuntimeWorkspaceFacade | null>(null);

export function useTerminalRuntimeWorkspace(): TerminalRuntimeWorkspaceFacade {
  const runtime = useContext(TerminalRuntimeWorkspaceContext);
  if (!runtime) {
    throw new Error("TerminalRuntimeWorkspaceProvider is required before using runtime workspace facade");
  }

  return runtime;
}
