import { AsyncLane } from "@terminal-platform/foundation";
import { toWorkspaceError, WorkspaceError } from "@terminal-platform/workspace-contracts";

import type {
  AttachedSession,
  BackendKind,
  CreateSessionRequest,
  MuxCommand,
  MuxCommandResult,
  PaneHistory,
  PaneId,
  PaneTreeNode,
  PruneSavedSessionsResult,
  SavedSessionRecord,
  ScreenDelta,
  ScreenSnapshot,
  SessionId,
  SessionRoute,
  SubscriptionEvent,
  SubscriptionSpec,
  TopologySnapshot,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSubscription } from "@terminal-platform/workspace-contracts";

import type { WorkspaceSnapshot } from "../read-models/workspace-snapshot.js";
import type { WorkspaceHistoricalPaneSnapshot } from "../read-models/workspace-snapshot.js";
import type { CatalogService } from "./catalog-service.js";
import type { ServiceContext } from "./service-context.js";

const PANE_HISTORY_INITIAL_EVENT_SEQ = 1n;
const PANE_HISTORY_PAGE_MAX_SEGMENTS = 256;
const PANE_HISTORY_PAGE_MAX_BYTES = 1024 * 1024;

export class SessionCommandService {
  readonly #context: ServiceContext;
  readonly #catalogService: CatalogService;
  readonly #lane = new AsyncLane();
  #disposed = false;
  #topologySubscription: LiveTopologySubscription | null = null;
  #paneSubscription: LivePaneSubscription | null = null;

  constructor(context: ServiceContext, catalogService: CatalogService) {
    this.#context = context;
    this.#catalogService = catalogService;
  }

