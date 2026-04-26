import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_COMPOSER_ACTIONS,
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_QUICK_COMMAND_LIMIT,
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  TERMINAL_PANE_MAX_COLS,
  TERMINAL_PANE_MAX_ROWS,
  TERMINAL_PANE_MIN_COLS,
  TERMINAL_PANE_MIN_ROWS,
  TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
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
  resolveTerminalCommandDockControlState,
  resolveTerminalCommandHistoryNavigation,
  resolveTerminalCommandInputStatus,
  resolveTerminalCommandQuickCommands,
  resolveTerminalEntityIdLabel,
  resolveTerminalSavedSessionsControlState,
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
  | TerminalWorkspaceCapabilityStatus;

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
      resolveTerminalCommandDockControlState,
      resolveTerminalCommandHistoryNavigation,
      resolveTerminalCommandInputStatus,
      resolveTerminalCommandQuickCommands,
      resolveTerminalEntityIdLabel,
      resolveTerminalSavedSessionsControlState,
      resolveTerminalScreenControlState,
      resolveTerminalScreenInputStatus,
      resolveTerminalTabStripControlState,
      resolveTerminalTabStripKeyboardIntent,
      resolveTerminalToolbarFontScaleOption,
      resolveTerminalToolbarLineWrapOption,
      resolveTerminalToolbarThemeOption,
      resolveTerminalTopologyControlState,
      resolveTerminalTopologyStatus,
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
    expect(TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT).toBeGreaterThan(0);
    expect(TERMINAL_PANE_MIN_ROWS).toBeLessThan(TERMINAL_PANE_MAX_ROWS);
    expect(TERMINAL_PANE_MIN_COLS).toBeLessThan(TERMINAL_PANE_MAX_COLS);
    expect(defaultTerminalCommandQuickCommands.length).toBeGreaterThan(0);
  });
});

function assertPublicControlTypesAreImportable(_value: PublicControlTypes): void {}

assertPublicControlTypesAreImportable(null as never);
