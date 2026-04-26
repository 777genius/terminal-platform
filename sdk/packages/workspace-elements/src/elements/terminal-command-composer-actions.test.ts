import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_COMPOSER_ACTIONS,
  TERMINAL_COMMAND_COMPOSER_ACTION_IDS,
  TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
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
});
