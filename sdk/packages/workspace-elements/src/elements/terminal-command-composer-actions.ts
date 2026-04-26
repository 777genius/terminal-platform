export const TERMINAL_COMMAND_COMPOSER_ACTION_IDS = {
  enter: "enter",
  interrupt: "interrupt",
  paste: "paste",
  submit: "submit",
} as const;

export type TerminalCommandComposerActionId =
  (typeof TERMINAL_COMMAND_COMPOSER_ACTION_IDS)[keyof typeof TERMINAL_COMMAND_COMPOSER_ACTION_IDS];

export type TerminalCommandComposerShortcut = "\u0003" | "\r";

export type TerminalCommandComposerActionPresentation = {
  readonly id: TerminalCommandComposerActionId;
  readonly ariaKeyShortcuts?: string;
  readonly ariaLabel: string;
  readonly keyHint?: string;
  readonly label: string;
  readonly part: string;
  readonly primary: boolean;
  readonly shortcut?: TerminalCommandComposerShortcut;
  readonly testId: string;
  readonly title: string;
};

export type TerminalCommandComposerActionOptions = {
  pasteTitle?: string | null;
};

export const TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE = "Paste clipboard into the focused pane";

const terminalCommandComposerActions = [
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit,
    ariaLabel: "Send command to the focused pane",
    ariaKeyShortcuts: "Enter",
    keyHint: "Enter",
    label: "Run",
    part: "send-command",
    primary: true,
    testId: "tp-send-command",
    title: "Send command to the focused pane",
  },
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste,
    ariaLabel: TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
    label: "Paste",
    part: "paste-clipboard",
    primary: false,
    testId: "tp-paste-clipboard",
    title: TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
  },
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt,
    ariaLabel: "Send Ctrl+C to the focused pane",
    keyHint: "Ctrl+C",
    label: "^C",
    part: "send-interrupt",
    primary: false,
    shortcut: "\u0003",
    testId: "tp-send-interrupt",
    title: "Send Ctrl+C to the focused pane",
  },
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.enter,
    ariaLabel: "Send Enter to the focused pane",
    keyHint: "Enter",
    label: "Enter",
    part: "send-enter",
    primary: false,
    shortcut: "\r",
    testId: "tp-send-enter",
    title: "Send Enter to the focused pane",
  },
] as const satisfies readonly TerminalCommandComposerActionPresentation[];

export const TERMINAL_COMMAND_COMPOSER_ACTIONS: readonly TerminalCommandComposerActionPresentation[] =
  terminalCommandComposerActions;

export function resolveTerminalCommandComposerActions(
  options: TerminalCommandComposerActionOptions = {},
): readonly TerminalCommandComposerActionPresentation[] {
  const pasteTitle = normalizeOptionalLabel(options.pasteTitle) ?? TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE;

  return terminalCommandComposerActions.map((action) => {
    if (action.id !== TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste) {
      return action;
    }

    return {
      ...action,
      ariaLabel: pasteTitle,
      title: pasteTitle,
    };
  });
}

function normalizeOptionalLabel(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized ? normalized : null;
}
