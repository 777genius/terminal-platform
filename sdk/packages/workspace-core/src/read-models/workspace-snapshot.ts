import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  BackendKind,
  DiscoveredSession,
  Handshake,
  PaneId,
  SavedSessionSummary,
  ScreenLine,
  ScreenSurfacePalette,
  SessionId,
  SessionSummary,
} from "@terminal-platform/runtime-types";
import type { WorkspaceErrorShape } from "@terminal-platform/workspace-contracts";

export type WorkspaceConnectionState =
  | "idle"
  | "bootstrapping"
  | "ready"
  | "error"
  | "disposed";
export type WorkspaceDiagnosticSeverity = "info" | "warn" | "error";

export const DEFAULT_WORKSPACE_THEME_ID = "terminal-platform-default" as const;
export const DEFAULT_TERMINAL_FONT_SCALE = "default" as const;
export const DEFAULT_COMMAND_HISTORY_LIMIT = 50 as const;

export const terminalPlatformWorkspaceThemeIds = [
  DEFAULT_WORKSPACE_THEME_ID,
  "terminal-platform-light",
] as const;

export const terminalPlatformTerminalFontScales = [
  "compact",
  DEFAULT_TERMINAL_FONT_SCALE,
  "large",
] as const;

export type TerminalPlatformWorkspaceThemeId =
  (typeof terminalPlatformWorkspaceThemeIds)[number];
export type TerminalPlatformTerminalFontScale =
  (typeof terminalPlatformTerminalFontScales)[number];

export interface WorkspaceDiagnosticRecord {
  code: string;
  message: string;
  severity: WorkspaceDiagnosticSeverity;
  recoverable: boolean;
  timestampMs: number;
  cause?: unknown;
}

export interface WorkspaceConnectionSnapshot {
  state: WorkspaceConnectionState;
  handshake: Handshake | null;
  lastError: WorkspaceErrorShape | null;
}

export interface WorkspaceCatalogSnapshot {
  sessions: SessionSummary[];
  savedSessions: SavedSessionSummary[];
  discoveredSessions: Partial<Record<BackendKind, DiscoveredSession[]>>;
  backendCapabilities: Partial<Record<BackendKind, BackendCapabilitiesInfo>>;
}

export interface WorkspaceSelectionSnapshot {
  activeSessionId: SessionId | null;
  activePaneId: PaneId | null;
}

export interface WorkspaceThemeSnapshot {
  themeId: string;
}

export interface WorkspaceTerminalDisplaySnapshot {
  fontScale: TerminalPlatformTerminalFontScale;
  lineWrap: boolean;
}

export interface WorkspaceCommandHistorySnapshot {
  entries: string[];
  limit: number;
}

export interface WorkspaceHistoricalPaneSnapshot {
  sessionId: SessionId;
  paneId: PaneId;
  sourceSessionId: SessionId;
  sourcePaneId: PaneId;
  source: "saved_session_restore" | "v2_pane_history";
  replayStrategy:
    | "empty"
    | "raw_vt_stream"
    | "rendered_snapshot"
    | "mixed"
    | "degraded";
  restoreGuaranteeLevel: string;
  lines: string[];
  richLines?: ScreenLine[];
  surfacePalette?: ScreenSurfacePalette;
  capturedAtMs: bigint;
  hasGaps: boolean;
  hasMoreSegments: boolean;
  fromEventSeq: bigint;
  nextEventSeq: bigint | null;
  segmentCount: number;
  loadedPayloadBytes: bigint;
  streamStartsWithLineBreak?: boolean;
  streamEndsWithLineBreak?: boolean;
}

export interface WorkspaceSnapshot {
  connection: WorkspaceConnectionSnapshot;
  catalog: WorkspaceCatalogSnapshot;
  selection: WorkspaceSelectionSnapshot;
  attachedSession: AttachedSession | null;
  diagnostics: WorkspaceDiagnosticRecord[];
  drafts: Record<string, string>;
  commandHistory: WorkspaceCommandHistorySnapshot;
  historicalPanes?: Record<string, WorkspaceHistoricalPaneSnapshot>;
  theme: WorkspaceThemeSnapshot;
  terminalDisplay: WorkspaceTerminalDisplaySnapshot;
}

