import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

type FocusedScreen = NonNullable<NonNullable<WorkspaceSnapshot["attachedSession"]>["focused_screen"]>;
type TerminalDisplay = WorkspaceSnapshot["terminalDisplay"];

export const TERMINAL_SCREEN_CHROME_MODES = {
  compact: "compact",
  full: "full",
} as const;

export type TerminalScreenChromeMode =
  (typeof TERMINAL_SCREEN_CHROME_MODES)[keyof typeof TERMINAL_SCREEN_CHROME_MODES];

export type TerminalScreenChromeMetaItemId =
  | "cursor"
  | "fontScale"
  | "sequence"
  | "source"
  | "size"
  | "wrap";

export type TerminalScreenChromeMetaItem = {
  readonly id: TerminalScreenChromeMetaItemId;
  readonly label: string;
  readonly title?: string;
};

export type TerminalScreenChromeOptions = {
  mode?: TerminalScreenChromeMode | null;
};

export type TerminalScreenChromeState = {
  readonly mode: TerminalScreenChromeMode;
  readonly title: string;
  readonly metaItems: readonly TerminalScreenChromeMetaItem[];
};

export function resolveTerminalScreenChromeState(
  screen: FocusedScreen,
  terminalDisplay: TerminalDisplay,
  options: TerminalScreenChromeOptions = {},
): TerminalScreenChromeState {
  const mode = normalizeTerminalScreenChromeMode(options.mode);
  const title = normalizeTerminalScreenTitle(screen.surface.title);

  return {
    mode,
    title,
    metaItems: mode === TERMINAL_SCREEN_CHROME_MODES.compact
      ? resolveCompactMetaItems(screen, terminalDisplay)
      : resolveFullMetaItems(screen, terminalDisplay),
  };
}

function resolveCompactMetaItems(
  screen: FocusedScreen,
  terminalDisplay: TerminalDisplay,
): readonly TerminalScreenChromeMetaItem[] {
  const items: TerminalScreenChromeMetaItem[] = [
    {
      id: "size",
      label: `${screen.cols}x${screen.rows}`,
      title: `${screen.cols} columns by ${screen.rows} rows`,
    },
    { id: "source", label: screen.source },
    { id: "sequence", label: `seq ${String(screen.sequence)}` },
    { id: "fontScale", label: terminalDisplay.fontScale },
    { id: "wrap", label: terminalDisplay.lineWrap ? "wrapped" : "nowrap" },
  ];

  if (screen.surface.cursor) {
    items.push({
      id: "cursor",
      label: `${screen.surface.cursor.row + 1}:${screen.surface.cursor.col + 1}`,
      title: `cursor ${screen.surface.cursor.row + 1}:${screen.surface.cursor.col + 1}`,
    });
  }

  return items;
}

function resolveFullMetaItems(
  screen: FocusedScreen,
  terminalDisplay: TerminalDisplay,
): readonly TerminalScreenChromeMetaItem[] {
  const items: TerminalScreenChromeMetaItem[] = [
    { id: "size", label: `${screen.cols} columns` },
    { id: "size", label: `${screen.rows} rows` },
    { id: "sequence", label: `seq ${String(screen.sequence)}` },
    { id: "source", label: screen.source },
    { id: "fontScale", label: terminalDisplay.fontScale },
    { id: "wrap", label: terminalDisplay.lineWrap ? "wrapped" : "nowrap" },
  ];

  if (screen.surface.cursor) {
    items.push({
      id: "cursor",
      label: `cursor ${screen.surface.cursor.row + 1}:${screen.surface.cursor.col + 1}`,
    });
  }

  return items;
}

function normalizeTerminalScreenChromeMode(
  mode: TerminalScreenChromeMode | null | undefined,
): TerminalScreenChromeMode {
  return mode === TERMINAL_SCREEN_CHROME_MODES.compact
    ? TERMINAL_SCREEN_CHROME_MODES.compact
    : TERMINAL_SCREEN_CHROME_MODES.full;
}

function normalizeTerminalScreenTitle(title: string | null | undefined): string {
  const normalized = title?.trim();
  return normalized ? normalized : "Live output";
}
