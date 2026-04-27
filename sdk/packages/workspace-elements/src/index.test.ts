import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_COMPOSER_ACTIONS,
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_DOCK_ACCESSORY_MODES,
  TERMINAL_COMMAND_DOCK_DEFAULT_RECENT_COMMAND_LIMIT,
  TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS,
  TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS,
  TERMINAL_COMMAND_DOCK_TERMINAL_RECENT_COMMAND_LIMIT,
  TERMINAL_COMMAND_INPUT_STATUS_DESCRIPTION_ID,
  TERMINAL_COMMAND_QUICK_COMMAND_LIMIT,
  TERMINAL_COMMAND_QUICK_COMMAND_TONES,
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  TERMINAL_SCREEN_ACTION_IDS,
  TERMINAL_SCREEN_CHROME_MODES,
  TERMINAL_SCREEN_SEARCH_ACTION_IDS,
  TERMINAL_SCREEN_EVENTS,
  TERMINAL_PANE_MAX_COLS,
  TERMINAL_PANE_MAX_ROWS,
  TERMINAL_PANE_MIN_COLS,
  TERMINAL_PANE_MIN_ROWS,
  TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
  TERMINAL_WORKSPACE_CHROME_TONES,
  TERMINAL_WORKSPACE_INSPECTOR_MODES,
  TERMINAL_WORKSPACE_LAYOUT_PRESETS,
  TERMINAL_WORKSPACE_NAVIGATION_MODES,
  TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES,
  TERMINAL_WORKSPACE_SECONDARY_DENSITIES,
  canRunTerminalTopologyCommand,
  canNavigateTerminalCommandHistory,
  compactTerminalId,
  countPaneTreeLeaves,
  createTerminalCommandHistoryNavigationState,
  defaultTerminalCommandQuickCommands,
  findRestorableSavedSession,
  hasSavedSession,
  resolveTerminalCommandComposerActions,
  resolveTerminalCommandComposerActionPlacement,
  resolveActiveBackendCapabilities,
  resolvePaneResizeCommand,
  resolveTerminalCommandDockAccessoryMode,
  resolveTerminalCommandDockAccessoryState,
  resolveTerminalCommandDockControlState,
  resolveTerminalCommandDockSessionActions,
  resolveTerminalCommandDockStatusBadges,
  resolveTerminalCommandDockStatusPlacement,
  resolveTerminalCommandHistoryNavigation,
  resolveTerminalCommandInputStatus,
  resolveTerminalCommandQuickCommands,
  resolveTerminalCommandRecentCommands,
  resolveTerminalEntityIdLabel,
  resolveTerminalSavedSessionsControlState,
  resolveTerminalScreenActions,
  resolveTerminalScreenChromeState,
  resolveTerminalScreenControlState,
  resolveTerminalScreenInputStatus,
  resolveTerminalScreenSearchActions,
  resolveTerminalTabStripControlState,
  resolveTerminalTabStripKeyboardIntent,
  TerminalTabStripElement,
  resolveTerminalToolbarFontScaleOption,
  resolveTerminalToolbarLineWrapOption,
  resolveTerminalToolbarThemeOption,
  resolveTerminalTopologyControlState,
  resolveTerminalTopologyStatus,
  resolveTerminalWorkspaceChromeState,
  resolveTerminalWorkspaceInspectorState,
  resolveTerminalWorkspaceLayoutState,
  resolveTerminalWorkspaceNavigationState,
  resolveWorkspaceCapability,
  TerminalCommandComposerElement,
  type TerminalCommandComposerActionId,
  type TerminalCommandComposerActionLabelMode,
  type TerminalCommandComposerActionOptions,
  type TerminalCommandComposerActionPlacement,
  type TerminalCommandComposerActionPresentation,
  type TerminalCommandComposerActionTone,
  type TerminalCommandDockCapabilityStatus,
  type TerminalCommandDockControlState,
  type TerminalCommandDockSessionActionId,
  type TerminalCommandDockSessionActionLabelMode,
  type TerminalCommandDockSessionActionOptions,
  type TerminalCommandDockSessionActionPlacement,
  type TerminalCommandDockSessionActionPresentation,
  type TerminalCommandDockSessionActionTone,
  type TerminalCommandDockStatusBadge,
  type TerminalCommandDockStatusBadgeId,
  type TerminalCommandDockStatusOptions,
  type TerminalCommandDockStatusPlacement,
  type TerminalCommandDockStatusTone,
  type TerminalCommandComposerDraftChangeDetail,
  type TerminalCommandComposerEventMap,
  type TerminalCommandComposerEventType,
  type TerminalCommandComposerHistoryNavigateDetail,
  type TerminalCommandComposerShortcut,
  type TerminalCommandComposerShortcutDetail,
  type TerminalCommandDockAccessoryMode,
  type TerminalCommandDockAccessoryOptions,
  type TerminalCommandDockAccessoryState,
  type TerminalCommandDockAccessoryStateOptions,
  type TerminalCommandHistoryInputState,
  type TerminalCommandHistoryNavigationDirection,
  type TerminalCommandHistoryNavigationResult,
  type TerminalCommandHistoryNavigationState,
  type TerminalCommandInputStatus,
  type TerminalCommandQuickCommand,
  type TerminalCommandQuickCommandPresentation,
  type TerminalCommandQuickCommandTone,
  type TerminalCommandRecentCommandPresentation,
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
  type TerminalScreenActionId,
  type TerminalScreenActionLabelMode,
  type TerminalScreenActionOptions,
  type TerminalScreenActionPlacement,
  type TerminalScreenActionPresentation,
  type TerminalScreenActionTone,
  type TerminalScreenCopiedDetail,
  type TerminalScreenCopyFailedDetail,
  type TerminalScreenCopyState,
  type TerminalScreenControlState,
  type TerminalScreenChromeMetaItem,
  type TerminalScreenChromeMetaItemId,
  type TerminalScreenChromeMode,
  type TerminalScreenChromeOptions,
  type TerminalScreenChromeState,
  type TerminalScreenEventMap,
  type TerminalScreenEventType,
  type TerminalScreenInputFailedDetail,
  type TerminalScreenInputSubmittedDetail,
  type TerminalScreenInputActivity,
  type TerminalScreenInputStatus,
  type TerminalScreenInputTone,
  type TerminalScreenSearchActionId,
  type TerminalScreenSearchActionLabelMode,
  type TerminalScreenSearchActionOptions,
  type TerminalScreenSearchActionPlacement,
  type TerminalScreenSearchActionPresentation,
  type TerminalScreenSearchActionTone,
  type TerminalScreenPasteFailedDetail,
  type TerminalScreenPasteSubmittedDetail,
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
  type TerminalWorkspaceChromeState,
  type TerminalWorkspaceChromeTone,
  type TerminalWorkspaceInspectorMode,
  type TerminalWorkspaceInspectorState,
  type TerminalWorkspaceLayoutOptions,
  type TerminalWorkspaceLayoutPreset,
  type TerminalWorkspaceLayoutState,
  type TerminalWorkspaceNavigationMode,
  type TerminalWorkspaceNavigationState,
  type TerminalWorkspaceSecondarySummaryOptions,
  type TerminalWorkspaceSecondaryChromeMode,
  type TerminalWorkspaceSecondaryDensity,
} from "./index.js";