  createSession(backend: BackendKind, request: CreateSessionRequest): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        const session = await transport.createSession(backend, request);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          catalog: {
            ...snapshot.catalog,
            sessions: mergeSession(snapshot.catalog.sessions, session),
          },
          selection: {
            ...snapshot.selection,
            activeSessionId: session.session_id,
          },
        }));
      } catch (error) {
        throw this.#handleTransportError(error, "failed to create session");
      }
    });
  }

  importSession(route: SessionRoute, title?: string | null): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        const session = await transport.importSession(route, title);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          catalog: {
            ...snapshot.catalog,
            sessions: mergeSession(snapshot.catalog.sessions, session),
          },
          selection: {
            ...snapshot.selection,
            activeSessionId: session.session_id,
          },
        }));
      } catch (error) {
        throw this.#handleTransportError(error, "failed to import session");
      }
    });
  }

  attachSession(sessionId: SessionId): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        const attachedSession = await transport.attachSession(sessionId);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          attachedSession,
          selection: {
            activeSessionId: attachedSession.session.session_id,
            activePaneId: attachedSession.focused_screen?.pane_id ?? null,
          },
        }));

        this.#syncLiveSessionSubscriptions(attachedSession);
        void this.#hydratePaneHistory(
          transport,
          attachedSession.session.session_id,
          attachedSession.focused_screen?.pane_id ?? focusedPaneId(attachedSession.topology),
        );
      } catch (error) {
        throw this.#handleTransportError(error, "failed to attach session");
      }
    });
  }

  restoreSavedSession(sessionId: SessionId): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const restoreBlocker = findSavedSessionRestoreBlocker(this.#context.getSnapshot(), sessionId);
        if (restoreBlocker) {
          throw restoreBlocker;
        }

        const transport = await this.#context.ensureTransport();
        const savedSession = await this.#loadSavedSessionForHistory(transport, sessionId);
        const restored = await transport.restoreSavedSession(sessionId);
        const attachedSession = await this.#attachRestoredSessionForHistory(
          transport,
          restored.session.session_id,
        );
        const historicalPanes = savedSession && attachedSession
          ? await this.#buildRestoredHistoricalPanes(transport, savedSession, attachedSession)
          : {};

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          catalog: {
            ...snapshot.catalog,
            sessions: mergeSession(snapshot.catalog.sessions, restored.session),
          },
          attachedSession: attachedSession ?? snapshot.attachedSession,
          historicalPanes: {
            ...(snapshot.historicalPanes ?? {}),
            ...historicalPanes,
          },
          selection: {
            activeSessionId: restored.session.session_id,
            activePaneId: attachedSession?.focused_screen?.pane_id ?? snapshot.selection.activePaneId,
          },
        }));

        if (attachedSession) {
          this.#syncLiveSessionSubscriptions(attachedSession);
        }
        await this.#catalogService.refreshSavedSessions();
      } catch (error) {
        throw this.#handleTransportError(error, "failed to restore saved session");
      }
    });
  }

  deleteSavedSession(sessionId: SessionId): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        await transport.deleteSavedSession(sessionId);
        await this.#catalogService.refreshSavedSessions();
      } catch (error) {
        throw this.#handleTransportError(error, "failed to delete saved session");
      }
    });
  }

  pruneSavedSessions(keepLatest: number): Promise<PruneSavedSessionsResult> {
    return this.#lane.enqueue(async () => {
      try {
        const keepLatestCount = normalizeSavedSessionPruneLimit(keepLatest);
        const transport = await this.#context.ensureTransport();
        const result = await transport.pruneSavedSessions(keepLatestCount);
        await this.#catalogService.refreshSavedSessions();
        return result;
      } catch (error) {
        throw this.#handleTransportError(error, "failed to prune saved sessions");
      }
    });
  }

  dispatchMuxCommand(sessionId: SessionId, command: MuxCommand): Promise<MuxCommandResult> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        return await transport.dispatchMuxCommand(sessionId, command);
      } catch (error) {
        throw this.#handleTransportError(error, "failed to dispatch mux command");
      }
    });
  }

  loadMorePaneHistory(paneId?: PaneId | null): Promise<boolean> {
    return this.#lane.enqueue(async () => {
      const snapshot = this.#context.getSnapshot();
      const targetPaneId = paneId ?? snapshot.selection.activePaneId;
      if (!targetPaneId) {
        return false;
      }

      const existingHistory = snapshot.historicalPanes?.[targetPaneId];
      if (!existingHistory?.hasMoreSegments || !existingHistory.nextEventSeq) {
        return false;
      }

      try {
        const transport = await this.#context.ensureTransport();
        if (!transport.getPaneHistory) {
          return false;
        }

        const history = await transport.getPaneHistory(
          existingHistory.sourceSessionId,
          existingHistory.sourcePaneId,
          paneHistoryPageRequest(existingHistory.nextEventSeq),
        );
        const nextPage = buildHydratedHistoricalPane(history, this.#context.now(), {
          identity: {
            sessionId: existingHistory.sessionId,
            paneId: existingHistory.paneId,
            sourceSessionId: existingHistory.sourceSessionId,
            sourcePaneId: existingHistory.sourcePaneId,
          },
          includeSnapshotFallback: false,
        });

        if (!nextPage) {
          return false;
        }

        this.#context.updateSnapshot((current) => {
          const currentHistory = current.historicalPanes?.[targetPaneId];
          if (
            !currentHistory
            || currentHistory.sourceSessionId !== existingHistory.sourceSessionId
            || currentHistory.sourcePaneId !== existingHistory.sourcePaneId
            || currentHistory.nextEventSeq !== existingHistory.nextEventSeq
          ) {
            return current;
          }

          return {
            ...current,
            historicalPanes: {
              ...(current.historicalPanes ?? {}),
              [targetPaneId]: mergeHistoricalPanePage(currentHistory, nextPage),
            },
          };
        });

        return true;
      } catch (error) {
        this.#context.recordDiagnostic({
          code: "pane_history_page_load_failed",
          message: `failed to load more pane history for ${targetPaneId}`,
          severity: "warn",
          recoverable: true,
          cause: error,
        });
        return false;
      }
    });
  }

  async openSubscription(
    sessionId: SessionId,
    spec: SubscriptionSpec,
  ): Promise<WorkspaceSubscription> {
    try {
      const transport = await this.#context.ensureTransport();
      return await transport.openSubscription(sessionId, spec);
    } catch (error) {
      throw this.#handleTransportError(error, "failed to open subscription");
    }
  }

  setActiveSession(sessionId: SessionId | null): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      selection: {
        ...snapshot.selection,
        activeSessionId: sessionId,
      },
    }));
  }

  setActivePane(paneId: PaneId | null): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      selection: {
        ...snapshot.selection,
        activePaneId: paneId,
      },
    }));
  }

  async dispose(): Promise<void> {
    if (this.#disposed) {
      return;
    }

    this.#disposed = true;
    await Promise.all([
      this.#closeTopologySubscription(),
      this.#closePaneSubscription(),
    ]);
  }

  #syncLiveSessionSubscriptions(attachedSession: AttachedSession): void {
    const sessionId = attachedSession.session.session_id;
    const paneId = attachedSession.focused_screen?.pane_id ?? focusedPaneId(attachedSession.topology);

    void Promise.all([
      this.#ensureTopologySubscription(sessionId),
      this.#ensurePaneSubscription(sessionId, paneId),
    ]).catch((error: unknown) => {
      if (!this.#disposed) {
        this.#handleTransportError(error, "failed to sync workspace subscriptions");
      }
    });
  }

  async #ensureTopologySubscription(sessionId: SessionId): Promise<void> {
    if (this.#disposed || this.#topologySubscription?.sessionId === sessionId) {
      return;
    }

    await this.#closeTopologySubscription();

    try {
      const transport = await this.#context.ensureTransport();
      const subscription = await transport.openSubscription(sessionId, {
        kind: "session_topology",
      });
      const record: LiveTopologySubscription = {
        kind: "topology",
        sessionId,
        subscription,
      };

      if (this.#disposed || this.#context.getSnapshot().attachedSession?.session.session_id !== sessionId) {
        await subscription.close();
        return;
      }

      this.#topologySubscription = record;
      void this.#consumeLiveSubscription(record);
    } catch (error) {
      if (!this.#disposed) {
        this.#handleTransportError(error, "failed to open session topology subscription");
      }
    }
  }

  async #ensurePaneSubscription(sessionId: SessionId, paneId: PaneId | null): Promise<void> {
    if (this.#disposed) {
      return;
    }

    if (!paneId) {
      await this.#closePaneSubscription();
      return;
    }

    if (
      this.#paneSubscription?.sessionId === sessionId
      && this.#paneSubscription.paneId === paneId
    ) {
      return;
    }

    await this.#closePaneSubscription();

    try {
      const transport = await this.#context.ensureTransport();
      const subscription = await transport.openSubscription(sessionId, {
        kind: "pane_surface",
        pane_id: paneId,
      });
      const record: LivePaneSubscription = {
        kind: "pane",
        sessionId,
        paneId,
        subscription,
      };

      const snapshot = this.#context.getSnapshot();
      if (
        this.#disposed
        || snapshot.attachedSession?.session.session_id !== sessionId
        || snapshot.selection.activePaneId !== paneId
      ) {
        await subscription.close();
        return;
      }

      this.#paneSubscription = record;
      void this.#consumeLiveSubscription(record);
      void this.#hydratePaneHistory(transport, sessionId, paneId);
    } catch (error) {
      if (!this.#disposed) {
        this.#handleTransportError(error, "failed to open pane surface subscription");
      }
    }
  }

  async #consumeLiveSubscription(record: LiveSubscription): Promise<void> {
    while (!this.#disposed && this.#isCurrentLiveSubscription(record)) {
      let event: SubscriptionEvent | null;
      try {
        event = await record.subscription.nextEvent();
      } catch (error) {
        if (!this.#disposed && this.#isCurrentLiveSubscription(record)) {
          this.#handleTransportError(error, "workspace subscription failed");
        }
        break;
      }

      if (!event || !this.#isCurrentLiveSubscription(record)) {
        break;
      }

      try {
        this.#applyLiveSubscriptionEvent(record.sessionId, event);
      } catch (error) {
        if (!this.#disposed && this.#isCurrentLiveSubscription(record)) {
          this.#handleTransportError(error, "failed to apply workspace subscription event");
        }
      }
    }
  }

  #applyLiveSubscriptionEvent(sessionId: SessionId, event: SubscriptionEvent): void {
    switch (event.kind) {
      case "topology_snapshot": {
        const topology = topologyFromSubscriptionEvent(event);
        const paneId = focusedPaneId(topology);

        this.#context.updateSnapshot((snapshot) => {
          const attachedSession = snapshot.attachedSession;
          if (!attachedSession || attachedSession.session.session_id !== sessionId) {
            return snapshot;
          }

          const focusedScreen =
            attachedSession.focused_screen?.pane_id === paneId
              ? attachedSession.focused_screen
              : null;

          return {
            ...snapshot,
            attachedSession: {
              ...attachedSession,
              topology,
              focused_screen: focusedScreen,
            },
            selection: {
              activeSessionId: attachedSession.session.session_id,
              activePaneId: paneId,
            },
          };
        });

        void this.#ensurePaneSubscription(sessionId, paneId);
        break;
      }
      case "screen_delta": {
        const delta = screenDeltaFromSubscriptionEvent(event);

        this.#context.updateSnapshot((snapshot) => {
          const attachedSession = snapshot.attachedSession;
          if (!attachedSession || attachedSession.session.session_id !== sessionId) {
            return snapshot;
          }

          const activePaneId = attachedSession.focused_screen?.pane_id ?? snapshot.selection.activePaneId;
          if (activePaneId !== delta.pane_id) {
            return snapshot;
          }

          const focusedScreen = applyScreenDelta(attachedSession.focused_screen, delta);

          return {
            ...snapshot,
            attachedSession: {
              ...attachedSession,
              focused_screen: focusedScreen,
            },
            selection: {
              activeSessionId: attachedSession.session.session_id,
              activePaneId: focusedScreen.pane_id,
            },
          };
        });
        break;
      }
      case "session_health_snapshot": {
        const health = sessionHealthFromSubscriptionEvent(event);

        this.#context.updateSnapshot((snapshot) => {
          const attachedSession = snapshot.attachedSession;
          if (!attachedSession || attachedSession.session.session_id !== sessionId) {
            return snapshot;
          }

          return {
            ...snapshot,
            attachedSession: {
              ...attachedSession,
              health,
            },
          };
        });
        break;
      }
    }
  }

  #isCurrentLiveSubscription(record: LiveSubscription): boolean {
    return record.kind === "topology"
      ? this.#topologySubscription === record
      : this.#paneSubscription === record;
  }

  async #hydratePaneHistory(
    transport: Awaited<ReturnType<ServiceContext["ensureTransport"]>>,
    sessionId: SessionId,
    paneId: PaneId | null,
  ): Promise<void> {
    if (!paneId || !transport.getPaneHistory) {
      return;
    }

    try {
      const history = await transport.getPaneHistory(
        sessionId,
        paneId,
        paneHistoryPageRequest(PANE_HISTORY_INITIAL_EVENT_SEQ),
      );
      const historicalPane = buildHydratedHistoricalPane(history, this.#context.now());
      if (!historicalPane) {
        return;
      }

      this.#context.updateSnapshot((snapshot) => {
        const attachedSession = snapshot.attachedSession;
        if (
          !attachedSession
          || attachedSession.session.session_id !== sessionId
          || snapshot.selection.activePaneId !== paneId
        ) {
          return snapshot;
        }

        const existingHistory = snapshot.historicalPanes?.[paneId];
        if (
          existingHistory?.sessionId === sessionId
          && existingHistory.sourceSessionId !== sessionId
        ) {
          return snapshot;
        }

        return {
          ...snapshot,
          historicalPanes: {
            ...(snapshot.historicalPanes ?? {}),
            [paneId]: historicalPane,
          },
        };
      });
    } catch (error) {
      this.#context.recordDiagnostic({
        code: "pane_history_hydration_failed",
        message: `failed to hydrate pane history for ${paneId} - ${errorMessage(error)}`,
        severity: "warn",
        recoverable: true,
        cause: error,
      });
    }
  }

  async #buildRestoredHistoricalPanes(
    transport: Awaited<ReturnType<ServiceContext["ensureTransport"]>>,
    saved: SavedSessionRecord,
    attachedSession: NonNullable<WorkspaceSnapshot["attachedSession"]>,
  ): Promise<Record<string, WorkspaceHistoricalPaneSnapshot>> {
    const historicalPanes = buildRestoredHistoricalPanes(saved, attachedSession, this.#context.now());
    const getPaneHistory = transport.getPaneHistory?.bind(transport);
    if (!getPaneHistory) {
      return historicalPanes;
    }

    const paneMap = mapSavedPaneIdsToLivePaneIds(saved.topology, attachedSession.topology);
    await Promise.all([...paneMap.entries()].map(async ([sourcePaneId, livePaneId]) => {
      try {
        const history = await getPaneHistory(
          saved.session_id,
          sourcePaneId,
          paneHistoryPageRequest(PANE_HISTORY_INITIAL_EVENT_SEQ),
        );
        const hydratedPane = buildHydratedHistoricalPane(history, this.#context.now(), {
          identity: {
            sessionId: attachedSession.session.session_id,
            paneId: livePaneId,
            sourceSessionId: saved.session_id,
            sourcePaneId,
          },
          includeSnapshotFallback: true,
        });
        if (hydratedPane) {
          historicalPanes[livePaneId] = hydratedPane;
        }
      } catch (error) {
        this.#context.recordDiagnostic({
          code: "saved_pane_history_hydration_failed",
          message: `failed to hydrate saved pane history for ${sourcePaneId} - ${errorMessage(error)}`,
          severity: "warn",
          recoverable: true,
          cause: error,
        });
      }
    }));

    return historicalPanes;
  }

  async #closeTopologySubscription(): Promise<void> {
    const record = this.#topologySubscription;
    this.#topologySubscription = null;
    await record?.subscription.close();
  }

  async #closePaneSubscription(): Promise<void> {
    const record = this.#paneSubscription;
    this.#paneSubscription = null;
    await record?.subscription.close();
  }

  #handleTransportError(error: unknown, message: string) {
    const workspaceError = toWorkspaceError(error, {
      code: "transport_failed",
      message,
      recoverable: true,
    });

    this.#context.recordDiagnostic({
      code: workspaceError.code,
      message: workspaceError.message,
      severity: "error",
      recoverable: workspaceError.recoverable,
      cause: workspaceError.cause,
    });

    return workspaceError;
  }

  async #loadSavedSessionForHistory(
    transport: Awaited<ReturnType<ServiceContext["ensureTransport"]>>,
    sessionId: SessionId,
  ): Promise<SavedSessionRecord | null> {
    try {
      return await transport.getSavedSession(sessionId);
    } catch (error) {
      this.#context.recordDiagnostic({
        code: "saved_history_prefetch_failed",
        message: `failed to prefetch saved session history for ${sessionId}`,
        severity: "warn",
        recoverable: true,
        cause: error,
      });
      return null;
    }
  }

  async #attachRestoredSessionForHistory(
    transport: Awaited<ReturnType<ServiceContext["ensureTransport"]>>,
    sessionId: SessionId,
  ): Promise<NonNullable<WorkspaceSnapshot["attachedSession"]> | null> {
    try {
      return await transport.attachSession(sessionId);
    } catch (error) {
      this.#context.recordDiagnostic({
        code: "restored_session_attach_failed",
        message: `failed to attach restored session ${sessionId}`,
        severity: "warn",
        recoverable: true,
        cause: error,
      });
      return null;
    }
  }
}

