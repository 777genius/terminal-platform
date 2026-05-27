import test from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:net";
import WebSocket from "ws";

import { TerminalRuntimeGatewayServer } from "../dist/features/terminal-runtime-host/main/adapters/input/TerminalRuntimeGatewayServer.js";
import {
  TerminalRuntimeControlService,
  TerminalRuntimeSessionStreamService,
} from "../dist/features/terminal-runtime-host/core/application/index.js";

const canBindLoopback = await probeLoopbackTcp();
const loopbackTestOptions = canBindLoopback
  ? undefined
  : { skip: "loopback TCP bind is unavailable in this environment" };

function createControlClient(url) {
  const socket = new WebSocket(url);
  const pending = new Map();
  const events = [];
  let nextId = 0;

  socket.on("message", (payload) => {
    const message = JSON.parse(payload.toString());
    if (message.type === "response") {
      const resolve = pending.get(message.requestId);
      if (resolve) {
        pending.delete(message.requestId);
        resolve(message);
      }
      return;
    }

    events.push(message);
  });

  return {
    socket,
    events,
    async connect() {
      await once(socket, "open");
    },
    async request(method, payload) {
      const requestId = `req-${++nextId}`;
      const response = await new Promise((resolve) => {
        pending.set(requestId, resolve);
        socket.send(JSON.stringify({
          type: "request",
          requestId,
          method,
          payload,
        }));
      });

      if (!response.ok) {
        throw new Error(response.error.message);
      }

      return response.result;
    },
    async close() {
      socket.close();
      await once(socket, "close");
    },
  };
}

function createStreamClient(url) {
  const socket = new WebSocket(url);
  const events = [];

  socket.on("message", (payload) => {
    events.push(JSON.parse(payload.toString()));
  });

  return {
    socket,
    events,
    async connect() {
      await once(socket, "open");
    },
    send(message) {
      socket.send(JSON.stringify(message));
    },
    async waitForEvent(predicate, timeoutMs = 1000) {
      const deadline = Date.now() + timeoutMs;
      while (Date.now() < deadline) {
        const match = events.find(predicate);
        if (match) {
          return match;
        }
        await new Promise((resolve) => setTimeout(resolve, 10));
      }
      throw new Error("Timed out waiting for stream event");
    },
    async close() {
      socket.close();
      await once(socket, "close");
    },
  };
}

function createRuntime(overrides = {}) {
  const importCalls = [];

  return {
    importCalls,
    handshakeInfo: async () => ({
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
        available_backends: ["tmux", "native"],
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
    }),
    listSessions: async () => overrides.listSessions ?? [],
    listSavedSessions: async () => [],
    discoverSessions: async (backend) => overrides.discoverSessions?.(backend) ?? [
      {
        route: {
          backend: "tmux",
          authority: "imported_foreign",
          external: {
            namespace: "tmux",
            value: "$3",
          },
        },
        title: "Foreign Session",
      },
    ],
    backendCapabilities: async (backend) => ({
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
      },
      degradedSemantics: [],
    }),
    createNativeSession: async () => {
      throw new Error("not used");
    },
    importSession: async (input) => {
      importCalls.push(input);
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
    restoreSavedSession: async () => {
      throw new Error("not used");
    },
    deleteSavedSession: async () => {
      throw new Error("not used");
    },
    dispatchMuxCommand: async () => ({ changed: true }),
    watchSessionState: async (sessionId, handlers) => {
      if (overrides.watchSessionState) {
        return overrides.watchSessionState(sessionId, handlers);
      }

      queueMicrotask(() => {
        handlers.onState({
          session: {
            session_id: sessionId,
            origin: {
              backend: "native",
              authority: "local_daemon",
              foreignReferenceLabel: null,
            },
            title: "Workspace",
            degradedSemantics: [],
          },
          topology: {
            session_id: sessionId,
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
          focusedScreen: null,
        });
      });

      return {
        sessionId,
        async dispose() {
          handlers.onClosed();
        },
      };
    },
  };
}

test("gateway exposes opaque import handles instead of foreign backend routes", loopbackTestOptions, async () => {
  const runtime = createRuntime();
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(runtime),
    sessionStreamService: new TerminalRuntimeSessionStreamService(runtime),
  });

  const client = createControlClient(gateway.controlPlaneUrl);
  try {
    await client.connect();

    const discovered = await client.request("discover_sessions", { backend: "tmux" });
    assert.equal(discovered.length, 1);
    assert.equal(typeof discovered[0].importHandle, "string");
    assert.equal(discovered[0].backend, "tmux");
    assert.equal(discovered[0].sourceLabel, "tmux");
    assert.equal("route" in discovered[0], false);

    const imported = await client.request("import_session", {
      importHandle: discovered[0].importHandle,
      title: "Imported Title",
    });

    assert.equal(imported.session_id, "session-imported");
    assert.deepEqual(runtime.importCalls, [
      {
        route: {
          backend: "tmux",
          authority: "imported_foreign",
          external: {
            namespace: "tmux",
            value: "$3",
          },
        },
        title: "Imported Title",
      },
    ]);
  } finally {
    await client.close();
    await gateway.dispose();
  }
});

