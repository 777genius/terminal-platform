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
  ProjectionSource,
  PruneSavedSessionsResult,
  RestoredSession,
  SavedSessionRecord,
  SavedSessionSummary,
  ScreenDelta,
  ScreenSnapshot,
  ScreenSurface,
  SessionId,
  SessionRoute,
  SessionSummary,
  SubscriptionEvent,
  SubscriptionMeta,
  SubscriptionSpec,
  TopologySnapshot,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSubscription, WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";
import { WorkspaceError } from "@terminal-platform/workspace-contracts";

export interface MemoryWorkspaceFixture {
  handshake: Handshake;
  sessions: SessionSummary[];
  savedSessions: SavedSessionSummary[];
  savedSessionRecords: Record<string, SavedSessionRecord>;
  discoveredSessions: Partial<Record<BackendKind, DiscoveredSession[]>>;
  backendCapabilities: Partial<Record<BackendKind, BackendCapabilitiesInfo>>;
  attachedSessions: Record<string, AttachedSession>;
  topologyBySessionId: Record<string, TopologySnapshot>;
  screensBySessionId: Record<string, Record<string, ScreenSnapshot>>;
}

export interface CreateMemoryWorkspaceTransportOptions {
  fixture?: MemoryWorkspaceFixture;
}

export function createMemoryWorkspaceTransport(
  options: CreateMemoryWorkspaceTransportOptions = {},
): WorkspaceTransportClient {
  const state = structuredClone(options.fixture ?? createDefaultMemoryWorkspaceFixture());
  let closed = false;
  let syntheticCounter = state.sessions.length;

  return {
    async handshake() {
      assertOpen();
      return clone(state.handshake);
    },
    async listSessions() {
      assertOpen();
      return clone(state.sessions);
    },
    async listSavedSessions() {
      assertOpen();
      return clone(state.savedSessions);
    },
    async discoverSessions(backend) {
      assertOpen();
      return clone(state.discoveredSessions[backend] ?? []);
    },
    async getBackendCapabilities(backend) {
      assertOpen();
      return clone(
        state.backendCapabilities[backend] ?? {
          backend,
          capabilities: createDefaultBackendCapabilities(),
        },
      );
    },
    async createSession(backend, request) {
      assertOpen();
      syntheticCounter += 1;
      const sessionId = `memory-session-${syntheticCounter}`;
      const session = createSessionSummary(sessionId, backend, request.title ?? null);
      state.sessions.push(session);
      seedSessionArtifacts(state, session, request.title ?? null);
      return clone(session);
    },
    async importSession(route, title) {
      assertOpen();
      syntheticCounter += 1;
      const sessionId = `memory-imported-${syntheticCounter}`;
      const session: SessionSummary = {
        session_id: sessionId,
        route,
        title: title ?? null,
      };
      state.sessions.push(session);
      seedSessionArtifacts(state, session, title ?? null);
      return clone(session);
    },
    async getSavedSession(sessionId) {
      assertOpen();
      return clone(requireRecord(state.savedSessionRecords, sessionId, "saved session"));
    },
    async deleteSavedSession(sessionId) {
      assertOpen();
      delete state.savedSessionRecords[sessionId];
      state.savedSessions = state.savedSessions.filter((session) => session.session_id !== sessionId);
      const result: DeleteSavedSessionResult = { session_id: sessionId };
      return clone(result);
    },
    async pruneSavedSessions(keepLatest) {
      assertOpen();
      const sorted = [...state.savedSessions].sort((left, right) => {
        if (left.saved_at_ms === right.saved_at_ms) {
          return 0;
        }

        return left.saved_at_ms > right.saved_at_ms ? -1 : 1;
      });
      const kept = sorted.slice(0, keepLatest);
      const keptIds = new Set(kept.map((session) => session.session_id));
      const deletedCount = sorted.length - kept.length;
      state.savedSessions = kept;
      for (const sessionId of Object.keys(state.savedSessionRecords)) {
        if (!keptIds.has(sessionId)) {
          delete state.savedSessionRecords[sessionId];
        }
      }
      const result: PruneSavedSessionsResult = { deleted_count: deletedCount, kept_count: kept.length };
      return clone(result);
    },
    async restoreSavedSession(sessionId) {
      assertOpen();
      const record = requireRecord(state.savedSessionRecords, sessionId, "saved session");
      state.sessions.push(recordToSessionSummary(record));
      seedSavedSessionArtifacts(state, record);
      const restored: RestoredSession = {
        saved_session_id: record.session_id,
        manifest: record.manifest,
        compatibility: record.compatibility,
        session: recordToSessionSummary(record),
        restore_semantics: record.restore_semantics,
      };
      return clone(restored);
    },
    async attachSession(sessionId) {
      assertOpen();
      return clone(requireRecord(state.attachedSessions, sessionId, "attached session"));
    },
    async getTopologySnapshot(sessionId) {
      assertOpen();
      return clone(requireRecord(state.topologyBySessionId, sessionId, "topology snapshot"));
    },
    async getScreenSnapshot(sessionId, paneId) {
      assertOpen();
      const screens = requireRecord(state.screensBySessionId, sessionId, "screen collection");
      return clone(requireRecord(screens, paneId, "screen snapshot"));
    },
    async getScreenDelta(sessionId, paneId, fromSequence) {
      assertOpen();
      const snapshot = await this.getScreenSnapshot(sessionId, paneId);
      const delta: ScreenDelta = {
        pane_id: snapshot.pane_id,
        from_sequence: fromSequence,
        to_sequence: snapshot.sequence,
        rows: snapshot.rows,
        cols: snapshot.cols,
        source: snapshot.source,
        patch: null,
        full_replace: snapshot.surface,
      };
      return clone(delta);
    },
    async dispatchMuxCommand() {
      assertOpen();
      const result: MuxCommandResult = { changed: true };
      return clone(result);
    },
    async openSubscription(sessionId, spec) {
      assertOpen();
      const event = await createInitialSubscriptionEvent(state, sessionId, spec);
      return new MemoryWorkspaceSubscription(event);
    },
    async close() {
      closed = true;
    },
  };

  function assertOpen() {
    if (closed) {
      throw new WorkspaceError({
        code: "disposed",
        message: "memory workspace transport is closed",
        recoverable: false,
      });
    }
  }
}

