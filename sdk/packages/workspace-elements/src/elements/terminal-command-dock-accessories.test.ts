import { describe, expect, it } from "vitest";

import {
  resolveTerminalCommandDockAccessoryMode,
  resolveTerminalCommandDockAccessoryState,
  TERMINAL_COMMAND_DOCK_ACCESSORY_MODES,
} from "./terminal-command-dock-accessories.js";

describe("terminal command dock accessories", () => {
  it("uses a compact accessory bar for terminal placement", () => {
    expect(resolveTerminalCommandDockAccessoryMode({ placement: "terminal" })).toBe(
      TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.bar,
    );
  });

  it("keeps panel placement in stacked accessory mode", () => {
    expect(resolveTerminalCommandDockAccessoryMode({ placement: "panel" })).toBe(
      TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.stack,
    );
    expect(resolveTerminalCommandDockAccessoryMode({ placement: "unknown" })).toBe(
      TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.stack,
    );
  });

  it("resolves accessory presentation state for responsive terminal layouts", () => {
    expect(resolveTerminalCommandDockAccessoryState({
      placement: "terminal",
      quickCommandCount: 5,
      recentCommandCount: 2,
    })).toEqual({
      mode: TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.bar,
      hasQuickCommands: true,
      hasRecentCommands: true,
      quickCommandCount: 5,
      recentCommandCount: 2,
    });
  });

  it("normalizes missing or invalid accessory counts for first-run terminals", () => {
    expect(resolveTerminalCommandDockAccessoryState({
      placement: "terminal",
      quickCommandCount: Number.NaN,
      recentCommandCount: -12,
    })).toMatchObject({
      hasQuickCommands: false,
      hasRecentCommands: false,
      quickCommandCount: 0,
      recentCommandCount: 0,
    });
  });
});