interface LiveTopologySubscription {
  readonly kind: "topology";
  readonly sessionId: SessionId;
  readonly subscription: WorkspaceSubscription;
}

interface LivePaneSubscription {
  readonly kind: "pane";
  readonly sessionId: SessionId;
  readonly paneId: PaneId;
  readonly subscription: WorkspaceSubscription;
}

type LiveSubscription = LiveTopologySubscription | LivePaneSubscription;

function topologyFromSubscriptionEvent(
  event: Extract<SubscriptionEvent, { kind: "topology_snapshot" }>,
): TopologySnapshot {
  const { kind: _kind, ...topology } = event;
  return topology;
}

function screenDeltaFromSubscriptionEvent(
  event: Extract<SubscriptionEvent, { kind: "screen_delta" }>,
): ScreenDelta {
  const { kind: _kind, ...delta } = event;
  return delta;
}

function sessionHealthFromSubscriptionEvent(
  event: Extract<SubscriptionEvent, { kind: "session_health_snapshot" }>,
): AttachedSession["health"] {
  const { kind: _kind, ...health } = event;
  return health;
}

function applyScreenDelta(snapshot: ScreenSnapshot | null, delta: ScreenDelta): ScreenSnapshot {
  if (delta.full_replace) {
    return {
      pane_id: delta.pane_id,
      sequence: delta.to_sequence,
      rows: delta.rows,
      cols: delta.cols,
      source: delta.source,
      surface: structuredClone(delta.full_replace),
    };
  }

  if (!snapshot) {
    throw new WorkspaceError({
      code: "protocol_error",
      message: `cannot apply patch delta for pane ${delta.pane_id} without a base screen snapshot`,
      recoverable: true,
    });
  }

  if (snapshot.pane_id !== delta.pane_id) {
    throw new WorkspaceError({
      code: "protocol_error",
      message: `screen delta pane mismatch. Expected ${snapshot.pane_id}, got ${delta.pane_id}`,
      recoverable: true,
    });
  }

  const next = structuredClone(snapshot);
  next.sequence = delta.to_sequence;
  next.rows = delta.rows;
  next.cols = delta.cols;
  next.source = delta.source;

  if (!delta.patch) {
    return next;
  }

  if (delta.patch.title_changed) {
    next.surface.title = delta.patch.title ?? null;
  }

  if (delta.patch.cursor_changed) {
    next.surface.cursor = structuredClone(delta.patch.cursor);
  }

  for (const update of delta.patch.line_updates ?? []) {
    while (next.surface.lines.length <= update.row) {
      next.surface.lines.push({ text: "" });
    }
    next.surface.lines[update.row] = structuredClone(update.line);
  }

  return next;
}

