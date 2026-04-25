export { defineTerminalPlatformElements } from "./define.js";

export { TerminalCommandDockElement } from "./elements/terminal-command-dock-element.js";
export {
  resolveTerminalCommandDockControlState,
  type TerminalCommandDockCapabilityStatus,
  type TerminalCommandDockControlState,
} from "./elements/terminal-command-dock-controls.js";
export {
  resolveTerminalCommandInputStatus,
  type TerminalCommandInputStatus,
} from "./elements/terminal-command-input-status.js";
export {
  TERMINAL_COMMAND_QUICK_COMMAND_LIMIT,
  defaultTerminalCommandQuickCommands,
  resolveTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./elements/terminal-command-quick-commands.js";
export { TerminalWorkspaceElement } from "./elements/terminal-workspace-element.js";
export { TerminalSessionListElement } from "./elements/terminal-session-list-element.js";
export { TerminalToolbarElement } from "./elements/terminal-toolbar-element.js";
export { TerminalStatusBarElement } from "./elements/terminal-status-bar-element.js";
export { TerminalScreenElement } from "./elements/terminal-screen-element.js";
export {
  resolveTerminalScreenControlState,
  type TerminalScreenControlState,
} from "./elements/terminal-screen-controls.js";
export {
  resolveTerminalScreenInputStatus,
  type TerminalScreenInputActivity,
  type TerminalScreenInputStatus,
  type TerminalScreenInputTone,
} from "./elements/terminal-screen-input-status.js";
export { TerminalPaneTreeElement } from "./elements/terminal-pane-tree-element.js";
export {
  compactTerminalId,
  resolveTerminalEntityIdLabel,
  type TerminalEntityIdLabel,
  type TerminalEntityIdLabelOptions,
} from "./elements/terminal-identity.js";
export {
  TERMINAL_PANE_MAX_COLS,
  TERMINAL_PANE_MAX_ROWS,
  TERMINAL_PANE_MIN_COLS,
  TERMINAL_PANE_MIN_ROWS,
  canRunTerminalTopologyCommand,
  countPaneTreeLeaves,
  resolvePaneResizeCommand,
  resolveTerminalTopologyControlState,
  type TerminalPaneResizeDelta,
  type TerminalPaneSize,
  type TerminalTopologyControlState,
} from "./elements/terminal-topology-controls.js";
export {
  resolveTerminalTopologyStatus,
  type TerminalTopologyStatus,
} from "./elements/terminal-topology-status.js";
export { TerminalSavedSessionsElement } from "./elements/terminal-saved-sessions-element.js";
export {
  findRestorableSavedSession,
  hasSavedSession,
  resolveTerminalSavedSessionsControlState,
  TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
  type TerminalSavedSessionItemControlState,
  type TerminalSavedSessionPendingAction,
  type TerminalSavedSessionRestoreSemanticsNote,
  type TerminalSavedSessionRestoreSemanticsTone,
  type TerminalSavedSessionRestoreStatus,
  type TerminalSavedSessionsBulkAction,
  type TerminalSavedSessionsControlOptions,
  type TerminalSavedSessionsControlState,
} from "./elements/terminal-saved-sessions-controls.js";
export {
  resolveActiveBackendCapabilities,
  resolveWorkspaceCapability,
  type TerminalWorkspaceCapabilityState,
  type TerminalWorkspaceCapabilityStatus,
} from "./elements/terminal-workspace-capabilities.js";
