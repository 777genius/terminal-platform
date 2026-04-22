import { useMemo } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import { createTerminalActiveSessionCommands } from "../commands/createTerminalActiveSessionCommands.js";
import { createTerminalActiveSessionModel } from "../presenters/createTerminalActiveSessionModel.js";

export function useTerminalActiveSession(runtime: TerminalRuntimeWorkspaceFacade) {
  const model = useMemo(() => createTerminalActiveSessionModel(runtime), [runtime]);
  const commands = useMemo(() => createTerminalActiveSessionCommands(runtime), [runtime]);

  return {
    model,
    commands,
  };
}
