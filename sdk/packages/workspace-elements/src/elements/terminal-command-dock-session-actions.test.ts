import { describe, expect, it } from "vitest";

import type { TerminalCommandDockControlState } from "./terminal-command-dock-controls.js";
import {
  TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS,
  resolveTerminalCommandDockSessionActions,
} from "./terminal-command-dock-session-actions.js";

describe("terminal command dock session actions", () => {
  it("uses compact command labels for terminal placement", () => {
    const actions = resolveTerminalCommandDockSessionActions(createControls(), {
      placement: "terminal",
    });

    expect(actions.map((action) => action.id)).toEqual([
      TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.saveLayout,
      TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.refreshTerminal,
      TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.clearCommandHistory,
    ]);
    expect(actions.map((action) => action.label)).toEqual(["Save", "Refresh", "Clear"]);
    expect(actions.map((action) => action.testId)).toEqual([
      "tp-save-layout",
      "tp-refresh-terminal",
      "tp-clear-command-history",
    ]);
    expect(actions[0]?.title).toBe("Save the focused session layout");
    expect(actions[1]?.ariaLabel).toBe("Refresh the active terminal session");
  });

  it("keeps descriptive labels for panel placement", () => {
    const actions = resolveTerminalCommandDockSessionActions(createControls(), {
      placement: "panel",
    });

    expect(actions.map((action) => action.label)).toEqual([
      "Save layout",
      "Refresh terminal",
      "Clear history",
    ]);
  });

  it("models degraded save-layout capability in presentation state", () => {
    expect(resolveTerminalCommandDockSessionActions(createControls({
      canSaveLayout: false,
      saveCapabilityStatus: "known",
    }))[0]).toMatchObject({
      disabled: true,
      title: "Save layout is not supported by the active backend",
    });

    expect(resolveTerminalCommandDockSessionActions(createControls({
      canSaveLayout: false,
      saveCapabilityStatus: "unknown",
    }))[0]).toMatchObject({
      disabled: true,
      title: "Save layout is disabled until backend capabilities load",
    });
  });

  it("requires an explicit confirmation state before clearing command history", () => {
    const clearAction = resolveTerminalCommandDockSessionActions(createControls(), {
      historyClearConfirmationArmed: true,
      placement: "terminal",
    }).at(-1);

    expect(clearAction).toMatchObject({
      confirming: true,
      dangerous: true,
      disabled: false,
      historyCount: 2,
      label: "Confirm clear 2",
      title: "Confirm clearing 2 command history entries",
    });
  });

  it("disables session actions while pending or missing required state", () => {
    const pendingActions = resolveTerminalCommandDockSessionActions(createControls(), {
      historyClearConfirmationArmed: true,
      pending: true,
      placement: "terminal",
    });

    expect(pendingActions[1]).toMatchObject({ disabled: true, label: "Refresh" });
    expect(pendingActions[2]).toMatchObject({ confirming: false, disabled: true, label: "Clear" });

    const idleActions = resolveTerminalCommandDockSessionActions(createControls({
      activeSessionId: null,
      commandHistory: [],
      canSaveLayout: false,
    }), {
      placement: "terminal",
    });

    expect(idleActions.map((action) => action.disabled)).toEqual([true, true, true]);
  });
});

function createControls(
  overrides: Partial<TerminalCommandDockControlState> = {},
): TerminalCommandDockControlState {
  return {
    activePaneId: "pane-1",
    activeSessionId: "session-1",
    canPasteClipboard: true,
    canSaveLayout: true,
    canSend: true,
    canUsePane: true,
    canWriteInput: true,
    commandHistory: ["pwd", "git status"],
    draft: "git status",
    inputCapabilityStatus: "known",
    pasteCapabilityStatus: "known",
    recentCommands: ["git status", "pwd"],
    saveCapabilityStatus: "known",
    ...overrides,
  };
}
