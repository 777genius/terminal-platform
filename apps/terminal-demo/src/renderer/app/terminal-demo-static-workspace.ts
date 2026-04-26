import type {
  BackendCapabilitiesInfo,
  BackendKind,
  MuxCommand,
  MuxCommandResult,
  PaneId,
  SavedSessionSummary,
  SessionId,
} from "@terminal-platform/runtime-types";
import {
  DEFAULT_COMMAND_HISTORY_LIMIT,
  createInitialWorkspaceSnapshot,
  type WorkspaceCommands,
  type WorkspaceDiagnostics,
  type WorkspaceKernel,
  type WorkspaceSelectors,
  type WorkspaceSnapshot,
} from "@terminal-platform/workspace-core";

export interface TerminalDemoPreviewBootstrapConfig {
  runtimeSlug: string;
}

export const DEFAULT_TERMINAL_DEMO_DISPLAY = {
  fontScale: "default",
  lineWrap: true,
} satisfies WorkspaceSnapshot["terminalDisplay"];

export function createStaticWorkspaceKernel(snapshot: WorkspaceSnapshot): WorkspaceKernel {
  let currentSnapshot: WorkspaceSnapshot = {
    ...snapshot,
    commandHistory: snapshot.commandHistory ?? {
      entries: [],
      limit: DEFAULT_COMMAND_HISTORY_LIMIT,
    },
    terminalDisplay: snapshot.terminalDisplay ?? DEFAULT_TERMINAL_DEMO_DISPLAY,
  };
  const listeners = new Set<() => void>();
  const notify = () => {
    for (const listener of listeners) {
      listener();
    }
  };
  const updateSnapshot = (nextSnapshot: WorkspaceSnapshot) => {
    currentSnapshot = nextSnapshot;
    notify();
  };
  const noopAsync = async () => {};
  const noopDiagnostics: WorkspaceDiagnostics = {
    list: () => currentSnapshot.diagnostics,
    clear: () => updateSnapshot({
      ...currentSnapshot,
      diagnostics: [],
    }),
  };
  const noopCommands: WorkspaceCommands = {
    bootstrap: noopAsync,
    refreshSessions: noopAsync,
    refreshSavedSessions: noopAsync,
    discoverSessions: noopAsync,
    getBackendCapabilities: async (backend: BackendKind) => {
      const capabilities = currentSnapshot.catalog.backendCapabilities[backend];
      if (!capabilities) {
        throw new Error(`No static backend capabilities for ${backend}`);
      }

      return capabilities;
    },
    createSession: noopAsync,
    importSession: noopAsync,
    attachSession: noopAsync,
    restoreSavedSession: noopAsync,
    deleteSavedSession: noopAsync,
    pruneSavedSessions: async (keepLatest: number) => ({
      deleted_count: Math.max(0, currentSnapshot.catalog.savedSessions.length - keepLatest),
      kept_count: Math.min(currentSnapshot.catalog.savedSessions.length, keepLatest),
    }),
    dispatchMuxCommand: async (sessionId: SessionId, command: MuxCommand) =>
      dispatchStaticMuxCommand(currentSnapshot, updateSnapshot, sessionId, command),
    openSubscription: async () => {
      throw new Error("not implemented in static kernel");
    },
    setActiveSession: (sessionId) => updateSnapshot({
      ...currentSnapshot,
      selection: {
        ...currentSnapshot.selection,
        activeSessionId: sessionId,
      },
    }),
    setActivePane: (paneId) => updateSnapshot({
      ...currentSnapshot,
      selection: {
        ...currentSnapshot.selection,
        activePaneId: paneId,
      },
    }),
    updateDraft: (paneId, value) => updateSnapshot({
      ...currentSnapshot,
      drafts: {
        ...currentSnapshot.drafts,
        [paneId]: value,
      },
    }),
    clearDraft: (paneId) => {
      const nextDrafts = { ...currentSnapshot.drafts };
      delete nextDrafts[paneId];
      updateSnapshot({
        ...currentSnapshot,
        drafts: nextDrafts,
      });
    },
    recordCommandHistory: (value) => {
      const normalizedValue = value.trim();
      if (!normalizedValue) {
        return;
      }

      const limit = currentSnapshot.commandHistory.limit;
      updateSnapshot({
        ...currentSnapshot,
        commandHistory: {
          ...currentSnapshot.commandHistory,
          entries: [
            ...currentSnapshot.commandHistory.entries.filter((entry) => entry !== normalizedValue),
            normalizedValue,
          ].slice(-limit),
        },
      });
    },
    clearCommandHistory: () => updateSnapshot({
      ...currentSnapshot,
      commandHistory: {
        ...currentSnapshot.commandHistory,
        entries: [],
      },
    }),
    setTheme: (themeId) => updateSnapshot({
      ...currentSnapshot,
      theme: { themeId },
    }),
    setTerminalFontScale: (fontScale) => updateSnapshot({
      ...currentSnapshot,
      terminalDisplay: {
        ...currentSnapshot.terminalDisplay,
        fontScale: fontScale as WorkspaceSnapshot["terminalDisplay"]["fontScale"],
      },
    }),
    setTerminalLineWrap: (lineWrap) => updateSnapshot({
      ...currentSnapshot,
      terminalDisplay: {
        ...currentSnapshot.terminalDisplay,
        lineWrap,
      },
    }),
    clearDiagnostics: () => noopDiagnostics.clear(),
  };
  const noopSelectors: WorkspaceSelectors = {
    connection: () => currentSnapshot.connection,
    sessions: () => currentSnapshot.catalog.sessions,
    savedSessions: () => currentSnapshot.catalog.savedSessions,
    activeSession: () => currentSnapshot.catalog.sessions.find(
      (item) => item.session_id === currentSnapshot.selection.activeSessionId,
    ) ?? null,
    activePaneId: () => currentSnapshot.selection.activePaneId,
    attachedSession: () => currentSnapshot.attachedSession,
    diagnostics: () => currentSnapshot.diagnostics,
    themeId: () => currentSnapshot.theme.themeId,
    terminalDisplay: () => currentSnapshot.terminalDisplay,
    commandHistory: () => currentSnapshot.commandHistory,
  };

  return {
    getSnapshot: () => currentSnapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    bootstrap: noopAsync,
    dispose: noopAsync,
    commands: noopCommands,
    selectors: noopSelectors,
    diagnostics: noopDiagnostics,
  };
}

