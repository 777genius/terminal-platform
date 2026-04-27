export { defineTerminalPlatformElements } from "./define.js";

export {
  TERMINAL_COMMAND_COMPOSER_ACTIONS,
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
  resolveTerminalCommandComposerActions,
  type TerminalCommandComposerActionId,
  type TerminalCommandComposerActionOptions,
  type TerminalCommandComposerActionPresentation,
} from "./elements/terminal-command-composer-actions.js";
export {
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  TerminalCommandComposerElement,
  type TerminalCommandComposerDraftChangeDetail,
  type TerminalCommandComposerEventMap,
  type TerminalCommandComposerEventType,
  type TerminalCommandComposerHistoryNavigateDetail,
  type TerminalCommandComposerShortcut,
  type TerminalCommandComposerShortcutDetail,
} from "./elements/terminal-command-composer-element.js";
export {
  TERMINAL_COMMAND_COMPOSER_DEFAULT_MAX_ROWS,
  TERMINAL_COMMAND_COMPOSER_DEFAULT_MIN_ROWS,
  resolveTerminalCommandComposerRowRange,
  resolveTerminalCommandComposerRows,
  type TerminalCommandComposerRowOptions,
  type TerminalCommandComposerRowRange,
} from "./elements/terminal-command-composer-layout.js";
export { TerminalCommandDockElement } from "./elements/terminal-command-dock-element.js";
export {
  TERMINAL_COMMAND_DOCK_ACCESSORY_MODES,
  resolveTerminalCommandDockAccessoryMode,
  type TerminalCommandDockAccessoryMode,
  type TerminalCommandDockAccessoryOptions,
} from "./elements/terminal-command-dock-accessories.js";
export {
  resolveTerminalCommandDockControlState,
  type TerminalCommandDockCapabilityStatus,
  type TerminalCommandDockControlState,
} from "./elements/terminal-command-dock-controls.js";
export {
  TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS,
  resolveTerminalCommandDockSessionActions,
  type TerminalCommandDockSessionActionId,
  type TerminalCommandDockSessionActionOptions,
  type TerminalCommandDockSessionActionPlacement,
  type TerminalCommandDockSessionActionPresentation,
} from "./elements/terminal-command-dock-session-actions.js";
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
export {
  canNavigateTerminalCommandHistory,
  createTerminalCommandHistoryNavigationState,
  resolveTerminalCommandHistoryNavigation,
  type TerminalCommandHistoryInputState,
  type TerminalCommandHistoryNavigationDirection,
  type TerminalCommandHistoryNavigationResult,
  type TerminalCommandHistoryNavigationState,
} from "./elements/terminal-command-history-navigation.js";
export { TerminalWorkspaceElement } from "./elements/terminal-workspace-element.js";
export { TerminalSessionListElement } from "./elements/terminal-session-list-element.js";
export { TerminalTabStripElement } from "./elements/terminal-tab-strip-element.js";
export {
  resolveTerminalTabStripControlState,
  type TerminalTabStripControlOptions,
  type TerminalTabStripControlState,
  type TerminalTabStripItemControlState,
} from "./elements/terminal-tab-strip-controls.js";
export {
  resolveTerminalTabStripKeyboardIntent,
  type TerminalTabStripKeyboardInput,
  type TerminalTabStripKeyboardIntent,
} from "./elements/terminal-tab-strip-keyboard-navigation.js";
export { TerminalToolbarElement } from "./elements/terminal-toolbar-element.js";
export {
  resolveTerminalToolbarFontScaleOption,
  resolveTerminalToolbarLineWrapOption,
  resolveTerminalToolbarThemeOption,
  type TerminalToolbarFontScaleOptionPresentation,
  type TerminalToolbarLineWrapOptionPresentation,
  type TerminalToolbarThemeOptionPresentation,
} from "./elements/terminal-toolbar-preferences.js";
export { TerminalStatusBarElement } from "./elements/terminal-status-bar-element.js";
export { TerminalScreenElement } from "./elements/terminal-screen-element.js";
export {
  TERMINAL_SCREEN_CHROME_MODES,
  resolveTerminalScreenChromeState,
  type TerminalScreenChromeMetaItem,
  type TerminalScreenChromeMetaItemId,
  type TerminalScreenChromeMode,
  type TerminalScreenChromeOptions,
  type TerminalScreenChromeState,
} from "./elements/terminal-screen-chrome.js";
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
export {
  TERMINAL_WORKSPACE_CHROME_TONES,
  TERMINAL_WORKSPACE_INSPECTOR_MODES,
  TERMINAL_WORKSPACE_LAYOUT_PRESETS,
  TERMINAL_WORKSPACE_NAVIGATION_MODES,
  TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES,
  TERMINAL_WORKSPACE_SECONDARY_DENSITIES,
  resolveTerminalWorkspaceChromeState,
  resolveTerminalWorkspaceInspectorState,
  resolveTerminalWorkspaceLayoutState,
  resolveTerminalWorkspaceNavigationState,
  type TerminalWorkspaceChromeState,
  type TerminalWorkspaceChromeTone,
  type TerminalWorkspaceInspectorMode,
  type TerminalWorkspaceInspectorState,
  type TerminalWorkspaceLayoutOptions,
  type TerminalWorkspaceLayoutPreset,
  type TerminalWorkspaceLayoutState,
  type TerminalWorkspaceNavigationMode,
  type TerminalWorkspaceNavigationState,
  type TerminalWorkspaceSecondaryChromeMode,
  type TerminalWorkspaceSecondaryDensity,
} from "./elements/terminal-workspace-layout.js";
