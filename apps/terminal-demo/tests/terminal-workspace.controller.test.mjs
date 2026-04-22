import test from "node:test";
import assert from "node:assert/strict";

import {
  initialTerminalWorkspaceViewState,
  TerminalWorkspaceController,
} from "../dist/features/terminal-workspace/core/application/index.js";

function createStore(overrides = {}) {
  let state = {
    ...structuredClone(initialTerminalWorkspaceViewState),
    ...overrides,
  };

  return {
    getState() {
      return state;
    },
    patch(patch) {
      state = {
        ...state,
        ...patch,
      };
    },
  };
}

function createCapabilities(backend, overrides = {}) {
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
      layout_dump: false,
      layout_override: false,
      read_only_client_mode: false,
      explicit_session_save: true,
      explicit_session_restore: true,
      plugin_panes: false,
      advisory_metadata_subscriptions: false,
      independent_resize_authority: false,
      ...overrides,
    },
    degradedSemantics: [],
  };
}

function createGateway(overrides = {}) {
  const sessions = overrides.sessions ?? [
    {
      session_id: "session-1",
      origin: {
        backend: "native",
        authority: "local_daemon",
        foreignReferenceLabel: null,
      },
      title: "Workspace",
      degradedSemantics: [],
    },
  ];
  const savedSessions = overrides.savedSessions ?? [];
  const discovered = overrides.discovered ?? {
    tmux: [
      {
        importHandle: "import-tmux-1",
        backend: "tmux",
        title: "Foreign",
        sourceLabel: "tmux",
        degradedSemantics: [],
      },
    ],
  };
  const sessionState = overrides.sessionState ?? {
    session: sessions[0],
    topology: {
      session_id: sessions[0].session_id,
      backend_kind: "native",
      focused_tab: "tab-1",
      tabs: [
        {
          tab_id: "tab-1",
          title: "Shell",
          focused_pane: "pane-1",
          root: {
            kind: "leaf",
            pane_id: "pane-1",
          },
        },
      ],
    },
    focusedScreen: {
      pane_id: "pane-1",
      sequence: "1",
      rows: 24,
      cols: 80,
      source: "native_emulator",
      surface: {
        title: null,
        cursor: null,
        lines: [
          { text: "demo" },
        ],
      },
    },
  };

  const gateway = {
    createNativeCalls: [],
    importCalls: [],
    subscribeCalls: [],
    subscriptionHandlers: null,
    dispatchCalls: [],
    async handshakeInfo() {
      return {
        handshake: {
          protocol_version: { major: 1, minor: 0 },
          binary_version: "1.0.0",
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
          },
          available_backends: ["native", "tmux"],
          session_scope: "terminal-demo",
        },
        assessment: {
          can_use: true,
          protocol: {
            can_connect: true,
            status: "compatible",
          },
          status: "ready",
        },
        degradedSemantics: [],
      };
    },
    async listSessions() {
      return sessions;
    },
    async listSavedSessions() {
      return savedSessions;
    },
    async discoverSessions(backend) {
      return discovered[backend] ?? [];
    },
    async backendCapabilities(backend) {
      return createCapabilities(backend, overrides.capabilityOverrides ?? {});
    },
    async createNativeSession(input) {
      gateway.createNativeCalls.push(input);
      if (overrides.createNativeError) {
        throw overrides.createNativeError;
      }
      return {
        session_id: "session-created",
        origin: {
          backend: "native",
          authority: "local_daemon",
          foreignReferenceLabel: null,
        },
        title: input?.title ?? "Created",
        degradedSemantics: [],
      };
    },
    async importSession(input) {
      gateway.importCalls.push(input);
      return {
        session_id: "session-imported",
        origin: {
          backend: "tmux",
          authority: "imported_foreign",
          foreignReferenceLabel: "tmux",
        },
        title: input.title ?? "Imported",
        degradedSemantics: [],
      };
    },
    async restoreSavedSession() {
      return sessions[0];
    },
    async deleteSavedSession(sessionId) {
      return { sessionId };
    },
    async dispatchMuxCommand(sessionId, command) {
      gateway.dispatchCalls.push({ sessionId, command });
      return { changed: true };
    },
    async subscribeSessionState(sessionId, handlers) {
      gateway.subscribeCalls.push(sessionId);
      gateway.subscriptionHandlers = handlers;
      handlers.onState(sessionState);
      if (overrides.subscribeError) {
        throw overrides.subscribeError;
      }
      return {
        subscriptionId: `sub-${sessionId}`,
        async dispose() {},
      };
    },
    dispose() {},
  };

  return gateway;
}

test("bootstrap loads catalog and attaches first session state", async () => {
  const store = createStore();
  const gateway = createGateway();
  const controller = new TerminalWorkspaceController(gateway, gateway, store);

  await controller.bootstrap();

  const state = store.getState();
  assert.equal(state.status, "ready");
  assert.equal(state.sessionStatus, "ready");
  assert.equal(state.activeSessionId, "session-1");
  assert.equal(state.handshake?.handshake.daemon_phase, "ready");
  assert.equal(state.capabilities.native?.backend, "native");
  assert.equal(state.discoveredSessions.tmux?.length, 1);
  assert.equal(state.activeSessionState?.focusedScreen?.pane_id, "pane-1");
  assert.deepEqual(gateway.subscribeCalls, ["session-1"]);
});

