import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import type { TerminalSavedSessionsCommands } from "./TerminalSavedSessionsCommands.js";

export function createTerminalSavedSessionsCommands(options: {
  runtime: TerminalRuntimeWorkspaceFacade;
  toggleVisibility(): void;
}): TerminalSavedSessionsCommands {
  return {
    restore: (sessionId) => options.runtime.commands.restoreSavedSession(sessionId),
    delete: (sessionId) => options.runtime.commands.deleteSavedSession(sessionId),
    toggleVisibility: () => options.toggleVisibility(),
  };
}