export function createDefaultMemoryWorkspaceFixture(): MemoryWorkspaceFixture {
  const backend: BackendKind = "native";
  const sessionId = "memory-session-1";
  const paneId = "memory-pane-1";
  const focusedTab = "memory-tab-1";
  const session = createSessionSummary(sessionId, backend, "Native shell");
  const topology = createTopologySnapshot(sessionId, backend, focusedTab, paneId, "Native shell");
  const screen = createScreenSnapshot(paneId, "shell", "ready");
  const attachedSession: AttachedSession = {
    session,
    health: createSessionHealthSnapshot(session.session_id),
    topology,
    focused_screen: screen,
  };
  const savedSession = createSavedSessionRecord(session, topology, screen);

  return {
    handshake: createDefaultHandshake(),
    sessions: [session],
    savedSessions: [savedSessionToSummary(savedSession)],
    savedSessionRecords: {
      [savedSession.session_id]: savedSession,
    },
    discoveredSessions: {
      native: [],
      tmux: [],
      zellij: [],
    },
    backendCapabilities: {
      native: {
        backend,
        capabilities: createDefaultBackendCapabilities(),
      },
    },
    attachedSessions: {
      [sessionId]: attachedSession,
    },
    topologyBySessionId: {
      [sessionId]: topology,
    },
    screensBySessionId: {
      [sessionId]: {
        [paneId]: screen,
      },
    },
  };
}