test("gateway exposes raw workspace handshake for SDK clients", loopbackTestOptions, async () => {
  const sdkClient = createSdkClient();
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
  });

  const client = createControlClient(gateway.controlPlaneUrl);
  try {
    await client.connect();

    const handshake = await client.request("workspace_handshake", undefined);
    const capabilities = await client.request("workspace_backend_capabilities", { backend: "native" });

    assert.equal("assessment" in handshake, false);
    assert.deepEqual(handshake.available_backends, ["native", "tmux"]);
    assert.equal(capabilities.backend, "native");
    assert.equal(capabilities.capabilities.pane_split, true);
  } finally {
    await client.close();
    await gateway.dispose();
  }
});

test("gateway fault injection fails one workspace pane history request without calling SDK client", loopbackTestOptions, async () => {
  const paneHistoryCalls = [];
  const sdkClient = {
    ...createSdkClient(),
    paneHistory: async (...args) => {
      paneHistoryCalls.push(args);
      return {
        session_id: args[0],
        pane_id: args[1],
        from_event_seq: BigInt(args[2] ?? 1),
        next_event_seq: null,
        has_more_segments: false,
        total_payload_bytes: 0n,
        replay_strategy: "raw_vt_stream",
        restore_plan: {
          restore_guarantee_level: "raw_vt_replay",
        },
        segments: [],
        gaps: [],
        latest_screen_snapshot: null,
      };
    },
  };
  let injectedFailures = 0;
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
    faultInjection: {
      beforeWorkspacePaneHistory: (request) => {
        assert.equal(request.sessionId, "saved-session-1");
        assert.equal(request.paneId, "pane-1");
        if (injectedFailures === 0) {
          injectedFailures += 1;
          throw new Error("Simulated workspace pane history failure for degraded persistence smoke");
        }
      },
    },
  });

  const client = createControlClient(gateway.controlPlaneUrl);
  try {
    await client.connect();

    await assert.rejects(
      () => client.request("workspace_pane_history", {
        sessionId: "saved-session-1",
        paneId: "pane-1",
        fromEventSeq: 1,
        maxSegments: 256,
        maxBytes: 1_048_576,
      }),
      /Simulated workspace pane history failure/,
    );
    assert.equal(paneHistoryCalls.length, 0);

    const history = await client.request("workspace_pane_history", {
      sessionId: "saved-session-1",
      paneId: "pane-1",
      fromEventSeq: 1,
      maxSegments: 256,
      maxBytes: 1_048_576,
    });
    assert.equal(history.session_id, "saved-session-1");
    assert.equal(history.pane_id, "pane-1");
    assert.equal(paneHistoryCalls.length, 1);
  } finally {
    await client.close();
    await gateway.dispose();
  }
});

test("gateway fault injection fails one workspace dispatch request without calling SDK client", loopbackTestOptions, async () => {
  const dispatchCalls = [];
  const sdkClient = {
    ...createSdkClient(),
    dispatchMuxCommand: async (...args) => {
      dispatchCalls.push(args);
      return { changed: true };
    },
  };
  let injectedFailures = 0;
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
    faultInjection: {
      beforeWorkspaceDispatchMuxCommand: (request) => {
        assert.equal(request.sessionId, "session-1");
        assert.equal(request.command.kind, "send_input");
        if (injectedFailures === 0) {
          injectedFailures += 1;
          const error = new Error("Simulated storage pressure");
          error.code = "storage_pressure";
          throw error;
        }
      },
    },
  });

  const client = createControlClient(gateway.controlPlaneUrl);
  const command = {
    kind: "send_input",
    pane_id: "pane-1",
    data: "echo storage-pressure\r",
  };
  try {
    await client.connect();

    await assert.rejects(
      () => client.request("workspace_dispatch_mux_command", {
        sessionId: "session-1",
        command,
      }),
      /Simulated storage pressure/,
    );
    assert.equal(dispatchCalls.length, 0);

    const result = await client.request("workspace_dispatch_mux_command", {
      sessionId: "session-1",
      command,
    });
    assert.equal(result.changed, true);
    assert.equal(dispatchCalls.length, 1);
  } finally {
    await client.close();
    await gateway.dispose();
  }
});

