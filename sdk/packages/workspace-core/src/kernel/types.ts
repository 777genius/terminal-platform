import type { TelemetrySink } from "@terminal-platform/foundation";
import type {
  BackendCapabilitiesInfo,
  BackendKind,
  CreateSessionRequest,
  MuxCommand,
  MuxCommandResult,
  PaneId,
  PruneSavedSessionsResult,
  SessionId,
  SessionRoute,
  SubscriptionSpec,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSubscription, WorkspaceTransportClient, WorkspaceTransportFactory } from "@terminal-platform/workspace-contracts";

import type {
  WorkspaceConnectionSnapshot,
  WorkspaceDiagnosticRecord,
  WorkspaceSnapshot,
  WorkspaceCommandHistorySnapshot,
  WorkspaceTerminalDisplaySnapshot,
} from "../read-models/workspace-snapshot.js";

export interface WorkspaceSelectors {
  connection(): WorkspaceConnectionSnapshot;
  sessions(): WorkspaceSnapshot["catalog"]["sessions"];
  savedSessions(): WorkspaceSnapshot["catalog"]["savedSessions"];
  activeSession(): WorkspaceSnapshot["catalog"]["sessions"][number] | null;
  activePaneId(): PaneId | null;
  attachedSession(): WorkspaceSnapshot["attachedSession"];
  diagnostics(): WorkspaceDiagnosticRecord[];
  themeId(): string;
  terminalDisplay(): WorkspaceTerminalDisplaySnapshot;
  commandHistory(): WorkspaceCommandHistorySnapshot;
}

export interface WorkspaceCommands {
  bootstrap(): Promise<void>;
  refreshSessions(): Promise<void>;
  refreshSavedSessions(): Promise<void>;
  discoverSessions(backend: BackendKind): Promise<void>;
  getBackendCapabilities(backend: BackendKind): Promise<BackendCapabilitiesInfo>;
  createSession(backend: BackendKind, request: CreateSessionRequest): Promise<void>;
  importSession(route: SessionRoute, title?: string | null): Promise<void>;
  attachSession(sessionId: SessionId): Promise<void>;
  restoreSavedSession(sessionId: SessionId): Promise<void>;
  deleteSavedSession(sessionId: SessionId): Promise<void>;
  pruneSavedSessions(keepLatest: number): Promise<PruneSavedSessionsResult>;
  dispatchMuxCommand(sessionId: SessionId, command: MuxCommand): Promise<MuxCommandResult>;
  openSubscription(
    sessionId: SessionId,
    spec: SubscriptionSpec,
  ): Promise<WorkspaceSubscription>;
  setActiveSession(sessionId: SessionId | null): void;
  setActivePane(paneId: PaneId | null): void;
  updateDraft(paneId: PaneId, value: string): void;
  clearDraft(paneId: PaneId): void;
  recordCommandHistory(value: string): void;
  clearCommandHistory(): void;
  setTheme(themeId: string): void;
  setTerminalFontScale(fontScale: string): void;
  setTerminalLineWrap(lineWrap: boolean): void;
  clearDiagnostics(): void;
}

export interface WorkspaceDiagnostics {
  list(): WorkspaceDiagnosticRecord[];
  clear(): void;
}

export interface WorkspaceKernel {
  getSnapshot(): WorkspaceSnapshot;
  subscribe(listener: () => void): () => void;
  bootstrap(): Promise<void>;
  dispose(): Promise<void>;
  commands: WorkspaceCommands;
  selectors: WorkspaceSelectors;
  diagnostics: WorkspaceDiagnostics;
}

export interface CreateWorkspaceKernelOptions {
  transport: WorkspaceTransportClient | WorkspaceTransportFactory;
  telemetry?: TelemetrySink;
  now?: () => number;
  availableThemeIds?: readonly string[];
  initialThemeId?: string | null;
  initialTerminalFontScale?: string | null;
  initialTerminalLineWrap?: boolean | null;
  commandHistoryLimit?: number | null;
}
