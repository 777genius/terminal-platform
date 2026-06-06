import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  BackendKind,
  CommandHistoryEntry,
  CreateSessionRequest,
  DeleteSavedSessionResult,
  DiscoveredSession,
  Handshake,
  MuxCommand,
  MuxCommandResult,
  PaneHistory,
  PaneId,
  PruneSavedSessionsResult,
  RestoredSession,
  SavedSessionRecord,
  SavedSessionSummary,
  ScreenDelta,
  ScreenSnapshot,
  SessionId,
  SessionRoute,
  SessionSummary,
  SubscriptionEvent,
  SubscriptionMeta,
  SubscriptionSpec,
  TopologySnapshot,
} from "@terminal-platform/runtime-types";

export interface WorkspaceSubscription {
  meta(): SubscriptionMeta;
  nextEvent(): Promise<SubscriptionEvent | null>;
  close(): Promise<void>;
}

export interface WorkspaceTransportClient {
  handshake(): Promise<Handshake>;
  listSessions(): Promise<SessionSummary[]>;
  listSavedSessions(): Promise<SavedSessionSummary[]>;
  listCommandHistory?(
    sessionId?: SessionId | null,
    limit?: number | null,
  ): Promise<CommandHistoryEntry[]>;
  getPaneHistory?(
    sessionId: SessionId,
    paneId: PaneId,
    options?: WorkspacePaneHistoryRequestOptions,
  ): Promise<PaneHistory>;
  discoverSessions(backend: BackendKind): Promise<DiscoveredSession[]>;
  getBackendCapabilities(backend: BackendKind): Promise<BackendCapabilitiesInfo>;
  createSession(backend: BackendKind, request: CreateSessionRequest): Promise<SessionSummary>;
  importSession(route: SessionRoute, title?: string | null): Promise<SessionSummary>;
  getSavedSession(sessionId: SessionId): Promise<SavedSessionRecord>;
  deleteSavedSession(sessionId: SessionId): Promise<DeleteSavedSessionResult>;
  pruneSavedSessions(keepLatest: number): Promise<PruneSavedSessionsResult>;
  restoreSavedSession(sessionId: SessionId): Promise<RestoredSession>;
  attachSession(sessionId: SessionId): Promise<AttachedSession>;
  getTopologySnapshot(sessionId: SessionId): Promise<TopologySnapshot>;
  getScreenSnapshot(sessionId: SessionId, paneId: PaneId): Promise<ScreenSnapshot>;
  getScreenDelta(
    sessionId: SessionId,
    paneId: PaneId,
    fromSequence: bigint,
  ): Promise<ScreenDelta>;
  dispatchMuxCommand(sessionId: SessionId, command: MuxCommand): Promise<MuxCommandResult>;
  openSubscription(sessionId: SessionId, spec: SubscriptionSpec): Promise<WorkspaceSubscription>;
  close(): Promise<void>;
}

export interface WorkspacePaneHistoryRequestOptions {
  fromEventSeq?: number | bigint | null;
  maxSegments?: number | null;
  maxBytes?: number | null;
}

export interface WorkspaceTransportFactory {
  create(): Promise<WorkspaceTransportClient> | WorkspaceTransportClient;
}