export interface CreateInitialWorkspaceSnapshotOptions {
  themeId?: string | null;
  terminalFontScale?: TerminalPlatformTerminalFontScale | null;
  terminalLineWrap?: boolean | null;
  commandHistoryEntries?: readonly string[] | null;
  commandHistoryLimit?: number | null;
}

export function createInitialWorkspaceSnapshot(
  options: CreateInitialWorkspaceSnapshotOptions = {},
): WorkspaceSnapshot {
  const commandHistoryLimit = normalizeCommandHistoryLimit(
    options.commandHistoryLimit,
  );

  return {
    connection: {
      state: "idle",
      handshake: null,
      lastError: null,
    },
    catalog: {
      sessions: [],
      savedSessions: [],
      discoveredSessions: {},
      backendCapabilities: {},
    },
    selection: {
      activeSessionId: null,
      activePaneId: null,
    },
    attachedSession: null,
    diagnostics: [],
    drafts: {},
    commandHistory: {
      entries: normalizeCommandHistoryEntries(
        options.commandHistoryEntries,
        commandHistoryLimit,
      ),
      limit: commandHistoryLimit,
    },
    historicalPanes: {},
    theme: {
      themeId: options.themeId ?? DEFAULT_WORKSPACE_THEME_ID,
    },
    terminalDisplay: {
      fontScale: options.terminalFontScale ?? DEFAULT_TERMINAL_FONT_SCALE,
      lineWrap: options.terminalLineWrap ?? true,
    },
  };
}

export function normalizeCommandHistoryLimit(
  limit: number | null | undefined,
): number {
  if (typeof limit !== "number" || !Number.isFinite(limit) || limit <= 0) {
    return DEFAULT_COMMAND_HISTORY_LIMIT;
  }

  return Math.max(1, Math.trunc(limit));
}

export function normalizeCommandHistoryEntries(
  entries: readonly string[] | null | undefined,
  limit: number | null | undefined,
): string[] {
  if (!Array.isArray(entries)) {
    return [];
  }

  const normalizedLimit = normalizeCommandHistoryLimit(limit);
  const normalizedEntries: string[] = [];

  for (const value of entries) {
    if (typeof value !== "string") {
      continue;
    }

    const entry = normalizeCommandHistoryEntry(value);
    if (!entry) {
      continue;
    }

    const existingIndex = normalizedEntries.indexOf(entry);
    if (existingIndex >= 0) {
      normalizedEntries.splice(existingIndex, 1);
    }
    normalizedEntries.push(entry);
  }

  return normalizedEntries.slice(-normalizedLimit);
}

export function normalizeCommandHistoryEntry(value: string): string | null {
  const entry = stripShellPromptPrefix(value.trim()).trim();
  return entry.trim().length > 0 ? entry : null;
}

function stripShellPromptPrefix(value: string): string {
  const promptCommandPattern =
    /^(?:\([^)]{1,48}\)\s*)*(?:(?:[\w.-]+@[\w.-]+)\s+)?(?:~(?:\/[\w.,@:+-][\w.,@:+/-]{0,180})?|(?:\/|\.\/|\.\.\/)?[\w.,@:+-][\w.,@:+/-]{0,180}|[A-Za-z]:[\w.,@:+/\\-]{0,180})\s[%$#]\s+(.+)$/u;
  const commandMatch = promptCommandPattern.exec(value);
  if (commandMatch) {
    return commandMatch[1] ?? "";
  }

  const promptOnlyPattern =
    /^(?:\([^)]{1,48}\)\s*)*(?:(?:[\w.-]+@[\w.-]+)\s+)?(?:~(?:\/[\w.,@:+-][\w.,@:+/-]{0,180})?|(?:\/|\.\/|\.\.\/)?[\w.,@:+-][\w.,@:+/-]{0,180}|[A-Za-z]:[\w.,@:+/\\-]{0,180})\s[%$#]\s*$/u;
  return promptOnlyPattern.test(value) ? "" : value;
}