function dispatchStaticMuxCommand(
  snapshot: WorkspaceSnapshot,
  updateSnapshot: (snapshot: WorkspaceSnapshot) => void,
  sessionId: SessionId,
  command: MuxCommand,
): Promise<MuxCommandResult> {
  if (!snapshot.attachedSession || snapshot.attachedSession.session.session_id !== sessionId) {
    return Promise.resolve({ changed: false });
  }

  if (command.kind === "send_input" || command.kind === "send_paste") {
    updateSnapshot(appendStaticPreviewInput(snapshot, command.pane_id as PaneId, command.data, command.kind));
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "save_session") {
    updateSnapshot({
      ...snapshot,
      catalog: {
        ...snapshot.catalog,
        savedSessions: [createStaticSavedSessionSummary(snapshot), ...snapshot.catalog.savedSessions].slice(0, 6),
      },
    });
    return Promise.resolve({ changed: true });
  }

  return Promise.resolve({ changed: false });
}

function appendStaticPreviewInput(
  snapshot: WorkspaceSnapshot,
  paneId: PaneId,
  data: string,
  kind: "send_input" | "send_paste",
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const focusedScreen = attachedSession?.focused_screen ?? null;
  if (!attachedSession || !focusedScreen || focusedScreen.pane_id !== paneId) {
    return snapshot;
  }

  const commandText = formatStaticPreviewInput(data, kind);
  const nextLines = [
    ...focusedScreen.surface.lines,
    { text: commandText },
    { text: "preview runtime accepted input without native host" },
  ].slice(-focusedScreen.rows);

  return {
    ...snapshot,
    attachedSession: {
      ...attachedSession,
      focused_screen: {
        ...focusedScreen,
        sequence: focusedScreen.sequence + 1n,
        surface: {
          ...focusedScreen.surface,
          lines: nextLines,
        },
      },
    },
  };
}