test("gateway rejects malformed control payloads before application ports", loopbackTestOptions, async () => {
  let discoverCalls = 0;
  const runtime = createRuntime({
    discoverSessions: async () => {
      discoverCalls += 1;
      return [];
    },
  });
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(runtime),
    sessionStreamService: new TerminalRuntimeSessionStreamService(runtime),
  });

  const client = createControlClient(gateway.controlPlaneUrl);
  try {
    await client.connect();

    await assert.rejects(
      () => client.request("discover_sessions", { backend: "screen" }),
      /Gateway payload backend must be one of: native, tmux, zellij/,
    );
    await assert.rejects(
      () => client.request("restore_saved_session", { sessionId: 42 }),
      /Gateway payload sessionId must be a non-empty string/,
    );
    await assert.rejects(
      () => client.request("dispatch_mux_command", {
        sessionId: "session-1",
        command: { kind: "shell_escape" },
      }),
      /Gateway payload command.kind is unsupported/,
    );
    await assert.rejects(
      () => client.request("dispatch_mux_command", {
        sessionId: "session-1",
        command: {
          kind: "send_input",
          pane_id: "pane-1",
        },
      }),
      /Gateway payload data must be a non-empty string/,
    );
    await assert.rejects(
      () => client.request("dispatch_mux_command", {
        sessionId: "session-1",
        command: {
          kind: "split_pane",
          pane_id: "pane-1",
          direction: "diagonal",
        },
      }),
      /Gateway payload command.direction is unsupported/,
    );
    await assert.rejects(
      () => client.request("dispatch_mux_command", {
        sessionId: "session-1",
        command: {
          kind: "resize_pane",
          pane_id: "pane-1",
          rows: 24.5,
          cols: 80,
        },
      }),
      /Gateway payload rows must be an integer/,
    );

    assert.equal(discoverCalls, 0);
  } finally {
    await client.close();
    await gateway.dispose();
  }
});

test("gateway keeps session state traffic on the stream plane only", loopbackTestOptions, async () => {
  const runtime = createRuntime({
    listSessions: [
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
    ],
  });
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(runtime),
    sessionStreamService: new TerminalRuntimeSessionStreamService(runtime),
  });

  const controlClient = createControlClient(gateway.controlPlaneUrl);
  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  try {
    await Promise.all([controlClient.connect(), streamClient.connect()]);

    const sessions = await controlClient.request("list_sessions", undefined);
    assert.equal(sessions.length, 1);

    streamClient.send({
      type: "stream_subscribe_session_state",
      subscriptionId: "sub-1",
      sessionId: "session-1",
    });

    const ack = await streamClient.waitForEvent((event) => event.type === "stream_subscription_ack");
    assert.equal(ack.subscriptionId, "sub-1");

    const stateEvent = await streamClient.waitForEvent((event) => event.type === "session_state");
    assert.equal(stateEvent.sessionId, "session-1");
    assert.equal(controlClient.events.some((event) => event.type === "session_state"), false);

    streamClient.send({
      type: "stream_unsubscribe_session_state",
      subscriptionId: "sub-1",
      sessionId: "session-1",
    });
    const closed = await streamClient.waitForEvent((event) => event.type === "subscription_closed");
    assert.equal(closed.subscriptionId, "sub-1");
  } finally {
    await Promise.all([
      controlClient.close(),
      streamClient.close(),
      gateway.dispose(),
    ]);
  }
});

