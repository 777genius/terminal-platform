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
  ScreenColor,
  ScreenLine,
  ScreenLineMedia,
  ScreenLineSemanticMark,
  ScreenLineSideEffect,
  ScreenLineSpan,
  ScreenSnapshot,
  ScreenSurfacePalette,
  ScreenTextStyle,
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
import { projectTerminalHistoryText } from "./terminal-history-text-projector.js";

const PANE_HISTORY_INITIAL_EVENT_SEQ = 1n;
const PANE_HISTORY_PAGE_MAX_SEGMENTS = 256;
const PANE_HISTORY_PAGE_MAX_BYTES = 1024 * 1024;

export class SessionCommandService {
  readonly #context: ServiceContext;
  readonly #catalogService: CatalogService;
  readonly #lane = new AsyncLane();
  readonly #paneHistoryHydrations = new Map<string, Promise<void>>();
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
          attachedSession,
          historicalPanes: {
            ...(snapshot.historicalPanes ?? {}),
            ...historicalPanes,
          },
          selection: {
            activeSessionId: restored.session.session_id,
            activePaneId: attachedSession?.focused_screen?.pane_id ?? null,
          },
        }));

        if (attachedSession) {
          this.#syncLiveSessionSubscriptions(attachedSession);
        } else {
          await this.#closeLiveSessionSubscriptions("restored session attach failed");
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
          allowEmptyPage: true,
          includeSnapshotFallback: false,
        });

        if (!nextPage) {
          return false;
        }

        let pageApplied = false;
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

          pageApplied = true;
          return {
            ...current,
            historicalPanes: {
              ...(current.historicalPanes ?? {}),
              [targetPaneId]: mergeHistoricalPanePage(currentHistory, nextPage),
            },
          };
        });

        return pageApplied;
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
    const getPaneHistory = transport.getPaneHistory.bind(transport);

    const existingHistory = this.#context.getSnapshot().historicalPanes?.[paneId];
    if (
      existingHistory?.sessionId === sessionId
      && existingHistory.sourceSessionId === sessionId
      && existingHistory.sourcePaneId === paneId
      && existingHistory.fromEventSeq === PANE_HISTORY_INITIAL_EVENT_SEQ
    ) {
      return;
    }

    const hydrationKey = paneHistoryHydrationKey(sessionId, paneId);
    const inFlightHydration = this.#paneHistoryHydrations.get(hydrationKey);
    if (inFlightHydration) {
      await inFlightHydration;
      return;
    }

    const hydration = this.#hydratePaneHistoryOnce(getPaneHistory, sessionId, paneId);
    this.#paneHistoryHydrations.set(hydrationKey, hydration);
    try {
      await hydration;
    } finally {
      if (this.#paneHistoryHydrations.get(hydrationKey) === hydration) {
        this.#paneHistoryHydrations.delete(hydrationKey);
      }
    }
  }

  async #hydratePaneHistoryOnce(
    getPaneHistory: PaneHistoryLoader,
    sessionId: SessionId,
    paneId: PaneId,
  ): Promise<void> {
    try {
      const history = await getPaneHistory(
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

  async #closeLiveSessionSubscriptions(reason: string): Promise<void> {
    try {
      await Promise.all([
        this.#closeTopologySubscription(),
        this.#closePaneSubscription(),
      ]);
    } catch (error) {
      this.#context.recordDiagnostic({
        code: "live_subscription_close_failed",
        message: `failed to close live session subscriptions after ${reason} - ${errorMessage(error)}`,
        severity: "warn",
        recoverable: true,
        cause: error,
      });
    }
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
type PaneHistoryLoader = NonNullable<Awaited<ReturnType<ServiceContext["ensureTransport"]>>["getPaneHistory"]>;

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
      ...(delta.buffer_kind ? { buffer_kind: delta.buffer_kind } : {}),
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
  if (delta.buffer_kind) {
    next.buffer_kind = delta.buffer_kind;
  } else {
    delete next.buffer_kind;
  }

  if (!delta.patch) {
    return next;
  }

  if (delta.patch.title_changed) {
    next.surface.title = delta.patch.title ?? null;
  }

  if (delta.patch.working_directory_uri_changed) {
    if (delta.patch.working_directory_uri) {
      next.surface.working_directory_uri = delta.patch.working_directory_uri;
    } else {
      delete next.surface.working_directory_uri;
    }
  }

  if (delta.patch.user_variables_changed) {
    if (delta.patch.user_variables && Object.keys(delta.patch.user_variables).length > 0) {
      next.surface.user_variables = structuredClone(delta.patch.user_variables);
    } else {
      delete next.surface.user_variables;
    }
  }

  if (delta.patch.cursor_changed) {
    next.surface.cursor = structuredClone(delta.patch.cursor);
  }

  if (delta.patch.palette_changed) {
    if (delta.patch.palette) {
      next.surface.palette = structuredClone(delta.patch.palette);
    } else {
      delete next.surface.palette;
    }
  }

  if (delta.patch.bell_count_changed) {
    if (delta.patch.bell_count) {
      next.surface.bell_count = delta.patch.bell_count;
    } else {
      delete next.surface.bell_count;
    }
  }

  if (delta.patch.progress_changed) {
    if (delta.patch.progress) {
      next.surface.progress = structuredClone(delta.patch.progress);
    } else {
      delete next.surface.progress;
    }
  }

  for (const update of delta.patch.line_updates ?? []) {
    while (next.surface.lines.length <= update.row) {
      next.surface.lines.push({ text: "", spans: [] });
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
  readonly allowEmptyPage?: boolean;
  readonly includeSnapshotFallback?: boolean;
}

function paneHistoryPageRequest(fromEventSeq: bigint) {
  return {
    fromEventSeq,
    maxSegments: PANE_HISTORY_PAGE_MAX_SEGMENTS,
    maxBytes: PANE_HISTORY_PAGE_MAX_BYTES,
  };
}

function paneHistoryHydrationKey(sessionId: SessionId, paneId: PaneId): string {
  return `${sessionId}\u0000${paneId}`;
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

    const richLines = normalizeHistoryScreenLines(savedScreen.surface.lines);
    const lines = richLines.map((line) => line.text);
    const surfacePalette = normalizeScreenSurfacePalette(savedScreen.surface.palette);
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
      ...(hasRichScreenLineSpans(richLines) ? { richLines } : {}),
      ...(surfacePalette ? { surfacePalette } : {}),
      capturedAtMs: saved.saved_at_ms || BigInt(Math.trunc(loadedAtMs)),
      hasGaps: false,
      hasMoreSegments: false,
      fromEventSeq: PANE_HISTORY_INITIAL_EVENT_SEQ,
      nextEventSeq: null,
      segmentCount: 0,
      loadedPayloadBytes: 0n,
      streamStartsWithLineBreak: true,
      streamEndsWithLineBreak: true,
    };
  }

  return historicalPanes;
}

