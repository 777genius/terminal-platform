export const TERMINAL_SCREEN_SEARCH_ACTION_IDS = {
  clearSearch: "clear-search",
  nextMatch: "next-match",
  previousMatch: "previous-match",
} as const;

export type TerminalScreenSearchActionId =
  (typeof TERMINAL_SCREEN_SEARCH_ACTION_IDS)[keyof typeof TERMINAL_SCREEN_SEARCH_ACTION_IDS];

export type TerminalScreenSearchActionLabelMode = "glyph" | "label";
export type TerminalScreenSearchActionPlacement = "panel" | "terminal";
export type TerminalScreenSearchActionTone = "secondary";

export interface TerminalScreenSearchActionOptions {
  matchCount?: number | null | undefined;
  placement?: string | null | undefined;
  query?: string | null | undefined;
}

export interface TerminalScreenSearchActionPresentation {
  readonly ariaLabel: string;
  readonly disabled: boolean;
  readonly id: TerminalScreenSearchActionId;
  readonly label: string;
  readonly labelMode: TerminalScreenSearchActionLabelMode;
  readonly placement: TerminalScreenSearchActionPlacement;
  readonly testId: string;
  readonly title: string;
  readonly tone: TerminalScreenSearchActionTone;
}

export function resolveTerminalScreenSearchActions(
  options: TerminalScreenSearchActionOptions = {},
): readonly TerminalScreenSearchActionPresentation[] {
  const placement = normalizeTerminalScreenSearchActionPlacement(options.placement);
  const compact = placement === "terminal";
  const labelMode = resolveTerminalScreenSearchActionLabelMode(compact);
  const matchCount = normalizeSearchMatchCount(options.matchCount);
  const hasMatches = matchCount > 0;
  const hasQuery = typeof options.query === "string" && options.query.length > 0;

  return [
    {
      id: TERMINAL_SCREEN_SEARCH_ACTION_IDS.previousMatch,
      testId: "tp-screen-search-prev",
      label: compact ? "\u2191" : "Prev",
      labelMode,
      placement,
      title: "Select previous search match",
      ariaLabel: "Select previous search match",
      disabled: !hasMatches,
      tone: "secondary",
    },
    {
      id: TERMINAL_SCREEN_SEARCH_ACTION_IDS.nextMatch,
      testId: "tp-screen-search-next",
      label: compact ? "\u2193" : "Next",
      labelMode,
      placement,
      title: "Select next search match",
      ariaLabel: "Select next search match",
      disabled: !hasMatches,
      tone: "secondary",
    },
    {
      id: TERMINAL_SCREEN_SEARCH_ACTION_IDS.clearSearch,
      testId: "tp-screen-search-clear",
      label: compact ? "\u00d7" : "Clear",
      labelMode,
      placement,
      title: "Clear search query",
      ariaLabel: "Clear search query",
      disabled: !hasQuery,
      tone: "secondary",
    },
  ];
}

function normalizeTerminalScreenSearchActionPlacement(
  placement: string | null | undefined,
): TerminalScreenSearchActionPlacement {
  return placement === "terminal" ? "terminal" : "panel";
}

function normalizeSearchMatchCount(matchCount: number | null | undefined): number {
  if (typeof matchCount !== "number" || !Number.isFinite(matchCount)) {
    return 0;
  }

  return Math.max(0, Math.trunc(matchCount));
}

function resolveTerminalScreenSearchActionLabelMode(
  compact: boolean,
): TerminalScreenSearchActionLabelMode {
  return compact ? "glyph" : "label";
}
