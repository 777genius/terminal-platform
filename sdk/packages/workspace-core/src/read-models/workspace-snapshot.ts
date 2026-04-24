import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  BackendKind,
  DiscoveredSession,
  Handshake,
  PaneId,
  SavedSessionSummary,
  SessionId,
  SessionSummary,
} from "@terminal-platform/runtime-types";
import type { WorkspaceErrorShape } from "@terminal-platform/workspace-contracts";

export type WorkspaceConnectionState = "idle" | "bootstrapping" | "ready" | "error" | "disposed";
export type WorkspaceDiagnosticSeverity = "info" | "warn" | "error";

export const DEFAULT_WORKSPACE_THEME_ID = "terminal-platform-default" as const;
export const DEFAULT_TERMINAL_FONT_SCALE = "default" as const;

export const terminalPlatformWorkspaceThemeIds = [
  DEFAULT_WORKSPACE_THEME_ID,
  "terminal-platform-light",
] as const;

export const terminalPlatformTerminalFontScales = [
  "compact",
  DEFAULT_TERMINAL_FONT_SCALE,
  "large",
] as const;

export type TerminalPlatformWorkspaceThemeId = (typeof terminalPlatformWorkspaceThemeIds)[number];
export type TerminalPlatformTerminalFontScale = (typeof terminalPlatformTerminalFontScales)[number];

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

export interface WorkspaceSnapshot {
  connection: WorkspaceConnectionSnapshot;
  catalog: WorkspaceCatalogSnapshot;
  selection: WorkspaceSelectionSnapshot;
  attachedSession: AttachedSession | null;
  diagnostics: WorkspaceDiagnosticRecord[];
  drafts: Record<string, string>;
  theme: WorkspaceThemeSnapshot;
  terminalDisplay: WorkspaceTerminalDisplaySnapshot;
}

export interface CreateInitialWorkspaceSnapshotOptions {
  themeId?: string | null;
  terminalFontScale?: TerminalPlatformTerminalFontScale | null;
  terminalLineWrap?: boolean | null;
}

export function createInitialWorkspaceSnapshot(
  options: CreateInitialWorkspaceSnapshotOptions = {},
): WorkspaceSnapshot {
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
    theme: {
      themeId: options.themeId ?? DEFAULT_WORKSPACE_THEME_ID,
    },
    terminalDisplay: {
      fontScale: options.terminalFontScale ?? DEFAULT_TERMINAL_FONT_SCALE,
      lineWrap: options.terminalLineWrap ?? true,
    },
  };
}
