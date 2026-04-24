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
    discoverSessions: async () => [
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
