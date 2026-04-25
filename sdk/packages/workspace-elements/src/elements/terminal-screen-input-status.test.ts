import { describe, expect, it } from "vitest";

import type { TerminalScreenControlState } from "./terminal-screen-controls.js";
import { resolveTerminalScreenInputStatus } from "./terminal-screen-input-status.js";

describe("terminal screen input status", () => {
  it("reports ready focused pane input when direct input is available", () => {
    expect(resolveTerminalScreenInputStatus(createControls(), "idle")).toMatchObject({
      label: "Input ready",
      tone: "ready",
    });
  });

  it("keeps pending capability explicit while optimistic direct input is enabled", () => {
    expect(resolveTerminalScreenInputStatus(createControls({
      inputCapabilityStatus: "unknown",
    }), "idle")).toMatchObject({
      label: "Input pending",
      tone: "pending",
    });
  });

  it("reports read only when the active backend rejects direct input", () => {
    expect(resolveTerminalScreenInputStatus(createControls({
      canUseDirectInput: false,
    }), "idle")).toMatchObject({
      label: "Read only",
      tone: "readonly",
    });
  });

  it("reports no input without an attached screen target", () => {
    expect(resolveTerminalScreenInputStatus(createControls({
      activePaneId: null,
      screen: null,
      canUseDirectInput: false,
    }), "idle")).toMatchObject({
      label: "No input",
      tone: "readonly",
    });
  });

  it("surfaces the latest direct input failure over capability state", () => {
    expect(resolveTerminalScreenInputStatus(createControls(), "failed")).toMatchObject({
      label: "Input failed",
      tone: "failed",
    });
  });
});

function createControls(
  overrides: Partial<TerminalScreenControlState> = {},
): TerminalScreenControlState {
  return {
    activeSessionId: "session-1",
    activePaneId: "pane-1",
    screen: {
      pane_id: "pane-1",
      sequence: 1n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      surface: {
        title: "shell",
        cursor: null,
        lines: [{ text: "ready" }],
      },
    },
    canCopyVisibleOutput: true,
    canUseDirectInput: true,
    inputCapabilityStatus: "known",
    ...overrides,
  };
}