function focusedPaneId(topology: TopologySnapshot): PaneId | null {
  const focusedTab =
    topology.tabs.find((tab) => tab.tab_id === topology.focused_tab)
    ?? topology.tabs[0]
    ?? null;

  if (!focusedTab) {
    return null;
  }

  return focusedTab.focused_pane ?? firstPaneId(focusedTab.root);
}

function firstPaneId(node: PaneTreeNode): PaneId {
  if (node.kind === "leaf") {
    return node.pane_id;
  }

  return firstPaneId(node.first);
}

function normalizeSavedSessionPruneLimit(keepLatest: number): number {
  if (!Number.isFinite(keepLatest) || keepLatest < 0) {
    throw new WorkspaceError({
      code: "protocol_error",
      message: `saved session prune limit must be a non-negative finite number: ${keepLatest}`,
      recoverable: false,
    });
  }

  return Math.trunc(keepLatest);
}

function findSavedSessionRestoreBlocker(
  snapshot: WorkspaceSnapshot,
  sessionId: SessionId,
): WorkspaceError | null {
  const savedSession = snapshot.catalog.savedSessions.find((candidate) => candidate.session_id === sessionId);
  if (!savedSession || savedSession.compatibility.can_restore) {
    return null;
  }

  return new WorkspaceError({
    code: "unsupported_capability",
    message: `saved session ${sessionId} is not restore-compatible: ${savedSession.compatibility.status}`,
    recoverable: false,
  });
}

