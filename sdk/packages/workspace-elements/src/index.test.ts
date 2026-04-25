import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_QUICK_COMMAND_LIMIT,
  TERMINAL_PANE_MAX_COLS,
  TERMINAL_PANE_MAX_ROWS,
  TERMINAL_PANE_MIN_COLS,
  TERMINAL_PANE_MIN_ROWS,
  TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
  canRunTerminalTopologyCommand,
  compactTerminalId,
  countPaneTreeLeaves,
  defaultTerminalCommandQuickCommands,
  findRestorableSavedSession,
  hasSavedSession,
  resolveActiveBackendCapabilities,
  resolvePaneResizeCommand,
  resolveTerminalCommandDockControlState,
  resolveTerminalCommandInputStatus,
  resolveTerminalCommandQuickCommands,
  resolveTerminalSavedSessionsControlState,
  resolveTerminalScreenControlState,
  resolveTerminalScreenInputStatus,
  resolveTerminalTopologyControlState,
  resolveTerminalTopologyStatus,
  resolveWorkspaceCapability,
  type TerminalCommandDockCapabilityStatus,
  type TerminalCommandDockControlState,
  type TerminalCommandInputStatus,
  type TerminalCommandQuickCommand,
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
  type TerminalTopologyControlState,
  type TerminalTopologyStatus,
  type TerminalWorkspaceCapabilityState,
  type TerminalWorkspaceCapabilityStatus,
} from "./index.js";

type PublicControlTypes =
  | TerminalCommandDockCapabilityStatus
  | TerminalCommandDockControlState
  | TerminalCommandInputStatus
  | TerminalCommandQuickCommand
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
  | TerminalTopologyControlState
  | TerminalTopologyStatus
  | TerminalWorkspaceCapabilityState
  | TerminalWorkspaceCapabilityStatus;

describe("workspace elements public api", () => {
  it("exports reusable control resolvers for custom UI surfaces", () => {
    const resolvers = [
      findRestorableSavedSession,
      hasSavedSession,
      resolveActiveBackendCapabilities,
      resolvePaneResizeCommand,
      resolveTerminalCommandDockControlState,
      resolveTerminalCommandInputStatus,
      resolveTerminalCommandQuickCommands,
      resolveTerminalSavedSessionsControlState,
      resolveTerminalScreenControlState,
      resolveTerminalScreenInputStatus,
      resolveTerminalTopologyControlState,
      resolveTerminalTopologyStatus,
      resolveWorkspaceCapability,
      canRunTerminalTopologyCommand,
      compactTerminalId,
      countPaneTreeLeaves,
    ];

    expect(resolvers.every((resolver) => typeof resolver === "function")).toBe(true);
  });

  it("exports stable control constants", () => {
    expect(TERMINAL_COMMAND_QUICK_COMMAND_LIMIT).toBeGreaterThan(0);
    expect(TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT).toBeGreaterThan(0);
    expect(TERMINAL_PANE_MIN_ROWS).toBeLessThan(TERMINAL_PANE_MAX_ROWS);
    expect(TERMINAL_PANE_MIN_COLS).toBeLessThan(TERMINAL_PANE_MAX_COLS);
    expect(defaultTerminalCommandQuickCommands.length).toBeGreaterThan(0);
  });
});

function assertPublicControlTypesAreImportable(_value: PublicControlTypes): void {}

assertPublicControlTypesAreImportable(null as never);
