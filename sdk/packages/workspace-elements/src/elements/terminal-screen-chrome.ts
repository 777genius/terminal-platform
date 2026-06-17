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
  | "progress"
  | "sequence"
  | "source"
  | "size"
  | "workingDirectory"
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
    const cursorLabel = formatCursorLabel(screen.surface.cursor);
    items.push({
      id: "cursor",
      label: cursorLabel,
      title: `cursor ${cursorLabel}`,
    });
  }
  const progress = formatTerminalProgress(screen.surface.progress, "compact");
  if (progress) {
    items.push(progress);
  }
  const workingDirectoryUri = screen.surface.working_directory_uri;
  const workingDirectory = formatWorkingDirectoryUri(workingDirectoryUri);
  if (workingDirectory && workingDirectoryUri) {
    items.push({
      id: "workingDirectory",
      label: workingDirectory,
      title: workingDirectoryUri,
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
      label: `cursor ${formatCursorLabel(screen.surface.cursor)}`,
    });
  }
  const progress = formatTerminalProgress(screen.surface.progress, "full");
  if (progress) {
    items.push(progress);
  }
  const workingDirectoryUri = screen.surface.working_directory_uri;
  const workingDirectory = formatWorkingDirectoryUri(workingDirectoryUri);
  if (workingDirectory && workingDirectoryUri) {
    items.push({
      id: "workingDirectory",
      label: `cwd ${workingDirectory}`,
      title: workingDirectoryUri,
    });
  }

  return items;
}

function formatWorkingDirectoryUri(uri: string | null | undefined): string {
  const normalized = uri?.trim();
  if (!normalized) {
    return "";
  }
  if (!normalized.startsWith("file://")) {
    return normalized;
  }

  try {
    const parsed = new URL(normalized);
    const path = decodeURIComponent(parsed.pathname || "/");
    const host = parsed.hostname;
    return host && host !== "localhost" ? `${host}:${path}` : path;
  } catch {
    return normalized;
  }
}

function formatTerminalProgress(
  progress: FocusedScreen["surface"]["progress"],
  mode: TerminalScreenChromeMode,
): TerminalScreenChromeMetaItem | null {
  if (!progress || progress.state === "inactive") {
    return null;
  }

  const normalizedValue =
    typeof progress.value === "number" && Number.isFinite(progress.value)
      ? Math.max(0, Math.min(100, Math.trunc(progress.value)))
      : null;
  const suffix = normalizedValue === null ? "" : ` ${normalizedValue}%`;

  switch (progress.state) {
    case "normal":
      return {
        id: "progress",
        label: normalizedValue === null
          ? mode === TERMINAL_SCREEN_CHROME_MODES.compact ? "progress" : "progress active"
          : mode === TERMINAL_SCREEN_CHROME_MODES.compact
            ? `${normalizedValue}%`
            : `progress ${normalizedValue}%`,
        title: normalizedValue === null
          ? "Terminal progress active"
          : `Terminal progress ${normalizedValue}%`,
      };
    case "error":
      return {
        id: "progress",
        label: mode === TERMINAL_SCREEN_CHROME_MODES.compact
          ? `error${suffix}`
          : `progress error${suffix}`,
        title: `Terminal progress error${suffix}`,
      };
    case "warning":
      return {
        id: "progress",
        label: mode === TERMINAL_SCREEN_CHROME_MODES.compact
          ? `warn${suffix}`
          : `progress warning${suffix}`,
        title: `Terminal progress warning${suffix}`,
      };
    case "indeterminate":
      return {
        id: "progress",
        label: mode === TERMINAL_SCREEN_CHROME_MODES.compact
          ? "pending"
          : "progress pending",
        title: "Terminal progress indeterminate",
      };
    default:
      return null;
  }
}

function formatCursorLabel(
  cursor: FocusedScreen["surface"]["cursor"],
): string {
  if (!cursor) {
    return "";
  }

  const location = `${cursor.row + 1}:${cursor.col + 1}`;
  const shape = cursor.shape ? cursor.shape.replace(/_/g, " ") : "";
  const blink = cursor.blinking ? " blinking" : "";

  return shape ? `${location} ${shape}${blink}` : `${location}${blink}`;
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