function buildHydratedHistoricalPane(
  history: PaneHistory,
  loadedAtMs: number,
  options: BuildHydratedHistoricalPaneOptions = {},
): WorkspaceHistoricalPaneSnapshot | null {
  const segmentText = linesFromHistorySegments(
    history.segments,
    terminalHistoryProjectionSize(history.latest_screen_snapshot?.screen_json),
  );
  const segmentRichLines = normalizeHistoryRichLines(segmentText.richLines ?? []);
  const segmentLines = segmentRichLines.map((line) => line.text);
  const includeSnapshotFallback = options.includeSnapshotFallback ?? true;
  const gapLines = history.gaps.map((gap) => {
    const eventRange = gap.event_seq_low && gap.event_seq_high
      ? ` events ${gap.event_seq_low}-${gap.event_seq_high}`
      : "";
    return `--- history gap${eventRange}: ${gap.reason} ---`;
  });
  const hasJournalLines = segmentLines.length > 0 || gapLines.length > 0;
  const useSnapshotFallback =
    includeSnapshotFallback
    && !hasJournalLines
    && !history.has_more_segments
    && Boolean(history.latest_screen_snapshot);
  const snapshotOutput =
    includeSnapshotFallback
    && !history.has_more_segments
    && history.latest_screen_snapshot
    ? linesFromScreenSnapshotJson(history.latest_screen_snapshot.screen_json)
    : { lines: [] };
  const journalRichLines = segmentRichLines.length > 0
    ? normalizeHistoryRichLines([
        ...segmentRichLines,
        ...gapLines.map((line) => createPlainHistoryScreenLine(line)),
      ])
    : undefined;
  const snapshotRichLines = snapshotOutput.richLines && snapshotOutput.richLines.length > 0
    ? normalizeHistoryRichLines([
        ...snapshotOutput.richLines,
        ...gapLines.map((line) => createPlainHistoryScreenLine(line)),
      ])
    : undefined;
  const candidateRichLines =
    journalRichLines &&
    snapshotRichLines &&
    doHistoryRichLinesMatchText(
      snapshotRichLines,
      journalRichLines.map((line) => line.text),
    )
      ? snapshotRichLines
      : journalRichLines ?? snapshotRichLines;
  const lines = candidateRichLines
    ? candidateRichLines.map((line) => line.text)
    : normalizeHistoryLines([
        ...(segmentLines.length > 0 ? segmentLines : snapshotOutput.lines),
        ...gapLines,
      ]);
  const richLines = candidateRichLines && doHistoryRichLinesMatchText(candidateRichLines, lines)
    ? candidateRichLines
    : undefined;
  const alignedRichLines =
    richLines && doHistoryRichLinesMatchText(richLines, lines)
      ? richLines
      : undefined;
  const surfacePalette =
    (useSnapshotFallback || Boolean(alignedRichLines))
    && snapshotOutput.surfacePalette
      ? snapshotOutput.surfacePalette
      : undefined;

  if (lines.length === 0 && !history.has_more_segments && options.allowEmptyPage !== true) {
    return null;
  }

  const latestSegment = history.segments.at(-1);
  const snapshotCapturedAtMs = useSnapshotFallback
    ? history.latest_screen_snapshot?.created_at_ms
    : undefined;
  const capturedAtMs =
    latestSegment?.created_at_ms
    ?? snapshotCapturedAtMs
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
    ...(alignedRichLines && hasRichScreenLineSpans(alignedRichLines)
      ? { richLines: alignedRichLines }
      : {}),
    ...(surfacePalette ? { surfacePalette } : {}),
    capturedAtMs,
    hasGaps: history.gaps.length > 0,
    hasMoreSegments: history.has_more_segments,
    fromEventSeq: history.from_event_seq,
    nextEventSeq: history.next_event_seq,
    segmentCount: history.segments.length,
    loadedPayloadBytes: history.total_payload_bytes,
    streamStartsWithLineBreak: segmentText.startsWithLineBreak ?? true,
    streamEndsWithLineBreak: gapLines.length > 0 || useSnapshotFallback
      ? true
      : (segmentText.endsWithLineBreak ?? true),
  };
}

