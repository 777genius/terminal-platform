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
  PaneTreeNode,
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
  SplitDirection,
  SubscriptionEvent,
  SubscriptionMeta,
  SubscriptionSpec,
  TabSnapshot,
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
  let topologyMutationCounter = countTopologyLeaves(state.topologyBySessionId) + countTopologyTabs(state.topologyBySessionId);
  let savedSessionCounter = resolveInitialSavedSessionCounter(state.savedSessions);

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
      requireRecord(state.savedSessionRecords, sessionId, "saved session");
      delete state.savedSessionRecords[sessionId];
      state.savedSessions = state.savedSessions.filter((session) => session.session_id !== sessionId);
      const result: DeleteSavedSessionResult = { session_id: sessionId };
      return clone(result);
    },
    async pruneSavedSessions(keepLatest) {
      assertOpen();
      const keepLatestCount = normalizeSavedSessionPruneLimit(keepLatest);
      const sorted = [...state.savedSessions].sort((left, right) => {
        if (left.saved_at_ms === right.saved_at_ms) {
          return 0;
        }

        return left.saved_at_ms > right.saved_at_ms ? -1 : 1;
      });
      const kept = sorted.slice(0, keepLatestCount);
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
      assertSavedSessionRestorable(record);
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
    async dispatchMuxCommand(sessionId, command) {
      assertOpen();
      if (command.kind === "send_input" || command.kind === "send_paste") {
        appendSyntheticInputToScreen(state, sessionId, command.pane_id, command.data, command.kind);
      }
      if (command.kind === "new_tab") {
        topologyMutationCounter += 1;
        addSyntheticTab(state, sessionId, topologyMutationCounter, command.title);
      }
      if (command.kind === "split_pane") {
        topologyMutationCounter += 1;
        splitSyntheticPane(state, sessionId, topologyMutationCounter, command.pane_id, command.direction);
      }
      if (command.kind === "close_pane") {
        closeSyntheticPane(state, sessionId, command.pane_id);
      }
      if (command.kind === "resize_pane") {
        resizeSyntheticPane(state, sessionId, command.pane_id, command.rows, command.cols);
      }
      if (command.kind === "focus_pane") {
        focusSyntheticPane(state, sessionId, command.pane_id);
      }
      if (command.kind === "focus_tab") {
        focusSyntheticTab(state, sessionId, command.tab_id);
      }
      if (command.kind === "rename_tab") {
        renameSyntheticTab(state, sessionId, command.tab_id, command.title);
      }
      if (command.kind === "close_tab") {
        closeSyntheticTab(state, sessionId, command.tab_id);
      }
      if (command.kind === "save_session") {
        savedSessionCounter += 1;
        saveSessionSnapshot(state, sessionId, savedSessionCounter);
      }
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
      minor: 2,
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
  options: {
    savedSessionId?: SessionId;
    savedAtMs?: bigint;
    title?: string | null;
    screens?: readonly ScreenSnapshot[];
  } = {},
): SavedSessionRecord {
  return {
    session_id: options.savedSessionId ?? session.session_id,
    route: session.route,
    title: options.title ?? session.title,
    launch: {
      program: "/bin/sh",
      args: [],
      cwd: null,
    },
    manifest: {
      format_version: 1,
      binary_version: "0.1.0-dev",
      protocol_major: 0,
      protocol_minor: 2,
    },
    compatibility: {
      can_restore: true,
      status: "compatible",
    },
    topology: clone(topology),
    screens: clone([...(options.screens ?? [screen])]),
    saved_at_ms: options.savedAtMs ?? 1n,
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

function appendSyntheticInputToScreen(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  paneId: PaneId,
  data: string,
  kind: "send_input" | "send_paste",
): void {
  const screens = requireRecord(state.screensBySessionId, sessionId, "screen collection");
  const screen = requireRecord(screens, paneId, "screen snapshot");
  const renderedLines = renderSyntheticInputLines(data, kind);

  if (renderedLines.length === 0) {
    return;
  }

  screen.sequence += 1n;
  screen.surface.lines = [...screen.surface.lines, ...renderedLines]
    .slice(-screen.rows);

  const cursorRow = Math.max(0, screen.surface.lines.length - 1);
  screen.surface.cursor = {
    row: cursorRow,
    col: screen.surface.lines[cursorRow]?.text.length ?? 0,
  };

  const attachedSession = state.attachedSessions[sessionId];
  if (attachedSession?.focused_screen?.pane_id === paneId) {
    attachedSession.focused_screen = screen;
  }
}

function addSyntheticTab(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  mutationIndex: number,
  title: string | null,
): void {
  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const paneId = `${sessionId}-pane-${mutationIndex}`;
  const tab: TabSnapshot = {
    tab_id: `${sessionId}-tab-${mutationIndex}`,
    title,
    root: {
      kind: "leaf",
      pane_id: paneId,
    },
    focused_pane: paneId,
  };
  const screen = createScreenSnapshot(paneId, title, "ready");

  topology.tabs.push(tab);
  topology.focused_tab = tab.tab_id;
  requireRecord(state.screensBySessionId, sessionId, "screen collection")[paneId] = screen;
  updateAttachedSessionFocus(state, sessionId, topology, screen);
}

function splitSyntheticPane(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  mutationIndex: number,
  paneId: PaneId,
  direction: SplitDirection,
): void {
  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const tab = topology.tabs.find((candidate) => containsPane(candidate.root, paneId));
  if (!tab) {
    throw new WorkspaceError({
      code: "pane_not_found",
      message: `missing pane for ${paneId}`,
      recoverable: false,
    });
  }

  const nextPaneId = `${sessionId}-pane-${mutationIndex}`;
  const changed = splitPaneTreeLeaf(tab.root, paneId, direction, nextPaneId);
  if (!changed) {
    throw new WorkspaceError({
      code: "pane_not_found",
      message: `missing pane for ${paneId}`,
      recoverable: false,
    });
  }

  const screen = createScreenSnapshot(nextPaneId, tab.title, "ready");
  tab.focused_pane = nextPaneId;
  topology.focused_tab = tab.tab_id;
  requireRecord(state.screensBySessionId, sessionId, "screen collection")[nextPaneId] = screen;
  updateAttachedSessionFocus(state, sessionId, topology, screen);
}

function focusSyntheticPane(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  paneId: PaneId,
): void {
  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const tab = topology.tabs.find((candidate) => containsPane(candidate.root, paneId));
  if (!tab) {
    throw new WorkspaceError({
      code: "pane_not_found",
      message: `missing pane for ${paneId}`,
      recoverable: false,
    });
  }

  const screen = requireRecord(
    requireRecord(state.screensBySessionId, sessionId, "screen collection"),
    paneId,
    "screen snapshot",
  );
  tab.focused_pane = paneId;
  topology.focused_tab = tab.tab_id;
  updateAttachedSessionFocus(state, sessionId, topology, screen);
}

function closeSyntheticPane(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  paneId: PaneId,
): void {
  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const tab = topology.tabs.find((candidate) => containsPane(candidate.root, paneId));
  if (!tab) {
    throw new WorkspaceError({
      code: "pane_not_found",
      message: `missing pane for ${paneId}`,
      recoverable: false,
    });
  }

  if (countPaneTreeLeaves(tab.root) <= 1) {
    throw new WorkspaceError({
      code: "unsupported_capability",
      message: "memory workspace keeps at least one pane per tab",
      recoverable: false,
    });
  }

  const removal = removePaneTreeLeaf(tab.root, paneId);
  if (!removal.removed || !removal.node) {
    throw new WorkspaceError({
      code: "pane_not_found",
      message: `missing pane for ${paneId}`,
      recoverable: false,
    });
  }

  tab.root = removal.node;
  const screens = requireRecord(state.screensBySessionId, sessionId, "screen collection");
  delete screens[paneId];

  const nextPaneId = tab.focused_pane === paneId
    ? firstPaneId(tab.root)
    : tab.focused_pane && containsPane(tab.root, tab.focused_pane)
      ? tab.focused_pane
      : firstPaneId(tab.root);
  tab.focused_pane = nextPaneId;
  topology.focused_tab = tab.tab_id;
  updateAttachedSessionFocus(state, sessionId, topology, requireRecord(screens, nextPaneId, "screen snapshot"));
}

function resizeSyntheticPane(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  paneId: PaneId,
  rows: number,
  cols: number,
): void {
  const screens = requireRecord(state.screensBySessionId, sessionId, "screen collection");
  const screen = requireRecord(screens, paneId, "screen snapshot");
  const nextRows = Math.max(1, Math.trunc(rows));
  const nextCols = Math.max(1, Math.trunc(cols));

  if (screen.rows === nextRows && screen.cols === nextCols) {
    return;
  }

  screen.sequence += 1n;
  screen.rows = nextRows;
  screen.cols = nextCols;
  screen.surface.lines = screen.surface.lines.slice(-nextRows);
  const cursorRow = Math.min(
    Math.max(screen.surface.cursor?.row ?? 0, 0),
    Math.max(nextRows - 1, 0),
  );
  const cursorCol = Math.min(
    Math.max(screen.surface.cursor?.col ?? 0, 0),
    Math.max(nextCols - 1, 0),
  );
  screen.surface.cursor = {
    row: cursorRow,
    col: cursorCol,
  };

  const attachedSession = state.attachedSessions[sessionId];
  if (attachedSession?.focused_screen?.pane_id === paneId) {
    attachedSession.focused_screen = screen;
  }
}

function focusSyntheticTab(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  tabId: string,
): void {
  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const tab = topology.tabs.find((candidate) => candidate.tab_id === tabId);
  if (!tab) {
    throw new WorkspaceError({
      code: "session_not_found",
      message: `missing tab for ${tabId}`,
      recoverable: false,
    });
  }

  const paneId = tab.focused_pane ?? firstPaneId(tab.root);
  const screen = requireRecord(
    requireRecord(state.screensBySessionId, sessionId, "screen collection"),
    paneId,
    "screen snapshot",
  );
  tab.focused_pane = paneId;
  topology.focused_tab = tab.tab_id;
  updateAttachedSessionFocus(state, sessionId, topology, screen);
}

function closeSyntheticTab(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  tabId: string,
): void {
  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const tabIndex = topology.tabs.findIndex((candidate) => candidate.tab_id === tabId);
  if (tabIndex === -1) {
    throw new WorkspaceError({
      code: "session_not_found",
      message: `missing tab for ${tabId}`,
      recoverable: false,
    });
  }

  if (topology.tabs.length <= 1) {
    throw new WorkspaceError({
      code: "unsupported_capability",
      message: "memory workspace keeps at least one tab per session",
      recoverable: false,
    });
  }

  const [closedTab] = topology.tabs.splice(tabIndex, 1);
  const screens = requireRecord(state.screensBySessionId, sessionId, "screen collection");
  for (const paneId of collectPaneIds(closedTab!.root)) {
    delete screens[paneId];
  }

  const focusedTab = topology.focused_tab === tabId
    ? topology.tabs[Math.min(tabIndex, topology.tabs.length - 1)]!
    : topology.tabs.find((candidate) => candidate.tab_id === topology.focused_tab) ?? topology.tabs[0]!;
  const nextPaneId = focusedTab.focused_pane ?? firstPaneId(focusedTab.root);
  focusedTab.focused_pane = nextPaneId;
  topology.focused_tab = focusedTab.tab_id;
  updateAttachedSessionFocus(state, sessionId, topology, requireRecord(screens, nextPaneId, "screen snapshot"));
}

function renameSyntheticTab(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  tabId: string,
  title: string,
): void {
  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const tab = topology.tabs.find((candidate) => candidate.tab_id === tabId);
  if (!tab) {
    throw new WorkspaceError({
      code: "session_not_found",
      message: `missing tab for ${tabId}`,
      recoverable: false,
    });
  }

  tab.title = title;
  const attachedSession = state.attachedSessions[sessionId];
  if (attachedSession) {
    attachedSession.topology = topology;
  }
}

function updateAttachedSessionFocus(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  topology: TopologySnapshot,
  screen: ScreenSnapshot,
): void {
  const attachedSession = state.attachedSessions[sessionId];
  if (attachedSession) {
    attachedSession.topology = topology;
    attachedSession.focused_screen = screen;
  }
}

function splitPaneTreeLeaf(
  node: PaneTreeNode,
  paneId: PaneId,
  direction: SplitDirection,
  nextPaneId: PaneId,
): boolean {
  if (node.kind === "leaf") {
    if (node.pane_id !== paneId) {
      return false;
    }

    delete (node as { pane_id?: PaneId }).pane_id;
    Object.assign(node, {
      kind: "split",
      direction,
      first: {
        kind: "leaf",
        pane_id: paneId,
      },
      second: {
        kind: "leaf",
        pane_id: nextPaneId,
      },
    });
    return true;
  }

  return splitPaneTreeLeaf(node.first, paneId, direction, nextPaneId)
    || splitPaneTreeLeaf(node.second, paneId, direction, nextPaneId);
}

function removePaneTreeLeaf(
  node: PaneTreeNode,
  paneId: PaneId,
): { node: PaneTreeNode | null; removed: boolean } {
  if (node.kind === "leaf") {
    return node.pane_id === paneId
      ? { node: null, removed: true }
      : { node, removed: false };
  }

  const first = removePaneTreeLeaf(node.first, paneId);
  if (first.removed) {
    return {
      node: first.node ? { ...node, first: first.node } : node.second,
      removed: true,
    };
  }

  const second = removePaneTreeLeaf(node.second, paneId);
  if (second.removed) {
    return {
      node: second.node ? { ...node, second: second.node } : node.first,
      removed: true,
    };
  }

  return { node, removed: false };
}

function containsPane(node: PaneTreeNode, paneId: PaneId): boolean {
  if (node.kind === "leaf") {
    return node.pane_id === paneId;
  }

  return containsPane(node.first, paneId) || containsPane(node.second, paneId);
}

function firstPaneId(node: PaneTreeNode): PaneId {
  if (node.kind === "leaf") {
    return node.pane_id;
  }

  return firstPaneId(node.first);
}

function collectPaneIds(node: PaneTreeNode): PaneId[] {
  if (node.kind === "leaf") {
    return [node.pane_id];
  }

  return [...collectPaneIds(node.first), ...collectPaneIds(node.second)];
}

function saveSessionSnapshot(
  state: MemoryWorkspaceFixture,
  sessionId: SessionId,
  savedSessionIndex: number,
): void {
  const session = state.sessions.find((candidate) => candidate.session_id === sessionId);
  if (!session) {
    throw new WorkspaceError({
      code: "session_not_found",
      message: `missing session for ${sessionId}`,
      recoverable: false,
    });
  }

  const topology = requireRecord(state.topologyBySessionId, sessionId, "topology snapshot");
  const screens = Object.values(requireRecord(state.screensBySessionId, sessionId, "screen collection"));
  const focusedPaneId = state.attachedSessions[sessionId]?.focused_screen?.pane_id;
  const focusedScreen = screens.find((screen) => screen.pane_id === focusedPaneId) ?? screens[0];
  if (!focusedScreen) {
    throw new WorkspaceError({
      code: "session_not_found",
      message: `missing screen snapshot for ${sessionId}`,
      recoverable: false,
    });
  }

  const savedSessionId = `${sessionId}-saved-${savedSessionIndex}`;
  const record = createSavedSessionRecord(session, topology, focusedScreen, {
    savedSessionId,
    savedAtMs: BigInt(savedSessionIndex),
    title: `${session.title ?? session.session_id} snapshot ${savedSessionIndex}`,
    screens,
  });

  state.savedSessionRecords[savedSessionId] = record;
  state.savedSessions = [
    savedSessionToSummary(record),
    ...state.savedSessions.filter((savedSession) => savedSession.session_id !== savedSessionId),
  ];
}

function resolveInitialSavedSessionCounter(savedSessions: readonly SavedSessionSummary[]): number {
  return savedSessions.reduce((maxCounter, savedSession) => {
    const suffixMatch = /-saved-(\d+)$/u.exec(savedSession.session_id);
    if (!suffixMatch) {
      return maxCounter;
    }

    const parsedCounter = Number.parseInt(suffixMatch[1]!, 10);
    return Number.isSafeInteger(parsedCounter)
      ? Math.max(maxCounter, parsedCounter)
      : maxCounter;
  }, savedSessions.length);
}

function countTopologyTabs(topologyBySessionId: Record<string, TopologySnapshot>): number {
  return Object.values(topologyBySessionId).reduce(
    (count, topology) => count + topology.tabs.length,
    0,
  );
}

function countTopologyLeaves(topologyBySessionId: Record<string, TopologySnapshot>): number {
  return Object.values(topologyBySessionId).reduce(
    (count, topology) => count + topology.tabs.reduce(
      (tabCount, tab) => tabCount + countPaneTreeLeaves(tab.root),
      0,
    ),
    0,
  );
}

function countPaneTreeLeaves(node: PaneTreeNode): number {
  if (node.kind === "leaf") {
    return 1;
  }

  return countPaneTreeLeaves(node.first) + countPaneTreeLeaves(node.second);
}

function renderSyntheticInputLines(
  data: string,
  kind: "send_input" | "send_paste",
): ScreenSurface["lines"] {
  const normalizedData = data.replace(/\r\n?/gu, "\n");
  if (normalizedData === "\u0003") {
    return [{ text: "^C" }];
  }

  const trimmedData = normalizedData.replace(/\s+$/u, "");
  if (!trimmedData) {
    return [{ text: "" }];
  }

  const prefix = kind === "send_paste" ? "paste" : "$";
  return trimmedData.split("\n").map((line, index) => ({
    text: index === 0 ? `${prefix} ${line}` : line,
  }));
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

function assertSavedSessionRestorable(record: SavedSessionRecord): void {
  if (record.compatibility.can_restore) {
    return;
  }

  throw new WorkspaceError({
    code: "unsupported_capability",
    message: `saved session ${record.session_id} is not restore-compatible: ${record.compatibility.status}`,
    recoverable: false,
  });
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

function clone<TValue>(value: TValue): TValue {
  return structuredClone(value);
}
