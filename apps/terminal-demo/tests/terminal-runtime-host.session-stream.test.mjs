import test from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:net";
import { setTimeout as delay } from "node:timers/promises";
import WebSocket, { WebSocketServer } from "ws";

import { WebSocketTerminalRuntimeSessionStateStream } from "../dist/features/terminal-runtime-host/renderer/adapters/WebSocketTerminalRuntimeSessionStateStream.js";

globalThis.WebSocket ??= WebSocket;

const canBindLoopback = await probeLoopbackTcp();
const loopbackTestOptions = canBindLoopback
  ? undefined
  : { skip: "loopback TCP bind is unavailable in this environment" };

function createSessionState(sequence) {
  return {
    session: {
      session_id: "session-1",
      origin: {
        backend: "native",
        authority: "local_daemon",
        foreignReferenceLabel: null,
      },
      title: "Workspace",
      degradedSemantics: [],
    },
    topology: {
      session_id: "session-1",
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
      sequence,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      surface: {
        title: null,
        cursor: null,
        lines: [
          { text: `state-${sequence}` },
        ],
      },
    },
  };
}

async function waitUntil(predicate, timeoutMs = 2_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const result = predicate();
    if (result) {
      return result;
    }
    await delay(20);
  }

  throw new Error("Timed out waiting for condition");
}

test("session stream reconnects and resubscribes after transient socket loss", loopbackTestOptions, async () => {
  const server = new WebSocketServer({ host: "127.0.0.1", port: 0 });
  await once(server, "listening");

  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("Failed to bind session stream test server");
  }

  const url = `ws://127.0.0.1:${address.port}/terminal-gateway/stream`;
  const adapter = new WebSocketTerminalRuntimeSessionStateStream(url);
  const subscribeCalls = [];
  const unsubscribeCalls = [];
  const receivedStates = [];
  const receivedErrors = [];
  let closedCount = 0;
  let connectionCount = 0;

  server.on("connection", (socket) => {
    connectionCount += 1;
    const currentConnection = connectionCount;

    socket.on("message", (payload) => {
      const message = JSON.parse(payload.toString());
      if (message.type === "stream_subscribe_session_state") {
        subscribeCalls.push({
          connection: currentConnection,
          subscriptionId: message.subscriptionId,
          sessionId: message.sessionId,
        });

        socket.send(JSON.stringify({
          type: "stream_subscription_ack",
          subscriptionId: message.subscriptionId,
          sessionId: message.sessionId,
        }));
        socket.send(JSON.stringify({
          type: "session_state",
          subscriptionId: message.subscriptionId,
          sessionId: message.sessionId,
          state: createSessionState(String(currentConnection)),
        }));

        if (currentConnection === 1) {
          setTimeout(() => {
            socket.close();
          }, 10);
        }
        return;
      }

      if (message.type === "stream_unsubscribe_session_state") {
        unsubscribeCalls.push({
          connection: currentConnection,
          subscriptionId: message.subscriptionId,
          sessionId: message.sessionId,
        });
        socket.send(JSON.stringify({
          type: "subscription_closed",
          subscriptionId: message.subscriptionId,
          sessionId: message.sessionId,
        }));
      }
    });
  });

  try {
    const subscription = await adapter.subscribeSessionState("session-1", {
      onState: (state) => {
        receivedStates.push(state.focusedScreen?.sequence ?? "none");
      },
      onError: (error) => {
        receivedErrors.push(error.message);
      },
      onClosed: () => {
        closedCount += 1;
      },
    });

    await waitUntil(() => receivedStates.length >= 2 && connectionCount >= 2);

    assert.deepEqual(receivedStates.slice(0, 2), ["1", "2"]);
    assert.equal(receivedErrors.length, 0);
    assert.equal(subscribeCalls.length, 2);
    assert.equal(subscribeCalls[0].subscriptionId, subscribeCalls[1].subscriptionId);
    assert.equal(subscribeCalls[0].sessionId, "session-1");
    assert.equal(subscribeCalls[1].sessionId, "session-1");

    await subscription.dispose();
    await waitUntil(() => unsubscribeCalls.length === 1 && closedCount === 1);

    assert.equal(unsubscribeCalls[0].connection, 2);
    assert.equal(unsubscribeCalls[0].subscriptionId, subscribeCalls[0].subscriptionId);
  } finally {
    adapter.dispose();
    await new Promise((resolve) => {
      server.close(() => resolve(undefined));
    });
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