function mergeHistoricalPanePage(
  existing: WorkspaceHistoricalPaneSnapshot,
  nextPage: WorkspaceHistoricalPaneSnapshot,
): WorkspaceHistoricalPaneSnapshot {
  const { richLines: _existingRichLines, ...existingWithoutRichLines } = existing;
  const lines = mergeHistoricalPaneLines(existing, nextPage);
  const richLines = mergeHistoricalPaneRichLines(existing, nextPage, lines);
  return {
    ...existingWithoutRichLines,
    source: nextPage.source,
    replayStrategy: nextPage.replayStrategy,
    restoreGuaranteeLevel: nextPage.restoreGuaranteeLevel,
    lines,
    ...(richLines ? { richLines } : {}),
    capturedAtMs: nextPage.capturedAtMs > existing.capturedAtMs
      ? nextPage.capturedAtMs
      : existing.capturedAtMs,
    hasGaps: existing.hasGaps || nextPage.hasGaps,
    hasMoreSegments: nextPage.hasMoreSegments,
    nextEventSeq: nextPage.nextEventSeq,
    segmentCount: existing.segmentCount + nextPage.segmentCount,
    loadedPayloadBytes: existing.loadedPayloadBytes + nextPage.loadedPayloadBytes,
    streamStartsWithLineBreak: existing.streamStartsWithLineBreak ?? true,
    streamEndsWithLineBreak: nextPage.streamEndsWithLineBreak ?? true,
  };
}