function mergeSession<TSession extends { session_id: string }>(
  sessions: readonly TSession[],
  nextSession: TSession,
): TSession[] {
  const remaining = sessions.filter((session) => session.session_id !== nextSession.session_id);
  return [...remaining, nextSession];
}

interface HydratedHistoricalPaneIdentity {
  readonly sessionId: SessionId;
  readonly paneId: PaneId;
  readonly sourceSessionId: SessionId;
  readonly sourcePaneId: PaneId;
}

interface BuildHydratedHistoricalPaneOptions {
  readonly identity?: HydratedHistoricalPaneIdentity;
  readonly includeSnapshotFallback?: boolean;
}

function paneHistoryPageRequest(fromEventSeq: bigint) {
  return {
    fromEventSeq,
    maxSegments: PANE_HISTORY_PAGE_MAX_SEGMENTS,
    maxBytes: PANE_HISTORY_PAGE_MAX_BYTES,
  };
}

function buildRestoredHistoricalPanes(
  saved: SavedSessionRecord,
  attachedSession: NonNullable<WorkspaceSnapshot["attachedSession"]>,
  loadedAtMs: number,
): Record<string, WorkspaceHistoricalPaneSnapshot> {
  const paneMap = mapSavedPaneIdsToLivePaneIds(saved.topology, attachedSession.topology);
  const savedScreensByPane = new Map(saved.screens.map((screen) => [screen.pane_id, screen]));
  const historicalPanes: Record<string, WorkspaceHistoricalPaneSnapshot> = {};

  for (const [sourcePaneId, livePaneId] of paneMap.entries()) {
    const savedScreen = savedScreensByPane.get(sourcePaneId);
    if (!savedScreen) {
      continue;
    }

    const lines = savedScreen.surface.lines
      .map((line) => line.text)
      .filter((line, index, source) => line.length > 0 || index < source.length - 1);
    if (lines.length === 0) {
      continue;
    }

    historicalPanes[livePaneId] = {
      sessionId: attachedSession.session.session_id,
      paneId: livePaneId,
      sourceSessionId: saved.session_id,
      sourcePaneId,
      source: "saved_session_restore",
      replayStrategy: "rendered_snapshot",
      restoreGuaranteeLevel: "visual_snapshot_only",
      lines,
      capturedAtMs: saved.saved_at_ms || BigInt(Math.trunc(loadedAtMs)),
      hasGaps: false,
      hasMoreSegments: false,
      fromEventSeq: PANE_HISTORY_INITIAL_EVENT_SEQ,
      nextEventSeq: null,
      segmentCount: 0,
      loadedPayloadBytes: 0n,
    };
  }

  return historicalPanes;
}

