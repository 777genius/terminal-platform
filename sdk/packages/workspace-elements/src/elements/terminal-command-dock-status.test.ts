import { describe, expect, it } from "vitest";

import type { TerminalCommandDockControlState } from "./terminal-command-dock-controls.js";
import type { TerminalCommandInputStatus } from "./terminal-command-input-status.js";
import {
  TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS,
  resolveTerminalCommandDockStatusBadges,
  resolveTerminalCommandDockStatusPlacement,
} from "./terminal-command-dock-status.js";

describe("terminal command dock status", () => {
  it("resolves stable badge presentations for custom command dock surfaces", () => {
    const badges = resolveTerminalCommandDockStatusBadges(createControls(), createInputStatus(), {
      placement: "panel",
    });

    expect(badges.map((badge) => [badge.id, badge.label, badge.testId, badge.tone])).toEqual([
      [TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.activePane, "Pane pane-1", "tp-command-active-pane", "ready"],
      [TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.input, "Ready", "tp-command-input-status", "ready"],
      [TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.historyCount, "3 history", "tp-command-history-count", "idle"],
    ]);
    expect(badges.at(-1)?.title).toBe("3 command history entries");
  });

  it("uses a compact command history badge for terminal placement", () => {
    const badges = resolveTerminalCommandDockStatusBadges(createControls(), createInputStatus(), {
      placement: "terminal",
    });

    expect(badges.at(-1)?.label).toBe("3 cmd");
    expect(badges.at(-1)?.title).toBe("3 command history entries");
  });

  it("keeps missing pane and singular command history states explicit", () => {
    const badges = resolveTerminalCommandDockStatusBadges(
      createControls({
        activePaneId: null,
        commandHistory: ["pwd"],
      }),
      createInputStatus({
        label: "Pick a pane",
        tone: "idle",
      }),
    );

    expect(badges.map((badge) => [badge.id, badge.label, badge.title, badge.tone])).toEqual([
      [TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.activePane, "No pane", "", "idle"],
      [TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.input, "Pick a pane", "Focused pane accepts command input.", "idle"],
      [TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.historyCount, "1 history", "1 command history entry", "idle"],
    ]);
  });

  it("normalizes unknown status placement to the panel contract", () => {
    expect(resolveTerminalCommandDockStatusPlacement("terminal")).toBe("terminal");
    expect(resolveTerminalCommandDockStatusPlacement("panel")).toBe("panel");
    expect(resolveTerminalCommandDockStatusPlacement("unknown")).toBe("panel");
  });
});

function createControls(
  overrides: Partial<TerminalCommandDockControlState> = {},
): TerminalCommandDockControlState {
  return {
    activeSessionId: "session-1",
    activePaneId: "pane-1",
    draft: "git status",
    commandHistory: ["pwd", "ls -la", "git status"],
    recentCommands: ["git status", "ls -la"],
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

function createInputStatus(
  overrides: Partial<TerminalCommandInputStatus> = {},
): TerminalCommandInputStatus {
  return {
    hint: "Enter sends the command.",
    label: "Ready",
    placeholder: "Type shell input for the focused pane",
    title: "Focused pane accepts command input.",
    tone: "ready",
    ...overrides,
  };
}