type PublicControlTypes =
  | TerminalCommandComposerActionId
  | TerminalCommandComposerActionLabelMode
  | TerminalCommandComposerActionOptions
  | TerminalCommandComposerActionPlacement
  | TerminalCommandComposerActionPresentation
  | TerminalCommandComposerActionTone
  | TerminalCommandDockCapabilityStatus
  | TerminalCommandDockControlState
  | TerminalCommandDockSessionActionId
  | TerminalCommandDockSessionActionLabelMode
  | TerminalCommandDockSessionActionOptions
  | TerminalCommandDockSessionActionPlacement
  | TerminalCommandDockSessionActionPresentation
  | TerminalCommandDockSessionActionTone
  | TerminalCommandDockStatusBadge
  | TerminalCommandDockStatusBadgeId
  | TerminalCommandDockStatusOptions
  | TerminalCommandDockStatusPlacement
  | TerminalCommandDockStatusTone
  | TerminalCommandComposerDraftChangeDetail
  | TerminalCommandComposerEventMap
  | TerminalCommandComposerEventType
  | TerminalCommandComposerHistoryNavigateDetail
  | TerminalCommandComposerShortcut
  | TerminalCommandComposerShortcutDetail
  | TerminalCommandDockAccessoryMode
  | TerminalCommandDockAccessoryOptions
  | TerminalCommandDockAccessoryState
  | TerminalCommandDockAccessoryStateOptions
  | TerminalCommandHistoryInputState
  | TerminalCommandHistoryNavigationDirection
  | TerminalCommandHistoryNavigationResult
  | TerminalCommandHistoryNavigationState
  | TerminalCommandInputStatus
  | TerminalCommandQuickCommand
  | TerminalCommandQuickCommandPresentation
  | TerminalCommandQuickCommandTone
  | TerminalCommandRecentCommandPresentation
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
  | TerminalScreenActionId
  | TerminalScreenActionLabelMode
  | TerminalScreenActionOptions
  | TerminalScreenActionPlacement
  | TerminalScreenActionPresentation
  | TerminalScreenActionTone
  | TerminalScreenCopiedDetail
  | TerminalScreenCopyFailedDetail
  | TerminalScreenCopyState
  | TerminalScreenControlState
  | TerminalScreenChromeMetaItem
  | TerminalScreenChromeMetaItemId
  | TerminalScreenChromeMode
  | TerminalScreenChromeOptions
  | TerminalScreenChromeState
  | TerminalScreenEventMap
  | TerminalScreenEventType
  | TerminalScreenInputFailedDetail
  | TerminalScreenInputSubmittedDetail
  | TerminalScreenInputActivity
  | TerminalScreenInputStatus
  | TerminalScreenInputTone
  | TerminalScreenSearchActionId
  | TerminalScreenSearchActionLabelMode
  | TerminalScreenSearchActionOptions
  | TerminalScreenSearchActionPlacement
  | TerminalScreenSearchActionPresentation
  | TerminalScreenSearchActionTone
  | TerminalScreenPasteFailedDetail
  | TerminalScreenPasteSubmittedDetail
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
  | TerminalWorkspaceChromeState
  | TerminalWorkspaceChromeTone
  | TerminalWorkspaceInspectorMode
  | TerminalWorkspaceInspectorState
  | TerminalWorkspaceLayoutOptions
  | TerminalWorkspaceLayoutPreset
  | TerminalWorkspaceLayoutState
  | TerminalWorkspaceNavigationMode
  | TerminalWorkspaceNavigationState
  | TerminalWorkspaceSecondarySummaryOptions
  | TerminalWorkspaceSecondaryChromeMode
  | TerminalWorkspaceSecondaryDensity;