function buildHydratedHistoricalPane(
  history: PaneHistory,
  loadedAtMs: number,
  options: BuildHydratedHistoricalPaneOptions = {},
): WorkspaceHistoricalPaneSnapshot | null {
  const segmentLines = linesFromHistorySegments(history.segments);
  const includeSnapshotFallback = options.includeSnapshotFallback ?? true;
  const snapshotLines = includeSnapshotFallback && history.latest_screen_snapshot
    ? linesFromScreenSnapshotJson(history.latest_screen_snapshot.screen_json)
    : [];
  const gapLines = history.gaps.map((gap) => {
    const eventRange = gap.event_seq_low && gap.event_seq_high
      ? ` events ${gap.event_seq_low}-${gap.event_seq_high}`
      : "";
    return `--- history gap${eventRange}: ${gap.reason} ---`;
  });
  const lines = normalizeHistoryLines([
    ...(segmentLines.length > 0 ? segmentLines : snapshotLines),
    ...gapLines,
  ]);

  if (lines.length === 0) {
    return null;
  }

  const latestSegment = history.segments.at(-1);
  const capturedAtMs =
    latestSegment?.created_at_ms
    ?? history.latest_screen_snapshot?.created_at_ms
    ?? BigInt(Math.trunc(loadedAtMs));
  const identity = options.identity ?? {
    sessionId: history.session_id,
    paneId: history.pane_id,
    sourceSessionId: history.session_id,
    sourcePaneId: history.pane_id,
  };

  return {
    sessionId: identity.sessionId,
    paneId: identity.paneId,
    sourceSessionId: identity.sourceSessionId,
    sourcePaneId: identity.sourcePaneId,
    source: "v2_pane_history",
    replayStrategy: history.replay_strategy,
    restoreGuaranteeLevel: history.restore_plan.restore_guarantee_level,
    lines,
    capturedAtMs,
    hasGaps: history.gaps.length > 0,
    hasMoreSegments: history.has_more_segments,
    fromEventSeq: history.from_event_seq,
    nextEventSeq: history.next_event_seq,
    segmentCount: history.segments.length,
    loadedPayloadBytes: history.total_payload_bytes,
  };
}