function mergeHistoricalPaneLines(
  existing: WorkspaceHistoricalPaneSnapshot,
  nextPage: WorkspaceHistoricalPaneSnapshot,
): string[] {
  if (
    existing.streamEndsWithLineBreak !== false
    || existing.lines.length === 0
    || nextPage.lines.length === 0
  ) {
    return normalizeHistoryLines([...existing.lines, ...nextPage.lines]);
  }

  const existingPrefix = existing.lines.slice(0, -1);
  const existingLastLine = existing.lines[existing.lines.length - 1] ?? "";
  const [nextFirstLine = "", ...nextRestLines] = nextPage.lines;
  if (nextPage.streamStartsWithLineBreak === true) {
    const nextLines = nextFirstLine === "" ? nextRestLines : nextPage.lines;
    return normalizeHistoryLines([...existingPrefix, existingLastLine, ...nextLines]);
  }

  return normalizeHistoryLines([
    ...existingPrefix,
    `${existingLastLine}${nextFirstLine}`,
    ...nextRestLines,
  ]);
}

interface HistorySegmentText {
  readonly richLines?: ScreenLine[];
  readonly startsWithLineBreak?: boolean;
  readonly endsWithLineBreak?: boolean;
}

interface HistorySnapshotLines {
  readonly lines: string[];
  readonly richLines?: ScreenLine[];
  readonly surfacePalette?: ScreenSurfacePalette;
}

function linesFromHistorySegments(
  segments: PaneHistory["segments"],
  size: { columns: number; rows: number } | null,
): HistorySegmentText {
  if (segments.length === 0) {
    return {};
  }

  const bytes = segments.flatMap((segment) => segment.payload);
  const projection = projectTerminalHistoryText(
    new TextDecoder().decode(Uint8Array.from(bytes)),
    size ? { columns: size.columns, rows: size.rows } : undefined,
  );
  return {
    richLines: projection.lines,
    startsWithLineBreak: projection.startsWithLineBreak,
    endsWithLineBreak: projection.endsWithLineBreak,
  };
}

function terminalHistoryProjectionSize(
  screenJson: string | null | undefined,
): { columns: number; rows: number } | null {
  if (!screenJson) return null;

  try {
    const parsed = JSON.parse(screenJson) as unknown;
    if (!isRecord(parsed)) return null;
    const columns = parsed.cols;
    const rows = parsed.rows;
    return typeof columns === "number"
      && Number.isInteger(columns)
      && columns > 0
      && typeof rows === "number"
      && Number.isInteger(rows)
      && rows > 0
      ? { columns, rows }
      : null;
  } catch {
    return null;
  }
}

function linesFromScreenSnapshotJson(screenJson: string): HistorySnapshotLines {
  try {
    const parsed = JSON.parse(screenJson) as unknown;
    if (isRecord(parsed)) {
      if (Array.isArray(parsed.lines)) {
        const lines = normalizeHistoryLines(
          parsed.lines.filter((line): line is string => typeof line === "string"),
        );
        return { lines };
      }

      const surface = parsed.surface;
      if (isRecord(surface) && Array.isArray(surface.lines)) {
        const richLines = normalizeHistoryRichLines(
          surface.lines.map(screenLineFromUnknown),
        );
        const surfacePalette = normalizeScreenSurfacePalette(
          screenSurfacePaletteFromUnknown(surface.palette),
        );
        return {
          lines: richLines.map((line) => line.text),
          ...(hasRichScreenLineSpans(richLines) ? { richLines } : {}),
          ...(surfacePalette ? { surfacePalette } : {}),
        };
      }
    }
  } catch {
    return { lines: [] };
  }

  return { lines: [] };
}