class MemoryWorkspaceSubscription implements WorkspaceSubscription {
  #events: SubscriptionEvent[];
  #closed = false;

  constructor(initialEvent: SubscriptionEvent) {
    this.#events = [initialEvent];
  }

  meta(): SubscriptionMeta {
    return {
      subscription_id: "memory-subscription-1",
    };
  }

  async nextEvent(): Promise<SubscriptionEvent | null> {
    if (this.#closed) {
      return null;
    }

    return this.#events.shift() ?? null;
  }

  async close(): Promise<void> {
    this.#closed = true;
    this.#events = [];
  }
}

async function createInitialSubscriptionEvent(
  fixture: MemoryWorkspaceFixture,
  sessionId: SessionId,
  spec: SubscriptionSpec,
): Promise<SubscriptionEvent> {
  if (spec.kind === "session_topology") {
    return {
      kind: "topology_snapshot",
      ...clone(requireRecord(fixture.topologyBySessionId, sessionId, "topology snapshot")),
    };
  }

  const screen = clone(
    requireRecord(
      requireRecord(fixture.screensBySessionId, sessionId, "screen collection"),
      spec.pane_id,
      "screen snapshot",
    ),
  );

  return {
    kind: "screen_delta",
    pane_id: screen.pane_id,
    from_sequence: screen.sequence,
    to_sequence: screen.sequence,
    rows: screen.rows,
    cols: screen.cols,
    source: screen.source,
    patch: null,
    full_replace: screen.surface,
  };
}

function createDefaultHandshake(): Handshake {
  return {
    protocol_version: {
      major: 0,
      minor: 1,
    },
    binary_version: "0.1.0-dev",
    daemon_phase: "ready",
    capabilities: {
      request_reply: true,
      topology_subscriptions: true,
      pane_subscriptions: true,
      backend_discovery: true,
      backend_capability_queries: true,
      saved_sessions: true,
      session_restore: true,
      degraded_error_reasons: true,
      session_health: true,
    },
    available_backends: ["native", "tmux", "zellij"],
    session_scope: "memory-fixture",
  };
}

function createDefaultBackendCapabilities(): BackendCapabilitiesInfo["capabilities"] {
  return {
    tiled_panes: true,
    floating_panes: false,
    split_resize: true,
    tab_create: true,
    tab_close: true,
    tab_focus: true,
    tab_rename: true,
    session_scoped_tab_refs: true,
    session_scoped_pane_refs: true,
    pane_split: true,
    pane_close: true,
    pane_focus: true,
    pane_input_write: true,
    pane_paste_write: true,
    raw_output_stream: false,
    rendered_viewport_stream: true,
    rendered_viewport_snapshot: true,
    rendered_scrollback_snapshot: false,
    layout_dump: true,
    layout_override: true,
    read_only_client_mode: false,
    explicit_session_save: true,
    explicit_session_restore: true,
    plugin_panes: false,
    advisory_metadata_subscriptions: true,
    independent_resize_authority: true,
  };
}

function createSessionSummary(
  sessionId: SessionId,
  backend: BackendKind,
  title: string | null,
): SessionSummary {
  return {
    session_id: sessionId,
    route: {
      backend,
      authority: backend === "native" ? "local_daemon" : "imported_foreign",
      external:
        backend === "native"
          ? {
              namespace: "native_session",
              value: sessionId,
            }
          : null,
    },
    title,
  };
}

function createTopologySnapshot(
  sessionId: SessionId,
  backend: BackendKind,
  tabId: string,
  paneId: string,
  title: string | null,
): TopologySnapshot {
  return {
    session_id: sessionId,
    backend_kind: backend,
    tabs: [
      {
        tab_id: tabId,
        title,
        root: {
          kind: "leaf",
          pane_id: paneId,
        },
        focused_pane: paneId,
      },
    ],
    focused_tab: tabId,
  };
}