function mergeHistoricalPanePage(
  existing: WorkspaceHistoricalPaneSnapshot,
  nextPage: WorkspaceHistoricalPaneSnapshot,
): WorkspaceHistoricalPaneSnapshot {
  return {
    ...existing,
    source: nextPage.source,
    replayStrategy: nextPage.replayStrategy,
    restoreGuaranteeLevel: nextPage.restoreGuaranteeLevel,
    lines: normalizeHistoryLines([...existing.lines, ...nextPage.lines]),
    capturedAtMs: nextPage.capturedAtMs > existing.capturedAtMs
      ? nextPage.capturedAtMs
      : existing.capturedAtMs,
    hasGaps: existing.hasGaps || nextPage.hasGaps,
    hasMoreSegments: nextPage.hasMoreSegments,
    nextEventSeq: nextPage.nextEventSeq,
    segmentCount: existing.segmentCount + nextPage.segmentCount,
    loadedPayloadBytes: existing.loadedPayloadBytes + nextPage.loadedPayloadBytes,
  };
}

function linesFromHistorySegments(segments: PaneHistory["segments"]): string[] {
  if (segments.length === 0) {
    return [];
  }

  const bytes = segments.flatMap((segment) => segment.payload);
  const text = sanitizeTerminalHistoryText(new TextDecoder().decode(Uint8Array.from(bytes)));
  return normalizeHistoryLines(text.split("\n"));
}

