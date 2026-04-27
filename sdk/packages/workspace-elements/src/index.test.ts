import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_COMPOSER_ACTIONS,
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_DOCK_ACCESSORY_MODES,
  TERMINAL_COMMAND_QUICK_COMMAND_LIMIT,
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  TERMINAL_SCREEN_CHROME_MODES,
  TERMINAL_PANE_MAX_COLS,
  TERMINAL_PANE_MAX_ROWS,
  TERMINAL_PANE_MIN_COLS,
  TERMINAL_PANE_MIN_ROWS,
  TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
  TERMINAL_WORKSPACE_INSPECTOR_MODES,
  TERMINAL_WORKSPACE_LAYOUT_PRESETS,
  TERMINAL_WORKSPACE_NAVIGATION_MODES,
  canRunTerminalTopologyCommand,
  canNavigateTerminalCommandHistory,
  compactTerminalId,
  countPaneTreeLeaves,
  createTerminalCommandHistoryNavigationState,
  defaultTerminalCommandQuickCommands,
  findRestorableSavedSession,
  hasSavedSession,
  resolveTerminalCommandComposerActions,
  resolveActiveBackendCapabilities,
  resolvePaneResizeCommand,
  resolveTerminalCommandDockAccessoryMode,
  resolveTerminalCommandDockControlState,
  resolveTerminalCommandHistoryNavigation,
  resolveTerminalCommandInputStatus,
  resolveTerminalCommandQuickCommands,
  resolveTerminalEntityIdLabel,
  resolveTerminalSavedSessionsControlState,
  resolveTerminalScreenChromeState,
  resolveTerminalScreenControlState,
  resolveTerminalScreenInputStatus,
  resolveTerminalTabStripControlState,
  resolveTerminalTabStripKeyboardIntent,
  TerminalTabStripElement,
  resolveTerminalToolbarFontScaleOption,
  resolveTerminalToolbarLineWrapOption,
  resolveTerminalToolbarThemeOption,
  resolveTerminalTopologyControlState,
  resolveTerminalTopologyStatus,
  resolveTerminalWorkspaceInspectorState,
  resolveTerminalWorkspaceLayoutState,
  resolveTerminalWorkspaceNavigationState,
  resolveWorkspaceCapability,
  TerminalCommandComposerElement,
  type TerminalCommandComposerActionId,
  type TerminalCommandComposerActionOptions,
  type TerminalCommandComposerActionPresentation,
  type TerminalCommandDockCapabilityStatus,
  type TerminalCommandDockControlState,
  type TerminalCommandComposerDraftChangeDetail,
  type TerminalCommandComposerEventMap,
  type TerminalCommandComposerEventType,
  type TerminalCommandComposerHistoryNavigateDetail,
  type TerminalCommandComposerShortcut,
  type TerminalCommandComposerShortcutDetail,
  type TerminalCommandDockAccessoryMode,
  type TerminalCommandDockAccessoryOptions,
  type TerminalCommandHistoryInputState,
  type TerminalCommandHistoryNavigationDirection,
  type TerminalCommandHistoryNavigationResult,
  type TerminalCommandHistoryNavigationState,
  type TerminalCommandInputStatus,
  type TerminalCommandQuickCommand,
  type TerminalEntityIdLabel,
  type TerminalEntityIdLabelOptions,
  type TerminalPaneResizeDelta,
  type TerminalPaneSize,
  type TerminalSavedSessionItemControlState,
  type TerminalSavedSessionPendingAction,
  type TerminalSavedSessionRestoreSemanticsNote,
  type TerminalSavedSessionRestoreSemanticsTone,
  type TerminalSavedSessionRestoreStatus,
  type TerminalSavedSessionsBulkAction,
  type TerminalSavedSessionsControlOptions,
  type TerminalSavedSessionsControlState,
  type TerminalScreenControlState,
  type TerminalScreenChromeMetaItem,
  type TerminalScreenChromeMetaItemId,
  type TerminalScreenChromeMode,
  type TerminalScreenChromeOptions,
  type TerminalScreenChromeState,
  type TerminalScreenInputActivity,
  type TerminalScreenInputStatus,
  type TerminalScreenInputTone,
  type TerminalTabStripControlOptions,
  type TerminalTabStripControlState,
  type TerminalTabStripItemControlState,
  type TerminalTabStripKeyboardInput,
  type TerminalTabStripKeyboardIntent,
  type TerminalToolbarFontScaleOptionPresentation,
  type TerminalToolbarLineWrapOptionPresentation,
  type TerminalToolbarThemeOptionPresentation,
  type TerminalTopologyControlState,
  type TerminalTopologyStatus,
  type TerminalWorkspaceCapabilityState,
  type TerminalWorkspaceCapabilityStatus,
  type TerminalWorkspaceInspectorMode,
  type TerminalWorkspaceInspectorState,
  type TerminalWorkspaceLayoutOptions,
  type TerminalWorkspaceLayoutPreset,
  type TerminalWorkspaceLayoutState,
  type TerminalWorkspaceNavigationMode,
  type TerminalWorkspaceNavigationState,
} from "./index.js";