test("gateway bridges workspace subscriptions over the stream plane for SDK clients", loopbackTestOptions, async () => {
  const subscriptionEvent = {
    kind: "screen_delta",
    pane_id: "pane-1",
    from_sequence: 0,
    to_sequence: 1,
    rows: 24,
    cols: 80,
    source: "native",
    full_replace: {
      lines: [{ text: "ready" }],
      cursor: null,
      title: null,
    },
    patch: null,
  };
  const sdkClient = {
    ...createSdkClient(),
    openCalls: [],
    openSubscription: async (sessionId, spec) => {
      const subscription = createDeferredWorkspaceSubscription("native-sub-1", subscriptionEvent);
      sdkClient.openCalls.push({ sessionId, spec, subscription });
      return subscription;
    },
  };
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  try {
    await streamClient.connect();

    streamClient.send({
      type: "workspace_subscribe",
      subscriptionId: "workspace-sub-1",
      sessionId: "session-1",
      spec: {
        kind: "pane_surface",
        pane_id: "pane-1",
      },
    });

    const ack = await streamClient.waitForEvent((event) => event.type === "workspace_subscription_ack");
    assert.equal(ack.subscriptionId, "workspace-sub-1");
    assert.deepEqual(ack.meta, { subscription_id: "native-sub-1" });
    assert.equal(sdkClient.openCalls.length, 1);
    assert.deepEqual(sdkClient.openCalls[0].spec, { kind: "pane_surface", pane_id: "pane-1" });

    const eventMessage = await streamClient.waitForEvent((event) => event.type === "workspace_subscription_event");
    assert.equal(eventMessage.subscriptionId, "workspace-sub-1");
    assert.deepEqual(eventMessage.event, subscriptionEvent);

    streamClient.send({
      type: "workspace_unsubscribe",
      subscriptionId: "workspace-sub-1",
    });
    const closed = await streamClient.waitForEvent((event) => event.type === "workspace_subscription_closed");
    assert.equal(closed.subscriptionId, "workspace-sub-1");
    assert.equal(sdkClient.openCalls[0].subscription.closeCalls, 1);
  } finally {
    await Promise.all([
      streamClient.close(),
      gateway.dispose(),
    ]);
  }
});

test("gateway settles workspace unsubscribe when SDK subscription close hangs", loopbackTestOptions, async () => {
  const subscription = createHangingWorkspaceSubscription("native-sub-hanging");
  const sdkClient = {
    ...createSdkClient(),
    openSubscription: async () => subscription,
  };
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  try {
    await streamClient.connect();

    streamClient.send({
      type: "workspace_subscribe",
      subscriptionId: "workspace-sub-hanging",
      sessionId: "session-1",
      spec: {
        kind: "session_topology",
      },
    });

    const ack = await streamClient.waitForEvent((event) => event.type === "workspace_subscription_ack");
    assert.equal(ack.subscriptionId, "workspace-sub-hanging");

    streamClient.send({
      type: "workspace_unsubscribe",
      subscriptionId: "workspace-sub-hanging",
    });
    const closed = await streamClient.waitForEvent((event) => event.type === "workspace_subscription_closed");
    assert.equal(closed.subscriptionId, "workspace-sub-hanging");
    assert.equal(subscription.closeCalls, 1);
  } finally {
    await Promise.all([
      streamClient.close(),
      gateway.dispose(),
    ]);
  }
});

test("gateway settles legacy session unsubscribe when runtime dispose hangs", loopbackTestOptions, async () => {
  const handle = createHangingLegacySessionStateHandle();
  const runtime = createRuntime({
    watchSessionState: async () => handle,
  });
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(runtime),
    sessionStreamService: new TerminalRuntimeSessionStreamService(runtime),
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  try {
    await streamClient.connect();

    streamClient.send({
      type: "stream_subscribe_session_state",
      subscriptionId: "legacy-sub-hanging-unsubscribe",
      sessionId: "session-1",
    });

    const ack = await streamClient.waitForEvent((event) => event.type === "stream_subscription_ack");
    assert.equal(ack.subscriptionId, "legacy-sub-hanging-unsubscribe");

    streamClient.send({
      type: "stream_unsubscribe_session_state",
      subscriptionId: "legacy-sub-hanging-unsubscribe",
      sessionId: "session-1",
    });

    const closed = await streamClient.waitForEvent((event) => event.type === "subscription_closed");
    assert.equal(closed.subscriptionId, "legacy-sub-hanging-unsubscribe");
    assert.equal(handle.disposeCalls, 1);
  } finally {
    await Promise.all([
      closeSocketIfOpen(streamClient.socket),
      gateway.dispose(),
    ]);
  }
});

