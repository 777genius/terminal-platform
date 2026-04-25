export {
  TerminalCommandDock,
  TerminalPaneTree,
  TerminalSavedSessions,
  TerminalScreen,
  TerminalSessionList,
  TerminalStatusBar,
  TerminalToolbar,
  TerminalWorkspace,
} from "./components/terminal-workspace.js";
export { useWorkspaceSnapshot } from "./hooks/use-workspace-snapshot.js";
export {
  TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
  findRestorableSavedSession,
  hasSavedSession,
  resolveTerminalSavedSessionsControlState,
} from "@terminal-platform/workspace-elements";

export type {
  TerminalCommandQuickCommand,
  TerminalSavedSessionItemControlState,
  TerminalSavedSessionPendingAction,
  TerminalSavedSessionRestoreSemanticsNote,
  TerminalSavedSessionRestoreSemanticsTone,
  TerminalSavedSessionRestoreStatus,
  TerminalSavedSessionsBulkAction,
  TerminalSavedSessionsControlOptions,
  TerminalSavedSessionsControlState,
} from "@terminal-platform/workspace-elements";
