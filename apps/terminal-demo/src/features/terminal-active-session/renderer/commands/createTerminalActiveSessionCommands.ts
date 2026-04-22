import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import type { TerminalActiveSessionCommands } from "./TerminalActiveSessionCommands.js";

export function createTerminalActiveSessionCommands(runtime: TerminalRuntimeWorkspaceFacade): TerminalActiveSessionCommands {
  return {
    refreshCatalog: () => runtime.commands.refreshCatalog(),
    newTab: () => runtime.commands.newTab(),
    splitHorizontal: () => runtime.commands.splitFocusedPane("horizontal"),
    splitVertical: () => runtime.commands.splitFocusedPane("vertical"),
    saveSession: () => runtime.commands.saveSession(),
    focusPane: (paneId) => runtime.commands.focusPane(paneId),
    focusTab: (tabId) => runtime.commands.focusTab(tabId),
  };
}