function screenSurfacePaletteFromUnknown(value: unknown): ScreenSurfacePalette | undefined {
  if (!isRecord(value)) {
    return undefined;
  }

  return {
    ...(isScreenColor(value.foreground) ? { foreground: value.foreground } : {}),
    ...(isScreenColor(value.background) ? { background: value.background } : {}),
    ...(isScreenColor(value.cursor) ? { cursor: value.cursor } : {}),
  };
}

function screenLineFromUnknown(value: unknown): ScreenLine {
  if (typeof value === "string") {
    return createPlainHistoryScreenLine(value);
  }

  if (!isRecord(value) || typeof value.text !== "string") {
    return createPlainHistoryScreenLine("");
  }

  const spans = Array.isArray(value.spans)
    ? value.spans
        .map(screenLineSpanFromUnknown)
        .filter((span): span is ScreenLineSpan => Boolean(span))
    : [];
  const normalizedSpans = spans.map((span) => span.text).join("") === value.text
    ? spans
    : [];
  const media = Array.isArray(value.media)
    ? value.media
        .map(screenLineMediaFromUnknown)
        .filter((item): item is ScreenLineMedia => Boolean(item))
    : [];
  const sideEffects = Array.isArray(value.side_effects)
    ? value.side_effects
        .map(screenLineSideEffectFromUnknown)
        .filter((item): item is ScreenLineSideEffect => Boolean(item))
    : [];
  const semanticMarks = Array.isArray(value.semantic_marks)
    ? value.semantic_marks
        .map(screenLineSemanticMarkFromUnknown)
        .filter((item): item is ScreenLineSemanticMark => Boolean(item))
    : [];
  return {
    text: value.text,
    spans: normalizedSpans,
    ...(media.length > 0 ? { media } : {}),
    ...(sideEffects.length > 0 ? { side_effects: sideEffects } : {}),
    ...(semanticMarks.length > 0 ? { semantic_marks: semanticMarks } : {}),
    ...(value.wrapped === true ? { wrapped: true } : {}),
  };
}

function screenLineMediaFromUnknown(value: unknown): ScreenLineMedia | null {
  if (!isRecord(value) || !isScreenLineMediaKind(value.kind)) {
    return null;
  }

  return {
    kind: value.kind,
    ...(typeof value.name === "string" && value.name.trim()
      ? { name: value.name.trim() }
      : {}),
    ...(typeof value.byte_size === "number" &&
    Number.isFinite(value.byte_size) &&
    value.byte_size >= 0
      ? { byte_size: Math.trunc(value.byte_size) }
      : {}),
    ...(typeof value.width === "string" && value.width.trim()
      ? { width: value.width.trim() }
      : {}),
    ...(typeof value.height === "string" && value.height.trim()
      ? { height: value.height.trim() }
      : {}),
    ...(typeof value.preserve_aspect_ratio === "boolean"
      ? { preserve_aspect_ratio: value.preserve_aspect_ratio }
      : {}),
    ...(value.inline === true ? { inline: true } : {}),
    ...(typeof value.mime_type === "string" && isSupportedInlineImageMimeType(value.mime_type)
      ? { mime_type: value.mime_type }
      : {}),
    ...(typeof value.data_base64 === "string" && value.data_base64.trim()
      ? { data_base64: value.data_base64.trim() }
      : {}),
    ...(value.truncated === true ? { truncated: true } : {}),
  };
}

function isScreenLineMediaKind(value: unknown): value is ScreenLineMedia["kind"] {
  return value === "iterm2_image" || value === "kitty_graphics" || value === "sixel";
}

function isSupportedInlineImageMimeType(value: string): boolean {
  return value === "image/png" || value === "image/jpeg" || value === "image/gif" || value === "image/webp";
}

