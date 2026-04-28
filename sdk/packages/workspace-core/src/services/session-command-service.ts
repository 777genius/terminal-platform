import { AsyncLane } from "@terminal-platform/foundation";
import { toWorkspaceError, WorkspaceError } from "@terminal-platform/workspace-contracts";

import type {
  AttachedSession,
  BackendKind,
  CreateSessionRequest,
  MuxCommand,
  MuxCommandResult,
  PaneId,
  PaneTreeNode,
  PruneSavedSessionsResult,
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
import type { CatalogService } from "./catalog-service.js";
import type { ServiceContext } from "./service-context.js";

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
        const restored = await transport.restoreSavedSession(sessionId);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          catalog: {
            ...snapshot.catalog,
            sessions: mergeSession(snapshot.catalog.sessions, restored.session),
          },
          selection: {
            ...snapshot.selection,
            activeSessionId: restored.session.session_id,
          },
        }));

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