function createScreenSnapshot(
  paneId: PaneId,
  title: string | null,
  line: string,
  source: ProjectionSource = "native_emulator",
): ScreenSnapshot {
  return {
    pane_id: paneId,
    sequence: 1n,
    rows: 24,
    cols: 80,
    source,
    surface: createScreenSurface(title, line),
  };
}

function createScreenSurface(title: string | null, line: string): ScreenSurface {
  return {
    title,
    cursor: {
      row: 0,
      col: 0,
    },
    lines: [{ text: line }],
  };
}

function createSavedSessionRecord(
  session: SessionSummary,
  topology: TopologySnapshot,
  screen: ScreenSnapshot,
): SavedSessionRecord {
  return {
    session_id: session.session_id,
    route: session.route,
    title: session.title,
    launch: {
      program: "/bin/sh",
      args: [],
      cwd: null,
    },
    manifest: {
      format_version: 1,
      binary_version: "0.1.0-dev",
      protocol_major: 0,
      protocol_minor: 1,
    },
    compatibility: {
      can_restore: true,
      status: "compatible",
    },
    topology,
    screens: [screen],
    saved_at_ms: 1n,
    restore_semantics: {
      restores_topology: true,
      restores_focus_state: true,
      restores_tab_titles: true,
      uses_saved_launch_spec: true,
      replays_saved_screen_buffers: true,
      preserves_process_state: false,
    },
  };
}

function savedSessionToSummary(record: SavedSessionRecord): SavedSessionSummary {
  return {
    session_id: record.session_id,
    route: record.route,
    title: record.title,
    saved_at_ms: record.saved_at_ms,
    manifest: record.manifest,
    compatibility: record.compatibility,
    has_launch: record.launch !== null,
    tab_count: record.topology.tabs.length,
    pane_count: record.screens.length,
    restore_semantics: record.restore_semantics,
  };
}

function recordToSessionSummary(record: SavedSessionRecord): SessionSummary {
  return {
    session_id: record.session_id,
    route: record.route,
    title: record.title,
  };
}

function seedSessionArtifacts(
  state: MemoryWorkspaceFixture,
  session: SessionSummary,
  title: string | null,
): void {
  const paneId = `${session.session_id}-pane-1`;
  const tabId = `${session.session_id}-tab-1`;
  const topology = createTopologySnapshot(session.session_id, session.route.backend, tabId, paneId, title);
  const screen = createScreenSnapshot(paneId, title, "ready");
  state.topologyBySessionId[session.session_id] = topology;
  state.screensBySessionId[session.session_id] = {
    [paneId]: screen,
  };
  state.attachedSessions[session.session_id] = {
    session,
    health: createSessionHealthSnapshot(session.session_id),
    topology,
    focused_screen: screen,
  };
}

function seedSavedSessionArtifacts(state: MemoryWorkspaceFixture, record: SavedSessionRecord): void {
  const session = recordToSessionSummary(record);
  state.topologyBySessionId[record.session_id] = record.topology;
  state.screensBySessionId[record.session_id] = Object.fromEntries(
    record.screens.map((screen) => [screen.pane_id, screen]),
  );
  state.attachedSessions[record.session_id] = {
    session,
    health: createSessionHealthSnapshot(record.session_id),
    topology: record.topology,
    focused_screen: record.screens[0] ?? null,
  };
}

function createSessionHealthSnapshot(sessionId: SessionId): AttachedSession["health"] {
  return {
    session_id: sessionId,
    phase: "ready",
    can_attach: true,
    invalidated: false,
    reason: null,
    detail: null,
  };
}

function requireRecord<TRecord>(source: Record<string, TRecord>, key: string, label: string): TRecord {
  const record = source[key];
  if (!record) {
    throw new WorkspaceError({
      code: "session_not_found",
      message: `missing ${label} for ${key}`,
      recoverable: false,
    });
  }

  return record;
}

function clone<TValue>(value: TValue): TValue {
  return structuredClone(value);
}
