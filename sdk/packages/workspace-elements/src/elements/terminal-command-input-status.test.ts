import { describe, expect, it } from "vitest";

import type { TerminalCommandDockControlState } from "./terminal-command-dock-controls.js";
import { resolveTerminalCommandInputStatus } from "./terminal-command-input-status.js";

describe("terminal command input status", () => {
  it("reports ready when the focused pane accepts command input", () => {
    expect(resolveTerminalCommandInputStatus(createControls())).toMatchObject({
      label: "Ready",
      tone: "ready",
    });
  });

  it("reports pending while backend input capability is loading", () => {
    expect(resolveTerminalCommandInputStatus(createControls({
      inputCapabilityStatus: "unknown",
    }))).toMatchObject({
      label: "Input pending",
      tone: "pending",
    });
  });

  it("reports read only when the backend rejects pane input writes", () => {
    expect(resolveTerminalCommandInputStatus(createControls({
      canSend: false,
      canWriteInput: false,
    }))).toMatchObject({
      label: "Read only",
      tone: "idle",
    });
  });

  it("reports busy while a command action is pending", () => {
    expect(resolveTerminalCommandInputStatus(createControls({
      canSend: false,
      canUsePane: false,
      canWriteInput: false,
    }))).toMatchObject({
      label: "Sending",
      tone: "pending",
    });
  });

  it("reports pick a pane without an active target", () => {
    expect(resolveTerminalCommandInputStatus(createControls({
      activeSessionId: null,
      activePaneId: null,
      canSend: false,
      canUsePane: false,
      canWriteInput: false,
    }))).toMatchObject({
      label: "Pick a pane",
      tone: "idle",
    });
  });
});

function createControls(
  overrides: Partial<TerminalCommandDockControlState> = {},
): TerminalCommandDockControlState {
  return {
    activeSessionId: "session-1",
    activePaneId: "pane-1",
    draft: "git status",
    commandHistory: [],
    recentCommands: [],
    canSend: true,
    canUsePane: true,
    canWriteInput: true,
    canPasteClipboard: true,
    canSaveLayout: true,
    inputCapabilityStatus: "known",
    pasteCapabilityStatus: "known",
    saveCapabilityStatus: "known",
    ...overrides,
  };
}
