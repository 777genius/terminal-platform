import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  BackendKind,
  CreateSessionRequest,
  DeleteSavedSessionResult,
  DiscoveredSession,
  Handshake,
  MuxCommand,
  MuxCommandResult,
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
import type { WorkspaceSubscription, WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";

export interface WorkspacePreloadSubscriptionBridge {
  meta(): SubscriptionMeta;
  nextEvent(): Promise<SubscriptionEvent | null>;
  close(): Promise<void>;
}

export interface WorkspacePreloadBridge {
  handshake(): Promise<Handshake>;
  listSessions(): Promise<SessionSummary[]>;
  listSavedSessions(): Promise<SavedSessionSummary[]>;
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
  getScreenDelta(sessionId: SessionId, paneId: PaneId, fromSequence: bigint): Promise<ScreenDelta>;
  dispatchMuxCommand(sessionId: SessionId, command: MuxCommand): Promise<MuxCommandResult>;
  openSubscription(sessionId: SessionId, spec: SubscriptionSpec): Promise<WorkspacePreloadSubscriptionBridge>;
  close?(): Promise<void>;
}

export interface CreateWorkspacePreloadTransportOptions<TBridge extends WorkspacePreloadBridge = WorkspacePreloadBridge> {
  bridge: TBridge;
}

export function createWorkspacePreloadTransport<TBridge extends WorkspacePreloadBridge = WorkspacePreloadBridge>(
  options: CreateWorkspacePreloadTransportOptions<TBridge>,
): WorkspaceTransportClient {
  return new WorkspacePreloadTransport(options.bridge);
}

class WorkspacePreloadTransport implements WorkspaceTransportClient {
  readonly #bridge: WorkspacePreloadBridge;

  constructor(bridge: WorkspacePreloadBridge) {
    this.#bridge = bridge;
  }

  handshake(): Promise<Handshake> {
    return this.#bridge.handshake();
  }

  listSessions(): Promise<SessionSummary[]> {
    return this.#bridge.listSessions();
  }

  listSavedSessions(): Promise<SavedSessionSummary[]> {
    return this.#bridge.listSavedSessions();
  }

  discoverSessions(backend: BackendKind): Promise<DiscoveredSession[]> {
    return this.#bridge.discoverSessions(backend);
  }

  getBackendCapabilities(backend: BackendKind): Promise<BackendCapabilitiesInfo> {
    return this.#bridge.getBackendCapabilities(backend);
  }

  createSession(backend: BackendKind, request: CreateSessionRequest): Promise<SessionSummary> {
    return this.#bridge.createSession(backend, request);
  }

  importSession(route: SessionRoute, title?: string | null): Promise<SessionSummary> {
    return this.#bridge.importSession(route, title ?? null);
  }

  getSavedSession(sessionId: SessionId): Promise<SavedSessionRecord> {
    return this.#bridge.getSavedSession(sessionId);
  }

  deleteSavedSession(sessionId: SessionId): Promise<DeleteSavedSessionResult> {
    return this.#bridge.deleteSavedSession(sessionId);
  }

  pruneSavedSessions(keepLatest: number): Promise<PruneSavedSessionsResult> {
    return this.#bridge.pruneSavedSessions(keepLatest);
  }

  restoreSavedSession(sessionId: SessionId): Promise<RestoredSession> {
    return this.#bridge.restoreSavedSession(sessionId);
  }

  attachSession(sessionId: SessionId): Promise<AttachedSession> {
    return this.#bridge.attachSession(sessionId);
  }

  getTopologySnapshot(sessionId: SessionId): Promise<TopologySnapshot> {
    return this.#bridge.getTopologySnapshot(sessionId);
  }

  getScreenSnapshot(sessionId: SessionId, paneId: PaneId): Promise<ScreenSnapshot> {
    return this.#bridge.getScreenSnapshot(sessionId, paneId);
  }

  getScreenDelta(sessionId: SessionId, paneId: PaneId, fromSequence: bigint): Promise<ScreenDelta> {
    return this.#bridge.getScreenDelta(sessionId, paneId, fromSequence);
  }

  dispatchMuxCommand(sessionId: SessionId, command: MuxCommand): Promise<MuxCommandResult> {
    return this.#bridge.dispatchMuxCommand(sessionId, command);
  }

  async openSubscription(sessionId: SessionId, spec: SubscriptionSpec): Promise<WorkspaceSubscription> {
    const subscription = await this.#bridge.openSubscription(sessionId, spec);
    return new WorkspacePreloadSubscription(subscription);
  }

  async close(): Promise<void> {
    await this.#bridge.close?.();
  }
}

class WorkspacePreloadSubscription implements WorkspaceSubscription {
  readonly #bridge: WorkspacePreloadSubscriptionBridge;

  constructor(bridge: WorkspacePreloadSubscriptionBridge) {
    this.#bridge = bridge;
  }

  meta(): SubscriptionMeta {
    return this.#bridge.meta();
  }

  nextEvent(): Promise<SubscriptionEvent | null> {
    return this.#bridge.nextEvent();
  }

  close(): Promise<void> {
    return this.#bridge.close();
  }
}
