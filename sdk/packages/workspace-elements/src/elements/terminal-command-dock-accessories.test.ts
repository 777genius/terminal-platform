import { describe, expect, it } from "vitest";

import {
  resolveTerminalCommandDockAccessoryMode,
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
});