test("createNativeSession uses parsed contract payload and surfaces action errors without throwing", async () => {
  const store = createStore({
    createTitleDraft: "  Workspace  ",
    createProgramDraft: " /bin/zsh ",
    createArgsDraft: ' -l "-c pwd" ',
    createCwdDraft: " /tmp/demo ",
  });
  const gateway = createGateway({
    createNativeError: new Error("native create failed"),
  });
  const controller = new TerminalWorkspaceController(gateway, gateway, store);

  await controller.createNativeSession();

  assert.deepEqual(gateway.createNativeCalls, [
    {
      title: "Workspace",
      launch: {
        program: "/bin/zsh",
        args: ["-l", "-c pwd"],
        cwd: "/tmp/demo",
      },
    },
  ]);
  assert.equal(store.getState().actionError, "native create failed");
});

test("importSession sends opaque import handle instead of foreign route", async () => {
  const store = createStore();
  const gateway = createGateway();
  const controller = new TerminalWorkspaceController(gateway, gateway, store);

  await controller.importSession({
    importHandle: "import-tmux-1",
    title: "Foreign",
  });

  assert.deepEqual(gateway.importCalls, [
    {
      importHandle: "import-tmux-1",
      title: "Foreign",
    },
  ]);
});

test("saveSession blocks explicitly when capability model says save is unsupported", async () => {
  const store = createStore({
    sessions: [
      {
        session_id: "session-1",
        origin: {
          backend: "tmux",
          authority: "imported_foreign",
          foreignReferenceLabel: "tmux",
        },
        title: "Tmux Session",
        degradedSemantics: [],
      },
    ],
    activeSessionId: "session-1",
    capabilities: {
      tmux: createCapabilities("tmux", {
        explicit_session_save: false,
      }),
    },
  });
  const gateway = createGateway();
  const controller = new TerminalWorkspaceController(gateway, gateway, store);

  await controller.saveSession();

  assert.equal(gateway.dispatchCalls.length, 0);
  assert.equal(store.getState().actionError, null);
  assert.equal(store.getState().actionDegradedReason?.code, "action_save_session_unsupported");
});

test("restoreSavedSession blocks explicitly when saved session is incompatible", async () => {
  const store = createStore({
    savedSessions: [
      {
        session_id: "saved-1",
        origin: {
          backend: "tmux",
          authority: "imported_foreign",
          foreignReferenceLabel: "tmux",
        },
        title: "Saved Foreign",
        saved_at_ms: 1700000000000,
        manifest: {
          format_version: 1,
          binary_version: "1.0.0",
          protocol_major: 1,
          protocol_minor: 0,
        },
        compatibility: {
          can_restore: false,
          status: "protocol_major_unsupported",
        },
        has_launch: true,
        tab_count: 1,
        pane_count: 1,
        restore_semantics: {
          restores_topology: true,
          restores_focus_state: true,
          restores_tab_titles: true,
          uses_saved_launch_spec: true,
          replays_saved_screen_buffers: false,
          preserves_process_state: false,
        },
        degradedSemantics: [
          {
            code: "saved_session_restore_unavailable",
            scope: "saved_session",
            severity: "error",
            summary: "Saved session cannot be restored safely",
            detail: "Restore compatibility is protocol_major_unsupported.",
          },
        ],
      },
    ],
  });
  const gateway = createGateway();
  const controller = new TerminalWorkspaceController(gateway, gateway, store);

  await controller.restoreSavedSession("saved-1");

  assert.equal(store.getState().actionDegradedReason?.code, "saved_session_restore_unavailable");
});

test("selectSession degrades to session error when subscription start fails", async () => {
  const store = createStore();
  const gateway = createGateway({
    subscribeError: new Error("subscription start failed"),
  });
  const controller = new TerminalWorkspaceController(gateway, gateway, store);

  await controller.selectSession("session-1");

  assert.equal(store.getState().sessionStatus, "error");
  assert.equal(store.getState().actionError, null);
  assert.equal(store.getState().sessionStreamHealth.phase, "error");
  assert.equal(store.getState().sessionStreamHealth.lastError, "subscription start failed");
});

test("session stream reconnecting state is explicit while keeping the last snapshot", async () => {
  const store = createStore();
  const gateway = createGateway();
  const controller = new TerminalWorkspaceController(gateway, gateway, store);

  await controller.selectSession("session-1");

  gateway.subscriptionHandlers.onStatusChange({
    phase: "reconnecting",
    reconnectAttempts: 2,
    lastError: "Terminal session stream connection closed",
  });

  const state = store.getState();
  assert.equal(state.sessionStatus, "ready");
  assert.equal(state.activeSessionState?.focusedScreen?.sequence, "1");
  assert.equal(state.sessionStreamHealth.phase, "reconnecting");
  assert.equal(state.sessionStreamHealth.reconnectAttempts, 2);
  assert.equal(state.sessionStreamHealth.lastError, "Terminal session stream connection closed");
});