describe("workspace elements public api", () => {
  it("exports reusable control resolvers for custom UI surfaces", () => {
    const resolvers = [
      TerminalCommandComposerElement,
      TerminalTabStripElement,
      resolveTerminalCommandComposerActions,
      resolveTerminalCommandComposerActionPlacement,
      findRestorableSavedSession,
      hasSavedSession,
      resolveActiveBackendCapabilities,
      resolvePaneResizeCommand,
      resolveTerminalCommandDockAccessoryMode,
      resolveTerminalCommandDockAccessoryState,
      resolveTerminalCommandDockControlState,
      resolveTerminalCommandDockSessionActions,
      resolveTerminalCommandDockStatusBadges,
      resolveTerminalCommandDockStatusPlacement,
      resolveTerminalCommandHistoryNavigation,
      resolveTerminalCommandInputStatus,
      resolveTerminalCommandQuickCommands,
      resolveTerminalCommandRecentCommands,
      resolveTerminalEntityIdLabel,
      resolveTerminalSavedSessionsControlState,
      resolveTerminalScreenActions,
      resolveTerminalScreenChromeState,
      resolveTerminalScreenControlState,
      resolveTerminalScreenInputStatus,
      resolveTerminalScreenSearchActions,
      resolveTerminalTabStripControlState,
      resolveTerminalTabStripKeyboardIntent,
      resolveTerminalToolbarFontScaleOption,
      resolveTerminalToolbarLineWrapOption,
      resolveTerminalToolbarThemeOption,
      resolveTerminalTopologyControlState,
      resolveTerminalTopologyStatus,
      resolveTerminalWorkspaceChromeState,
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
    expect(TERMINAL_COMMAND_DOCK_DEFAULT_RECENT_COMMAND_LIMIT).toBe(5);
    expect(TERMINAL_COMMAND_DOCK_TERMINAL_RECENT_COMMAND_LIMIT).toBe(2);
    expect(TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.saveLayout).toBe("save-layout");
    expect(TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.historyCount).toBe("history-count");
    expect(TERMINAL_COMMAND_INPUT_STATUS_DESCRIPTION_ID).toBe("tp-command-input-status");
    expect(TERMINAL_COMMAND_QUICK_COMMAND_TONES.primary).toBe("primary");
    expect(TERMINAL_SCREEN_ACTION_IDS.copyVisible).toBe("copy-visible");
    expect(TERMINAL_SCREEN_CHROME_MODES.compact).toBe("compact");
    expect(TERMINAL_SCREEN_SEARCH_ACTION_IDS.nextMatch).toBe("next-match");
    expect(TERMINAL_SCREEN_EVENTS.inputSubmitted).toBe("tp-terminal-screen-input-submitted");
    expect(TERMINAL_SCREEN_EVENTS.pasteSubmitted).toBe("tp-terminal-screen-paste-submitted");
    expect(TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT).toBeGreaterThan(0);
    expect(TERMINAL_PANE_MIN_ROWS).toBeLessThan(TERMINAL_PANE_MAX_ROWS);
    expect(TERMINAL_PANE_MIN_COLS).toBeLessThan(TERMINAL_PANE_MAX_COLS);
    expect(defaultTerminalCommandQuickCommands.length).toBeGreaterThan(0);
    expect(TERMINAL_WORKSPACE_CHROME_TONES.terminal).toBe("terminal");
    expect(TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed).toBe("collapsed");
    expect(TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal).toBe("terminal");
    expect(TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed).toBe("collapsed");
    expect(TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES.terminal).toBe("terminal");
    expect(TERMINAL_WORKSPACE_SECONDARY_DENSITIES.compact).toBe("compact");
  });
});

function assertPublicControlTypesAreImportable(_value: PublicControlTypes): void {}

assertPublicControlTypesAreImportable(null as never);