function screenLineSideEffectFromUnknown(value: unknown): ScreenLineSideEffect | null {
  if (
    !isRecord(value) ||
    !isScreenLineSideEffectKind(value.kind) ||
    !isScreenLineSideEffectDisposition(value.disposition)
  ) {
    return null;
  }

  return {
    kind: value.kind,
    disposition: value.disposition,
    ...(isScreenLineSideEffectTarget(value.target)
      ? { target: value.target }
      : {}),
    ...(typeof value.message === "string" && value.message.trim()
      ? { message: value.message.trim() }
      : {}),
  };
}

function isScreenLineSideEffectKind(
  value: unknown,
): value is ScreenLineSideEffect["kind"] {
  return (
    value === "clipboard_read" ||
    value === "clipboard_write" ||
    value === "desktop_notification"
  );
}

function isScreenLineSideEffectDisposition(
  value: unknown,
): value is ScreenLineSideEffect["disposition"] {
  return value === "blocked";
}

function isScreenLineSideEffectTarget(
  value: unknown,
): value is NonNullable<ScreenLineSideEffect["target"]> {
  return (
    value === "clipboard" ||
    value === "selection" ||
    value === "desktop_notification" ||
    value === "unknown"
  );
}

function screenLineSemanticMarkFromUnknown(value: unknown): ScreenLineSemanticMark | null {
  if (!isRecord(value) || !isScreenLineSemanticMarkKind(value.kind)) {
    return null;
  }

  return {
    kind: value.kind,
    ...(isScreenLineSemanticMarkCol(value.col) ? { col: value.col } : {}),
    ...(isScreenLineSemanticMarkExitCode(value.exit_code)
      ? { exit_code: value.exit_code }
      : {}),
  };
}

function isScreenLineSemanticMarkKind(
  value: unknown,
): value is ScreenLineSemanticMark["kind"] {
  return (
    value === "command_finished" ||
    value === "input_start" ||
    value === "output_start" ||
    value === "prompt_start"
  );
}

function isScreenLineSemanticMarkExitCode(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 255;
}