test("gateway closes late legacy session handles after unsubscribe races", loopbackTestOptions, async () => {
  const handle = createHangingLegacySessionStateHandle();
  const watchStarted = createDeferred();
  const releaseHandle = createDeferred();
  const runtime = createRuntime({
    watchSessionState: async () => {
      watchStarted.resolve();
      await releaseHandle.promise;
      return handle;
    },
  });
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(runtime),
    sessionStreamService: new TerminalRuntimeSessionStreamService(runtime),
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  try {
    await streamClient.connect();

    streamClient.send({
      type: "stream_subscribe_session_state",
      subscriptionId: "legacy-sub-late",
      sessionId: "session-1",
    });
    await watchStarted.promise;

    streamClient.send({
      type: "stream_unsubscribe_session_state",
      subscriptionId: "legacy-sub-late",
      sessionId: "session-1",
    });
    const closed = await streamClient.waitForEvent((event) => event.type === "subscription_closed");
    assert.equal(closed.subscriptionId, "legacy-sub-late");

    releaseHandle.resolve();
    await waitForCondition(() => handle.disposeCalls === 1);
    assert.equal(handle.disposeCalls, 1);
  } finally {
    await Promise.all([
      closeSocketIfOpen(streamClient.socket),
      gateway.dispose(),
    ]);
  }
});

test("gateway closes late workspace subscriptions after unsubscribe races", loopbackTestOptions, async () => {
  const subscription = createHangingWorkspaceSubscription("native-sub-late");
  const openStarted = createDeferred();
  const releaseSubscription = createDeferred();
  const sdkClient = {
    ...createSdkClient(),
    openSubscription: async () => {
      openStarted.resolve();
      await releaseSubscription.promise;
      return subscription;
    },
  };
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  try {
    await streamClient.connect();

    streamClient.send({
      type: "workspace_subscribe",
      subscriptionId: "workspace-sub-late",
      sessionId: "session-1",
      spec: {
        kind: "session_topology",
      },
    });
    await openStarted.promise;

    streamClient.send({
      type: "workspace_unsubscribe",
      subscriptionId: "workspace-sub-late",
    });
    const closed = await streamClient.waitForEvent((event) => event.type === "workspace_subscription_closed");
    assert.equal(closed.subscriptionId, "workspace-sub-late");

    releaseSubscription.resolve();
    await waitForCondition(() => subscription.closeCalls === 1);
    assert.equal(subscription.closeCalls, 1);
  } finally {
    await Promise.all([
      closeSocketIfOpen(streamClient.socket),
      gateway.dispose(),
    ]);
  }
});

