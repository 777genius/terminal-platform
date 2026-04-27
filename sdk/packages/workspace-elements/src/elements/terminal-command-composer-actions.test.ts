import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_COMPOSER_ACTIONS,
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
  resolveTerminalCommandComposerActionPlacement,
  resolveTerminalCommandComposerActions,
} from "./terminal-command-composer-actions.js";

describe("terminal command composer actions", () => {
  it("keeps terminal command actions in a stable ergonomic order", () => {
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.id)).toEqual([
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit,
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste,
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt,
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.enter,
    ]);
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.part)).toEqual([
      "send-command",
      "paste-clipboard",
      "send-interrupt",
      "send-enter",
    ]);
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.testId)).toEqual([
      "tp-send-command",
      "tp-paste-clipboard",
      "tp-send-interrupt",
      "tp-send-enter",
    ]);
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.keyHint ?? null)).toEqual([
      "Enter",
      null,
      "Ctrl+C",
      "Enter",
    ]);
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.tone)).toEqual([
      "primary",
      "secondary",
      "secondary",
      "secondary",
    ]);
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.primary)).toEqual([true, false, false, false]);
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.placement)).toEqual([
      "panel",
      "panel",
      "panel",
      "panel",
    ]);
  });

  it("overrides paste title without mutating the default action contract", () => {
    const actions = resolveTerminalCommandComposerActions({
      pasteTitle: "Paste from browser clipboard",
    });
    const paste = actions.find((action) => action.id === TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste);

    expect(paste?.title).toBe("Paste from browser clipboard");
    expect(paste?.ariaLabel).toBe("Paste from browser clipboard");
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS[1]?.title).toBe(TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE);
  });

  it("keeps interrupt and enter modeled as explicit terminal shortcuts", () => {
    const shortcuts = resolveTerminalCommandComposerActions()
      .filter((action) => action.shortcut)
      .map((action) => [action.id, action.shortcut]);

    expect(shortcuts).toEqual([
      [TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt, "\u0003"],
      [TERMINAL_COMMAND_COMPOSER_ACTION_IDS.enter, "\r"],
    ]);
  });

  it("only advertises real UI keyboard shortcuts through aria-keyshortcuts", () => {
    const ariaShortcuts = resolveTerminalCommandComposerActions()
      .filter((action) => action.ariaKeyShortcuts)
      .map((action) => [action.id, action.ariaKeyShortcuts]);

    expect(ariaShortcuts).toEqual([[TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit, "Enter"]]);
  });

  it("resolves compact terminal-placement action labels without changing accessible names", () => {
    const actions = resolveTerminalCommandComposerActions({ placement: "terminal" });

    expect(actions.map((action) => action.placement)).toEqual([
      "terminal",
      "terminal",
      "terminal",
      "terminal",
    ]);
    expect(actions.map((action) => action.label)).toEqual(["Run", "Paste", "^C", "\u21b5"]);
    expect(actions.at(-1)?.ariaLabel).toBe("Send Enter to the focused pane");
    expect(actions.at(-1)?.title).toBe("Send Enter to the focused pane");
  });

  it("normalizes unknown action placement to the panel contract", () => {
    expect(resolveTerminalCommandComposerActionPlacement("terminal")).toBe("terminal");
    expect(resolveTerminalCommandComposerActionPlacement("panel")).toBe("panel");
    expect(resolveTerminalCommandComposerActionPlacement("unknown")).toBe("panel");
    expect(resolveTerminalCommandComposerActions({ placement: "unknown" }).map((action) => action.label)).toEqual([
      "Run",
      "Paste",
      "^C",
      "Enter",
    ]);
  });
});
