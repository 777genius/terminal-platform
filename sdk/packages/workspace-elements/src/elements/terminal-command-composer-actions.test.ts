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
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.id),
    ).toEqual([
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit,
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste,
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt,
      TERMINAL_COMMAND_COMPOSER_ACTION_IDS.enter,
    ]);
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.part),
    ).toEqual([
      "send-command",
      "paste-clipboard",
      "send-interrupt",
      "send-enter",
    ]);
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.testId),
    ).toEqual([
      "tp-send-command",
      "tp-paste-clipboard",
      "tp-send-interrupt",
      "tp-send-enter",
    ]);
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.keyHint ?? null),
    ).toEqual(["Enter", null, "Ctrl+C", "Enter"]);
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.tone),
    ).toEqual(["primary", "secondary", "secondary", "secondary"]);
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.primary),
    ).toEqual([true, false, false, false]);
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.labelMode),
    ).toEqual(["label", "label", "label", "label"]);
    expect(
      TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.placement),
    ).toEqual(["panel", "panel", "panel", "panel"]);
  });

  it("overrides paste title without mutating the default action contract", () => {
    const actions = resolveTerminalCommandComposerActions({
      pasteTitle: "Paste from browser clipboard",
    });
    const paste = actions.find(
      (action) => action.id === TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste,
    );

    expect(paste?.title).toBe("Paste from browser clipboard");
    expect(paste?.ariaLabel).toBe("Paste from browser clipboard");
    expect(TERMINAL_COMMAND_COMPOSER_ACTIONS[1]?.title).toBe(
      TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
    );
  });

  it("overrides visible action labels for host localization", () => {
    const actions = resolveTerminalCommandComposerActions({
      actionLabels: {
        [TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit]: {
          ariaLabel: "Localized submit aria",
          label: "Start",
          title: "Localized submit title",
        },
        [TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt]: {
          label: "Stop",
          title: "Localized interrupt title",
        },
      },
      placement: "terminal",
      terminalActions: {
        canInterrupt: true,
        canSend: true,
      },
    });

    expect(
      actions.map((action) => [
        action.id,
        action.label,
        action.title,
        action.ariaLabel,
      ]),
    ).toEqual([
      [
        TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit,
        "Start",
        "Localized submit title",
        "Localized submit aria",
      ],
      [
        TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt,
        "Stop",
        "Localized interrupt title",
        "Localized interrupt title",
      ],
    ]);
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

    expect(ariaShortcuts).toEqual([
      [TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit, "Enter"],
    ]);
  });

  it("resolves readable terminal-placement labels without changing accessible names", () => {
    const actions = resolveTerminalCommandComposerActions({
      placement: "terminal",
    });

    expect(actions.map((action) => action.placement)).toEqual([
      "terminal",
      "terminal",
      "terminal",
      "terminal",
    ]);
    expect(actions.map((action) => action.label)).toEqual([
      "Run",
      "Paste",
      "Ctrl+C",
      "Enter",
    ]);
    expect(actions.map((action) => action.labelMode)).toEqual([
      "label",
      "label",
      "label",
      "label",
    ]);
    expect(actions.at(0)?.ariaLabel).toBe("Send command to the focused pane");
    expect(actions.at(-1)?.ariaLabel).toBe("Send Enter to the focused pane");
    expect(actions.at(-1)?.title).toBe("Send Enter to the focused pane");
  });

  it("filters terminal-placement action buttons from command state", () => {
    expect(
      resolveTerminalCommandComposerActions({
        placement: "terminal",
        terminalActions: {
          canInterrupt: false,
          canSend: false,
        },
      }),
    ).toEqual([]);

    expect(
      resolveTerminalCommandComposerActions({
        placement: "terminal",
        terminalActions: {
          canInterrupt: false,
          canSend: true,
        },
      }).map((action) => action.id),
    ).toEqual([TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit]);

    expect(
      resolveTerminalCommandComposerActions({
        placement: "terminal",
        terminalActions: {
          canInterrupt: true,
          canSend: false,
        },
      }).map((action) => action.id),
    ).toEqual([TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt]);

    expect(
      resolveTerminalCommandComposerActions({
        placement: "terminal",
        terminalActions: {
          canInterrupt: true,
          canSend: true,
        },
      }).map((action) => [action.id, action.label]),
    ).toEqual([
      [TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit, "Run"],
      [TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt, "Ctrl+C"],
    ]);
  });

  it("normalizes unknown action placement to the panel contract", () => {
    expect(resolveTerminalCommandComposerActionPlacement("terminal")).toBe(
      "terminal",
    );
    expect(resolveTerminalCommandComposerActionPlacement("panel")).toBe(
      "panel",
    );
    expect(resolveTerminalCommandComposerActionPlacement("unknown")).toBe(
      "panel",
    );
    expect(
      resolveTerminalCommandComposerActions({ placement: "unknown" }).map(
        (action) => action.label,
      ),
    ).toEqual(["Run", "Paste", "^C", "Enter"]);
  });
});