type PublicControlTypes =
  | TerminalCommandComposerActionId
  | TerminalCommandComposerActionOptions
  | TerminalCommandComposerActionPresentation
  | TerminalCommandDockCapabilityStatus
  | TerminalCommandDockControlState
  | TerminalCommandComposerDraftChangeDetail
  | TerminalCommandComposerEventMap
  | TerminalCommandComposerEventType
  | TerminalCommandComposerHistoryNavigateDetail
  | TerminalCommandComposerShortcut
  | TerminalCommandComposerShortcutDetail
  | TerminalCommandDockAccessoryMode
  | TerminalCommandDockAccessoryOptions
  | TerminalCommandHistoryInputState
  | TerminalCommandHistoryNavigationDirection
  | TerminalCommandHistoryNavigationResult
  | TerminalCommandHistoryNavigationState
  | TerminalCommandInputStatus
  | TerminalCommandQuickCommand
  | TerminalEntityIdLabel
  | TerminalEntityIdLabelOptions
  | TerminalPaneResizeDelta
  | TerminalPaneSize
  | TerminalSavedSessionItemControlState
  | TerminalSavedSessionPendingAction
  | TerminalSavedSessionRestoreSemanticsNote
  | TerminalSavedSessionRestoreSemanticsTone
  | TerminalSavedSessionRestoreStatus
  | TerminalSavedSessionsBulkAction
  | TerminalSavedSessionsControlOptions
  | TerminalSavedSessionsControlState
  | TerminalScreenControlState
  | TerminalScreenChromeMetaItem
  | TerminalScreenChromeMetaItemId
  | TerminalScreenChromeMode
  | TerminalScreenChromeOptions
  | TerminalScreenChromeState
  | TerminalScreenInputActivity
  | TerminalScreenInputStatus
  | TerminalScreenInputTone
  | TerminalTabStripControlOptions
  | TerminalTabStripControlState
  | TerminalTabStripItemControlState
  | TerminalTabStripKeyboardInput
  | TerminalTabStripKeyboardIntent
  | TerminalToolbarFontScaleOptionPresentation
  | TerminalToolbarLineWrapOptionPresentation
  | TerminalToolbarThemeOptionPresentation
  | TerminalTopologyControlState
  | TerminalTopologyStatus
  | TerminalWorkspaceCapabilityState
  | TerminalWorkspaceCapabilityStatus
  | TerminalWorkspaceInspectorMode
  | TerminalWorkspaceInspectorState
  | TerminalWorkspaceLayoutOptions
  | TerminalWorkspaceLayoutPreset
  | TerminalWorkspaceLayoutState
  | TerminalWorkspaceNavigationMode
  | TerminalWorkspaceNavigationState;

describe("workspace elements public api", () => {
  it("exports reusable control resolvers for custom UI surfaces", () => {
    const resolvers = [
      TerminalCommandComposerElement,
      TerminalTabStripElement,
      resolveTerminalCommandComposerActions,
      findRestorableSavedSession,
      hasSavedSession,
      resolveActiveBackendCapabilities,
      resolvePaneResizeCommand,
      resolveTerminalCommandDockAccessoryMode,
      resolveTerminalCommandDockControlState,
      resolveTerminalCommandHistoryNavigation,
      resolveTerminalCommandInputStatus,
      resolveTerminalCommandQuickCommands,
      resolveTerminalEntityIdLabel,
      resolveTerminalSavedSessionsControlState,
      resolveTerminalScreenChromeState,
      resolveTerminalScreenControlState,
      resolveTerminalScreenInputStatus,
      resolveTerminalTabStripControlState,
      resolveTerminalTabStripKeyboardIntent,
      resolveTerminalToolbarFontScaleOption,
      resolveTerminalToolbarLineWrapOption,
      resolveTerminalToolbarThemeOption,
      resolveTerminalTopologyControlState,
      resolveTerminalTopologyStatus,
      resolveTerminalWorkspaceInspectorState,
      resolveTerminalWorkspaceLayoutState,
      resolveTerminalWorkspaceNavigationState,
      resolveWorkspaceCapability,
      canRunTerminalTopologyCommand,
      canNavigateTerminalCommandHistory,
      compactTerminalId,
      countPaneTreeLeaves,
      createTerminalCommandHistoryNavigationState,
    ];

    expect(resolvers.every((resolver) => typeof resolver === "function")).toBe(true);
  });

  it("exports stable control constants", () => {
    expect(TERMINAL_COMMAND_QUICK_COMMAND_LIMIT).toBeGreaterThan(0);
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS[0]?.id).toBe(TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit);
    expect(TERMINAL_COMMAND_COMPOSER_EVENTS.submit).toBe("tp-terminal-command-submit");
    expect(TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.bar).toBe("bar");
    expect(TERMINAL_SCREEN_CHROME_MODES.compact).toBe("compact");
    expect(TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT).toBeGreaterThan(0);
    expect(TERMINAL_PANE_MIN_ROWS).toBeLessThan(TERMINAL_PANE_MAX_ROWS);
    expect(TERMINAL_PANE_MIN_COLS).toBeLessThan(TERMINAL_PANE_MAX_COLS);
    expect(defaultTerminalCommandQuickCommands.length).toBeGreaterThan(0);
    expect(TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed).toBe("collapsed");
    expect(TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal).toBe("terminal");
    expect(TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed).toBe("collapsed");
  });
});

function assertPublicControlTypesAreImportable(_value: PublicControlTypes): void {}

assertPublicControlTypesAreImportable(null as never);
