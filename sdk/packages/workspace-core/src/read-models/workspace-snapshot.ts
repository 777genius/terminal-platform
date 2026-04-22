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

export interface WorkspaceSnapshot {
  connection: WorkspaceConnectionSnapshot;
  catalog: WorkspaceCatalogSnapshot;
  selection: WorkspaceSelectionSnapshot;
  attachedSession: AttachedSession | null;
  diagnostics: WorkspaceDiagnosticRecord[];
  drafts: Record<string, string>;
  theme: WorkspaceThemeSnapshot;
}

export function createInitialWorkspaceSnapshot(): WorkspaceSnapshot {
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
      themeId: "terminal-platform-default",
    },
  };
}