function linesFromScreenSnapshotJson(screenJson: string): string[] {
  try {
    const parsed = JSON.parse(screenJson) as unknown;
    if (isRecord(parsed)) {
      if (Array.isArray(parsed.lines)) {
        return normalizeHistoryLines(parsed.lines.filter((line): line is string => typeof line === "string"));
      }

      const surface = parsed.surface;
      if (isRecord(surface) && Array.isArray(surface.lines)) {
        return normalizeHistoryLines(surface.lines.map((line) => {
          if (isRecord(line) && typeof line.text === "string") {
            return line.text;
          }
          return "";
        }));
      }
    }
  } catch {
    return [];
  }

  return [];
}

function sanitizeTerminalHistoryText(text: string): string {
  return text
    .replace(/\x1B\][^\x07]*(?:\x07|\x1B\\)/g, "")
    .replace(/\x1B\[[0-?]*[ -/]*[@-~]/g, "")
    .replace(/\x1B[@-Z\\-_]/g, "")
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n")
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, "");
}

function normalizeHistoryLines(lines: readonly string[]): string[] {
  const normalized = lines.map((line) => line.trimEnd());
  while (normalized.length > 0 && normalized[normalized.length - 1] === "") {
    normalized.pop();
  }
  return normalized;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function mapSavedPaneIdsToLivePaneIds(
  savedTopology: SavedSessionRecord["topology"],
  liveTopology: NonNullable<WorkspaceSnapshot["attachedSession"]>["topology"],
): Map<PaneId, PaneId> {
  const pairs = new Map<PaneId, PaneId>();
  const tabCount = Math.min(savedTopology.tabs.length, liveTopology.tabs.length);

  for (let tabIndex = 0; tabIndex < tabCount; tabIndex += 1) {
    const savedTab = savedTopology.tabs[tabIndex];
    const liveTab = liveTopology.tabs[tabIndex];
    if (!savedTab || !liveTab) {
      continue;
    }

    const savedPanes = collectPaneTreeIds(savedTab.root);
    const livePanes = collectPaneTreeIds(liveTab.root);
    const paneCount = Math.min(savedPanes.length, livePanes.length);
    for (let paneIndex = 0; paneIndex < paneCount; paneIndex += 1) {
      pairs.set(savedPanes[paneIndex]!, livePanes[paneIndex]!);
    }
  }

  return pairs;
}

function collectPaneTreeIds(node: PaneTreeNode): PaneId[] {
  if (node.kind === "leaf") {
    return [node.pane_id];
  }

  return [...collectPaneTreeIds(node.first), ...collectPaneTreeIds(node.second)];
}
