export const TERMINAL_SCREEN_ACTION_IDS = {
  copyVisible: "copy-visible",
  followOutput: "follow-output",
  scrollLatest: "scroll-latest",
} as const;

export type TerminalScreenActionId =
  (typeof TERMINAL_SCREEN_ACTION_IDS)[keyof typeof TERMINAL_SCREEN_ACTION_IDS];

export type TerminalScreenActionPlacement = "panel" | "terminal";
export type TerminalScreenCopyState = "idle" | "copied" | "failed";
export type TerminalScreenActionTone = "danger" | "primary" | "secondary" | "success";
export type TerminalScreenActionLabelMode = "glyph" | "label";

export interface TerminalScreenActionOptions {
  canCopyVisibleOutput?: boolean;
  copyState?: TerminalScreenCopyState;
  followOutput?: boolean;
  placement?: string | null | undefined;
}

export interface TerminalScreenActionPresentation {
  readonly ariaLabel: string;
  readonly ariaPressed?: boolean;
  readonly disabled: boolean;
  readonly id: TerminalScreenActionId;
  readonly label: string;
  readonly labelMode: TerminalScreenActionLabelMode;
  readonly placement: TerminalScreenActionPlacement;
  readonly testId: string;
  readonly title: string;
  readonly tone: TerminalScreenActionTone;
}

export function resolveTerminalScreenActions(
  options: TerminalScreenActionOptions = {},
): readonly TerminalScreenActionPresentation[] {
  const placement = normalizeTerminalScreenActionPlacement(options.placement);
  const compact = placement === "terminal";
  const followOutput = options.followOutput !== false;
  const copyState = normalizeTerminalScreenCopyState(options.copyState);
  const canCopyVisibleOutput = options.canCopyVisibleOutput === true;

  return [
    {
      id: TERMINAL_SCREEN_ACTION_IDS.followOutput,
      testId: "tp-screen-follow",
      label: resolveFollowOutputLabel(followOutput, compact),
      labelMode: resolveTerminalScreenActionLabelMode(compact),
      placement,
      title: followOutput ? "Pause automatic terminal output follow" : "Follow terminal output",
      ariaLabel: followOutput ? "Pause automatic terminal output follow" : "Follow terminal output",
      ariaPressed: followOutput,
      disabled: false,
      tone: followOutput ? "primary" : "secondary",
    },
    {
      id: TERMINAL_SCREEN_ACTION_IDS.scrollLatest,
      testId: "tp-screen-scroll-latest",
      label: compact ? "\u2193" : "Scroll latest",
      labelMode: resolveTerminalScreenActionLabelMode(compact),
      placement,
      title: "Scroll to latest terminal output",
      ariaLabel: "Scroll to latest terminal output",
      disabled: false,
      tone: "secondary",
    },
    {
      id: TERMINAL_SCREEN_ACTION_IDS.copyVisible,
      testId: "tp-screen-copy",
      label: resolveCopyVisibleLabel(copyState, compact),
      labelMode: resolveTerminalScreenActionLabelMode(compact),
      placement,
      title: resolveCopyVisibleTitle(copyState),
      ariaLabel: resolveCopyVisibleAriaLabel(copyState),
      disabled: !canCopyVisibleOutput,
      tone: resolveCopyVisibleTone(copyState),
    },
  ];
}

function normalizeTerminalScreenActionPlacement(
  placement: string | null | undefined,
): TerminalScreenActionPlacement {
  return placement === "terminal" ? "terminal" : "panel";
}

function normalizeTerminalScreenCopyState(
  copyState: TerminalScreenCopyState | undefined,
): TerminalScreenCopyState {
  return copyState === "copied" || copyState === "failed" ? copyState : "idle";
}

function resolveFollowOutputLabel(followOutput: boolean, compact: boolean): string {
  if (!followOutput) {
    return compact ? "\u25b6" : "Paused";
  }

  return compact ? "\u23f8" : "Following";
}

function resolveCopyVisibleLabel(copyState: TerminalScreenCopyState, compact: boolean): string {
  if (copyState === "copied") {
    return compact ? "\u2713" : "Copied";
  }

  if (copyState === "failed") {
    return compact ? "!" : "Copy failed";
  }

  return compact ? "\u2398" : "Copy visible";
}

function resolveCopyVisibleTitle(copyState: TerminalScreenCopyState): string {
  if (copyState === "copied") {
    return "Visible terminal output copied";
  }

  if (copyState === "failed") {
    return "Visible terminal output could not be copied";
  }

  return "Copy visible terminal output";
}

function resolveCopyVisibleAriaLabel(copyState: TerminalScreenCopyState): string {
  if (copyState === "copied") {
    return "Visible terminal output copied";
  }

  if (copyState === "failed") {
    return "Copy visible terminal output failed";
  }

  return "Copy visible terminal output";
}

function resolveCopyVisibleTone(copyState: TerminalScreenCopyState): TerminalScreenActionTone {
  if (copyState === "copied") {
    return "success";
  }

  if (copyState === "failed") {
    return "danger";
  }

  return "secondary";
}

function resolveTerminalScreenActionLabelMode(compact: boolean): TerminalScreenActionLabelMode {
  return compact ? "glyph" : "label";
}