function isScreenLineSemanticMarkCol(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function screenLineSpanFromUnknown(value: unknown): ScreenLineSpan | null {
  if (!isRecord(value) || typeof value.text !== "string") {
    return null;
  }

  return {
    text: value.text,
    style: screenTextStyleFromUnknown(value.style),
  };
}

function screenTextStyleFromUnknown(value: unknown): ScreenTextStyle {
  const style = isRecord(value) ? value : {};
  return {
    foreground: isScreenColor(style.foreground) ? style.foreground : null,
    background: isScreenColor(style.background) ? style.background : null,
    underline_color: isScreenColor(style.underline_color) ? style.underline_color : null,
    bold: style.bold === true,
    dim: style.dim === true,
    italic: style.italic === true,
    blink: style.blink === true,
    underline: isScreenUnderlineStyle(style.underline) ? style.underline : null,
    overline: style.overline === true,
    border: isScreenTextBorderStyle(style.border) ? style.border : null,
    ...(isScreenTextBaseline(style.baseline) ? { baseline: style.baseline } : {}),
    inverse: style.inverse === true,
    hidden: style.hidden === true,
    strikethrough: style.strikethrough === true,
    hyperlink: typeof style.hyperlink === "string" ? style.hyperlink : null,
  };
}

function isScreenColor(value: unknown): value is ScreenColor {
  if (!isRecord(value) || typeof value.kind !== "string") {
    return false;
  }

  if (value.kind === "named") {
    return typeof value.name === "string";
  }
  if (value.kind === "indexed") {
    return typeof value.index === "number";
  }
  if (value.kind === "rgb") {
    return (
      typeof value.r === "number" &&
      typeof value.g === "number" &&
      typeof value.b === "number"
    );
  }
  return false;
}

function isScreenUnderlineStyle(value: unknown): value is ScreenTextStyle["underline"] {
  return (
    value === "single" ||
    value === "double" ||
    value === "curly" ||
    value === "dotted" ||
    value === "dashed"
  );
}

function isScreenTextBorderStyle(value: unknown): value is ScreenTextStyle["border"] {
  return value === "framed" || value === "encircled";
}

function isScreenTextBaseline(value: unknown): value is NonNullable<ScreenTextStyle["baseline"]> {
  return value === "superscript" || value === "subscript";
}

function normalizeScreenSurfacePalette(
  value: ScreenSurfacePalette | null | undefined,
): ScreenSurfacePalette | undefined {
  if (!value) {
    return undefined;
  }

  return value.foreground || value.background || value.cursor
    ? value
    : undefined;
}

function normalizeHistoryScreenLines(lines: readonly ScreenLine[]): ScreenLine[] {
  return normalizeHistoryRichLines(lines);
}

function normalizeHistoryRichLines(lines: readonly ScreenLine[]): ScreenLine[] {
  const normalized = lines.map(trimHistoryScreenLineEnd);
  while (
    normalized.length > 0 &&
    normalized[normalized.length - 1]?.text === "" &&
    !hasHistoryScreenLineMetadata(normalized[normalized.length - 1])
  ) {
    normalized.pop();
  }
  return normalized;
}

function trimHistoryScreenLineEnd(line: ScreenLine): ScreenLine {
  const spans = line.spans ?? [];
  const text = line.text.slice(0, richHistoryLineTrimmedLength(line.text, spans));
  const metadata = historyScreenLineMetadata(line);
  if (text === line.text) {
    return spans.length > 0
      ? {
          text: line.text,
          spans,
          ...metadata,
          ...(line.wrapped === true ? { wrapped: true } : {}),
        }
      : {
          ...createPlainHistoryScreenLine(line.text, line.wrapped === true),
          ...metadata,
        };
  }

  return {
    text,
    spans: trimScreenLineSpansToText(spans, text),
    ...metadata,
    ...(line.wrapped === true ? { wrapped: true } : {}),
  };
}

function historyScreenLineMetadata(
  line: ScreenLine,
): Partial<Pick<ScreenLine, "media" | "semantic_marks" | "side_effects">> {
  return {
    ...(line.media && line.media.length > 0 ? { media: line.media } : {}),
    ...(line.side_effects && line.side_effects.length > 0
      ? { side_effects: line.side_effects }
      : {}),
    ...(line.semantic_marks && line.semantic_marks.length > 0
      ? { semantic_marks: line.semantic_marks }
      : {}),
  };
}

function hasHistoryScreenLineMetadata(line: ScreenLine | undefined): boolean {
  return Boolean(
    (line?.media && line.media.length > 0) ||
      (line?.side_effects && line.side_effects.length > 0) ||
      (line?.semantic_marks && line.semantic_marks.length > 0),
  );
}

function trimScreenLineSpansToText(
  spans: readonly ScreenLineSpan[],
  text: string,
): ScreenLineSpan[] {
  const nextSpans: ScreenLineSpan[] = [];
  let remaining = text.length;
  for (const span of spans) {
    if (remaining <= 0) {
      break;
    }
    const nextText = span.text.slice(0, remaining);
    if (nextText.length > 0) {
      nextSpans.push({ ...span, text: nextText });
    }
    remaining -= span.text.length;
  }
  return nextSpans.map((span) => span.text).join("") === text ? nextSpans : [];
}

function richHistoryLineTrimmedLength(
  text: string,
  spans: readonly ScreenLineSpan[],
): number {
  if (spans.length === 0) {
    return text.trimEnd().length;
  }

  let end = text.length;
  for (const span of [...spans].reverse()) {
    const spanStart = Math.max(0, end - span.text.length);
    if (!isPlainScreenTextStyle(span.style)) {
      break;
    }

    const trimmedSpanLength = span.text.trimEnd().length;
    if (trimmedSpanLength === span.text.length) {
      break;
    }

    end = spanStart + trimmedSpanLength;
    if (trimmedSpanLength > 0) {
      break;
    }
  }
  return end;
}

function isPlainScreenTextStyle(style: ScreenTextStyle): boolean {
  return (
    !style.foreground &&
    !style.background &&
    !style.underline_color &&
    !style.bold &&
    !style.dim &&
    !style.italic &&
    !style.blink &&
    !style.underline &&
    !style.overline &&
    !style.border &&
    !style.baseline &&
    !style.inverse &&
    !style.hidden &&
    !style.strikethrough &&
    !style.hyperlink
  );
}

function createPlainHistoryScreenLine(text: string, wrapped = false): ScreenLine {
  return { text, spans: [], ...(wrapped ? { wrapped: true } : {}) };
}

function hasRichScreenLineSpans(lines: readonly ScreenLine[]): boolean {
  return lines.some(
    (line) =>
      line.spans.some((span) => !isPlainScreenTextStyle(span.style)) ||
      line.wrapped === true ||
      hasHistoryScreenLineMetadata(line),
  );
}

function doHistoryRichLinesMatchText(
  richLines: readonly ScreenLine[],
  lines: readonly string[],
): boolean {
  return (
    richLines.length === lines.length &&
    richLines.every((line, index) => line.text === lines[index])
  );
}

function mergeHistoricalPaneRichLines(
  existing: WorkspaceHistoricalPaneSnapshot,
  nextPage: WorkspaceHistoricalPaneSnapshot,
  lines: readonly string[],
): ScreenLine[] | undefined {
  if (!existing.richLines || !nextPage.richLines) {
    return undefined;
  }

  const candidate = normalizeHistoryRichLines(mergeHistoricalPaneRichLineContent(existing, nextPage));
  return doHistoryRichLinesMatchText(candidate, lines) && hasRichScreenLineSpans(candidate)
    ? candidate
    : undefined;
}

function mergeHistoricalPaneRichLineContent(
  existing: WorkspaceHistoricalPaneSnapshot,
  nextPage: WorkspaceHistoricalPaneSnapshot,
): ScreenLine[] {
  if (
    existing.streamEndsWithLineBreak !== false ||
    existing.richLines?.length === 0
  ) {
    const nextLines = nextPage.streamStartsWithLineBreak === true
      ? dropLeadingEmptyHistoryRichLine(nextPage.richLines ?? [])
      : (nextPage.richLines ?? []);
    return [...(existing.richLines ?? []), ...nextLines];
  }

  const existingLines = existing.richLines ?? [];
  const nextLines = nextPage.richLines ?? [];
  const existingPrefix = existingLines.slice(0, -1);
  const existingLastLine = existingLines[existingLines.length - 1] ?? createPlainHistoryScreenLine("");
  const [nextFirstLine = createPlainHistoryScreenLine(""), ...nextRestLines] = nextLines;
  if (nextPage.streamStartsWithLineBreak === true) {
    const linesAfterBreak = nextFirstLine.text === "" ? nextRestLines : nextLines;
    return [...existingPrefix, existingLastLine, ...linesAfterBreak];
  }

  return [
    ...existingPrefix,
    joinHistoryScreenLines(existingLastLine, nextFirstLine),
    ...nextRestLines,
  ];
}

function dropLeadingEmptyHistoryRichLine(lines: readonly ScreenLine[]): ScreenLine[] {
  return lines[0]?.text === "" && !hasHistoryScreenLineMetadata(lines[0])
    ? [...lines.slice(1)]
    : [...lines];
}

function joinHistoryScreenLines(first: ScreenLine, second: ScreenLine): ScreenLine {
  return {
    text: `${first.text}${second.text}`,
    spans: [...first.spans, ...second.spans],
    ...combinedHistoryScreenLineMetadata(first, second),
    ...(first.wrapped === true || second.wrapped === true ? { wrapped: true } : {}),
  };
}

function combinedHistoryScreenLineMetadata(first: ScreenLine, second: ScreenLine) {
  const media = [...(first.media ?? []), ...(second.media ?? [])];
  const sideEffects = [...(first.side_effects ?? []), ...(second.side_effects ?? [])];
  const semanticMarks = [
    ...(first.semantic_marks ?? []),
    ...(second.semantic_marks ?? []),
  ];
  return {
    ...(media.length > 0 ? { media } : {}),
    ...(sideEffects.length > 0 ? { side_effects: sideEffects } : {}),
    ...(semanticMarks.length > 0 ? { semantic_marks: semanticMarks } : {}),
  };
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
