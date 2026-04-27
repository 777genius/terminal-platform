export const TERMINAL_COMMAND_COMPOSER_ACTION_IDS = {
  enter: "enter",
  interrupt: "interrupt",
  paste: "paste",
  submit: "submit",
} as const;

export type TerminalCommandComposerActionId =
  (typeof TERMINAL_COMMAND_COMPOSER_ACTION_IDS)[keyof typeof TERMINAL_COMMAND_COMPOSER_ACTION_IDS];

export type TerminalCommandComposerActionPlacement = "panel" | "terminal";

export type TerminalCommandComposerShortcut = "\u0003" | "\r";

export type TerminalCommandComposerActionTone = "primary" | "secondary";

export type TerminalCommandComposerActionPresentation = {
  readonly id: TerminalCommandComposerActionId;
  readonly ariaKeyShortcuts?: string;
  readonly ariaLabel: string;
  readonly keyHint?: string;
  readonly label: string;
  readonly placement: TerminalCommandComposerActionPlacement;
  readonly part: string;
  readonly primary: boolean;
  readonly shortcut?: TerminalCommandComposerShortcut;
  readonly testId: string;
  readonly title: string;
  readonly tone: TerminalCommandComposerActionTone;
};

export type TerminalCommandComposerActionOptions = {
  pasteTitle?: string | null;
  placement?: string | null;
};

export const TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE = "Paste clipboard into the focused pane";

type TerminalCommandComposerActionDefinition =
  Omit<TerminalCommandComposerActionPresentation, "label" | "placement"> & {
    readonly label: string;
    readonly labels: Readonly<Record<TerminalCommandComposerActionPlacement, string>>;
  };

const terminalCommandComposerActions = [
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit,
    ariaLabel: "Send command to the focused pane",
    ariaKeyShortcuts: "Enter",
    keyHint: "Enter",
    label: "Run",
    labels: {
      panel: "Run",
      terminal: "Run",
    },
    part: "send-command",
    primary: true,
    testId: "tp-send-command",
    title: "Send command to the focused pane",
    tone: "primary",
  },
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste,
    ariaLabel: TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
    label: "Paste",
    labels: {
      panel: "Paste",
      terminal: "Paste",
    },
    part: "paste-clipboard",
    primary: false,
    testId: "tp-paste-clipboard",
    title: TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE,
    tone: "secondary",
  },
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.interrupt,
    ariaLabel: "Send Ctrl+C to the focused pane",
    keyHint: "Ctrl+C",
    label: "^C",
    labels: {
      panel: "^C",
      terminal: "^C",
    },
    part: "send-interrupt",
    primary: false,
    shortcut: "\u0003",
    testId: "tp-send-interrupt",
    title: "Send Ctrl+C to the focused pane",
    tone: "secondary",
  },
  {
    id: TERMINAL_COMMAND_COMPOSER_ACTION_IDS.enter,
    ariaLabel: "Send Enter to the focused pane",
    keyHint: "Enter",
    label: "Enter",
    labels: {
      panel: "Enter",
      terminal: "\u21b5",
    },
    part: "send-enter",
    primary: false,
    shortcut: "\r",
    testId: "tp-send-enter",
    title: "Send Enter to the focused pane",
    tone: "secondary",
  },
] as const satisfies readonly TerminalCommandComposerActionDefinition[];

export const TERMINAL_COMMAND_COMPOSER_ACTIONS: readonly TerminalCommandComposerActionPresentation[] =
  resolveTerminalCommandComposerActions();

export function resolveTerminalCommandComposerActions(
  options: TerminalCommandComposerActionOptions = {},
): readonly TerminalCommandComposerActionPresentation[] {
  const pasteTitle = normalizeOptionalLabel(options.pasteTitle) ?? TERMINAL_COMMAND_COMPOSER_DEFAULT_PASTE_TITLE;
  const placement = resolveTerminalCommandComposerActionPlacement(options.placement);

  return terminalCommandComposerActions.map((action) => {
    const label = resolveTerminalCommandComposerActionLabel(action, placement);
    const { labels: _labels, ...baseAction } = action;
    const presentation = {
      ...baseAction,
      label,
      placement,
    };

    return action.id === TERMINAL_COMMAND_COMPOSER_ACTION_IDS.paste
      ? {
          ...presentation,
          ariaLabel: pasteTitle,
          title: pasteTitle,
        }
      : presentation;
  });
}

export function resolveTerminalCommandComposerActionPlacement(
  placement: string | null | undefined,
): TerminalCommandComposerActionPlacement {
  return placement === "terminal" ? "terminal" : "panel";
}

function normalizeOptionalLabel(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized ? normalized : null;
}

function resolveTerminalCommandComposerActionLabel(
  action: (typeof terminalCommandComposerActions)[number],
  placement: TerminalCommandComposerActionPlacement,
): string {
  return action.labels[placement] ?? action.label;
}