function formatStaticPreviewInput(data: string, kind: "send_input" | "send_paste"): string {
  if (data === "\u0003") {
    return "^C";
  }

  if (data === "\r" || data === "\n") {
    return "";
  }

  const normalizedData = data.replace(/\r\n/g, "\n").replace(/\r/g, "\n").trimEnd();
  const prefix = kind === "send_paste" ? "paste" : "$";
  return normalizedData
    .split("\n")
    .filter((line) => line.length > 0)
    .map((line) => `${prefix} ${line}`)
    .join("\n");
}

function createStaticSavedSessionSummary(snapshot: WorkspaceSnapshot): SavedSessionSummary {
  const session = snapshot.attachedSession?.session ?? snapshot.catalog.sessions[0];
  if (!session) {
    throw new Error("Static preview cannot save a layout without a session");
  }

  return {
    session_id: "preview-saved-session" as SessionId,
    route: session.route,
    title: "Preview saved layout",
    saved_at_ms: BigInt(Date.now()),
    manifest: {
      format_version: 1,
      binary_version: "preview",
      protocol_major: snapshot.connection.handshake?.protocol_version.major ?? 0,
      protocol_minor: snapshot.connection.handshake?.protocol_version.minor ?? 2,
    },
    compatibility: {
      can_restore: true,
      status: "compatible",
    },
    has_launch: true,
    tab_count: snapshot.attachedSession?.topology.tabs.length ?? 1,
    pane_count: 1,
    restore_semantics: {
      restores_topology: true,
      restores_focus_state: true,
      restores_tab_titles: true,
      uses_saved_launch_spec: true,
      replays_saved_screen_buffers: false,
      preserves_process_state: false,
    },
  };
}

export function createDemoPreviewWorkspaceSnapshot(config: TerminalDemoPreviewBootstrapConfig): WorkspaceSnapshot {
  const sessionId = "preview-session-native" as SessionId;
  const paneId = "preview-pane-main";
  const tabId = "preview-tab-shell";
  const session = {
    session_id: sessionId,
    origin: {
      backend: "native" as const,
      authority: "local_daemon" as const,
      foreign_reference_label: null,
    },
    route: {
      backend: "native" as const,
      authority: "local_daemon" as const,
      external: null,
    },
    title: "Terminal Platform preview",
    degraded_semantics: [],
  };

  return {
    ...createInitialWorkspaceSnapshot({
      terminalFontScale: DEFAULT_TERMINAL_DEMO_DISPLAY.fontScale,
      terminalLineWrap: DEFAULT_TERMINAL_DEMO_DISPLAY.lineWrap,
    }),
    connection: {
      state: "ready",
      handshake: {
        protocol_version: { major: 0, minor: 2 },
        binary_version: "preview",
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
        available_backends: ["native"],
        session_scope: config.runtimeSlug,
      },
      lastError: null,
    },
    catalog: {
      sessions: [session],
      savedSessions: [],
      discoveredSessions: {},
      backendCapabilities: {
        native: createDemoPreviewBackendCapabilities("native"),
      },
    },
    selection: {
      activeSessionId: sessionId,
      activePaneId: paneId,
    },
    attachedSession: {
      session,
      health: {
        session_id: sessionId,
        phase: "ready",
        can_attach: true,
        invalidated: false,
        reason: null,
        detail: null,
      },
      topology: {
        session_id: sessionId,
        backend_kind: "native",
        focused_tab: tabId,
        tabs: [
          {
            tab_id: tabId,
            title: "Shell",
            focused_pane: paneId,
            root: {
              kind: "leaf",
              pane_id: paneId,
            },
          },
        ],
      },
      focused_screen: {
        pane_id: paneId,
        sequence: 1n,
        rows: 24,
        cols: 96,
        source: "native_emulator",
        surface: {
          title: "Shell",
          cursor: null,
          lines: [
            { text: "terminal-platform demo static preview" },
            { text: "$ npm run build:renderer" },
            { text: "[ok] renderer bundle ready" },
            { text: "$ npm run smoke:browser" },
            { text: "waiting for native runtime and Chrome CDP..." },
          ],
        },
      },
    },
    drafts: {
      [paneId]: "git status",
    },
    commandHistory: {
      entries: ["npm run build:renderer", "npm run smoke:browser", "git status"],
      limit: DEFAULT_COMMAND_HISTORY_LIMIT,
    },
  };
}

export function createDemoPreviewBackendCapabilities(backend: BackendKind): BackendCapabilitiesInfo {
  return {
    backend,
    capabilities: {
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
    },
  };
}