test("gateway dispose settles legacy session subscriptions when runtime dispose hangs", loopbackTestOptions, async () => {
  const handle = createHangingLegacySessionStateHandle();
  const runtime = createRuntime({
    watchSessionState: async (sessionId, handlers) => {
      queueMicrotask(() => {
        handlers.onState({
          session: {
            session_id: sessionId,
            origin: {
              backend: "native",
              authority: "local_daemon",
              foreignReferenceLabel: null,
            },
            title: "Workspace",
            degradedSemantics: [],
          },
          topology: {
            session_id: sessionId,
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
          focusedScreen: null,
        });
      });
      return handle;
    },
  });
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(runtime),
    sessionStreamService: new TerminalRuntimeSessionStreamService(runtime),
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  let disposed = false;
  try {
    await streamClient.connect();

    streamClient.send({
      type: "stream_subscribe_session_state",
      subscriptionId: "legacy-sub-hanging",
      sessionId: "session-1",
    });

    const ack = await streamClient.waitForEvent((event) => event.type === "stream_subscription_ack");
    assert.equal(ack.subscriptionId, "legacy-sub-hanging");

    const result = await Promise.race([
      gateway.dispose().then(() => "disposed"),
      new Promise((resolve) => setTimeout(() => resolve("timeout"), 1000)),
    ]);
    disposed = result === "disposed";
    assert.equal(result, "disposed");
    assert.equal(handle.disposeCalls, 1);
  } finally {
    await closeSocketIfOpen(streamClient.socket);
    if (!disposed) {
      void gateway.dispose().catch(() => undefined);
    }
  }
});

test("gateway dispose settles workspace subscriptions when SDK close hangs", loopbackTestOptions, async () => {
  const subscription = createHangingWorkspaceSubscription("native-sub-dispose-hanging");
  const sdkClient = {
    ...createSdkClient(),
    openSubscription: async () => subscription,
  };
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  let disposed = false;
  try {
    await streamClient.connect();

    streamClient.send({
      type: "workspace_subscribe",
      subscriptionId: "workspace-sub-dispose-hanging",
      sessionId: "session-1",
      spec: {
        kind: "session_topology",
      },
    });

    const ack = await streamClient.waitForEvent((event) => event.type === "workspace_subscription_ack");
    assert.equal(ack.subscriptionId, "workspace-sub-dispose-hanging");

    const result = await Promise.race([
      gateway.dispose().then(() => "disposed"),
      new Promise((resolve) => setTimeout(() => resolve("timeout"), 1000)),
    ]);
    disposed = result === "disposed";
    assert.equal(result, "disposed");
    assert.equal(subscription.closeCalls, 1);
  } finally {
    await closeSocketIfOpen(streamClient.socket);
    if (!disposed) {
      void gateway.dispose().catch(() => undefined);
    }
  }
});

test("gateway closes stream transport instead of throwing when server send fails", loopbackTestOptions, async () => {
  const originalSend = WebSocket.prototype.send;
  WebSocket.prototype.send = function patchedSend(data, ...args) {
    if (String(data).includes("\"type\":\"workspace_subscription_ack\"")) {
      throw new Error("Simulated server-side websocket send failure");
    }

    return originalSend.call(this, data, ...args);
  };

  const subscription = createDeferredWorkspaceSubscription("native-sub-1", null);
  const sdkClient = {
    ...createSdkClient(),
    openSubscription: async () => subscription,
  };
  const gateway = await TerminalRuntimeGatewayServer.start({
    runtimeSlug: "terminal-demo",
    controlService: new TerminalRuntimeControlService(createRuntime()),
    sessionStreamService: new TerminalRuntimeSessionStreamService(createRuntime()),
    clientProvider: {
      getClient: async () => sdkClient,
    },
  });

  const streamClient = createStreamClient(gateway.sessionStreamUrl);
  try {
    await streamClient.connect();

    const closed = once(streamClient.socket, "close");
    streamClient.send({
      type: "workspace_subscribe",
      subscriptionId: "workspace-sub-1",
      sessionId: "session-1",
      spec: {
        kind: "pane_surface",
        pane_id: "pane-1",
      },
    });

    await closed;
    await waitForCondition(() => subscription.closeCalls === 1);
    assert.equal(subscription.closeCalls, 1);
  } finally {
    WebSocket.prototype.send = originalSend;
    await closeSocketIfOpen(streamClient.socket);
    await gateway.dispose();
  }
});

function createSdkClient() {
  return {
    handshakeInfo: async () => ({
      handshake: {
        protocol_version: { major: 0, minor: 2 },
        binary_version: "0.1.0-test",
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
    }),
    backendCapabilities: async (backend) => ({
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
    }),
  };
}

function createDeferredWorkspaceSubscription(subscriptionId, event) {
  let delivered = false;
  let releaseCloseWait;
  const closeWait = new Promise((resolve) => {
    releaseCloseWait = resolve;
  });

  return {
    subscriptionId,
    closeCalls: 0,
    async nextEvent() {
      if (!delivered) {
        delivered = true;
        return event;
      }

      await closeWait;
      return null;
    },
    async close() {
      this.closeCalls += 1;
      releaseCloseWait();
    },
  };
}

function createHangingWorkspaceSubscription(subscriptionId) {
  return {
    subscriptionId,
    closeCalls: 0,
    async nextEvent() {
      await new Promise(() => {});
      return null;
    },
    async close() {
      this.closeCalls += 1;
      await new Promise(() => {});
    },
  };
}

function createHangingLegacySessionStateHandle() {
  return {
    disposeCalls: 0,
    async dispose() {
      this.disposeCalls += 1;
      await new Promise(() => {});
    },
  };
}

function createDeferred() {
  let resolve;
  const promise = new Promise((innerResolve) => {
    resolve = innerResolve;
  });

  return {
    promise,
    resolve,
  };
}

async function waitForCondition(predicate, timeoutMs = 1000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) {
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 10));
  }

  throw new Error("Timed out waiting for condition");
}

async function closeSocketIfOpen(socket) {
  if (socket.readyState === WebSocket.CLOSED) {
    return;
  }

  const closed = once(socket, "close").catch(() => undefined);
  if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
    socket.close();
  }

  await Promise.race([
    closed,
    new Promise((resolve) => setTimeout(resolve, 500)),
  ]);
}

async function probeLoopbackTcp() {
  const server = createServer();
  return new Promise((resolve) => {
    const cleanup = () => {
      server.off("error", onError);
      server.off("listening", onListening);
    };
    const onError = () => {
      cleanup();
      resolve(false);
    };
    const onListening = () => {
      cleanup();
      server.close(() => resolve(true));
    };

    server.once("error", onError);
    server.once("listening", onListening);
    server.listen(0, "127.0.0.1");
  });
}
