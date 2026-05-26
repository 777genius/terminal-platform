import test from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:net";
import { setTimeout as delay } from "node:timers/promises";
import WebSocket, { WebSocketServer } from "ws";

import { WebSocketTerminalRuntimeControlPlane } from "../dist/features/terminal-runtime-host/renderer/adapters/WebSocketTerminalRuntimeControlPlane.js";

globalThis.WebSocket ??= WebSocket;

const canBindLoopback = await probeLoopbackTcp();
const loopbackTestOptions = canBindLoopback
  ? undefined
  : { skip: "loopback TCP bind is unavailable in this environment" };

test("control plane retries startup connection races before sending requests", loopbackTestOptions, async () => {
  const port = await reserveLoopbackPort();
  const adapter = new WebSocketTerminalRuntimeControlPlane(`ws://127.0.0.1:${port}/terminal-gateway/control`);
  let server = null;
  let connectionCount = 0;

  try {
    const request = adapter.handshakeInfo();
    await delay(75);

    server = new WebSocketServer({ host: "127.0.0.1", port });
    server.on("connection", (socket) => {
      connectionCount += 1;
      socket.on("message", (payload) => {
        const message = JSON.parse(payload.toString());
        socket.send(JSON.stringify({
          type: "response",
          requestId: message.requestId,
          ok: true,
          result: createHandshakeInfo(),
        }));
      });
    });
    await once(server, "listening");

    const handshake = await request;
    assert.equal(handshake.handshake.session_scope, "terminal-demo");
    assert.equal(connectionCount, 1);
  } finally {
    adapter.dispose();
    if (server) {
      await new Promise((resolve) => {
        server.close(() => resolve(undefined));
      });
    }
  }
});

test("control plane dispose releases pending startup retry waits", loopbackTestOptions, async () => {
  const port = await reserveLoopbackPort();
  const adapter = new WebSocketTerminalRuntimeControlPlane(`ws://127.0.0.1:${port}/terminal-gateway/control`);
  const request = adapter.handshakeInfo();

  await delay(25);
  adapter.dispose();

  await assert.rejects(
    Promise.race([
      request,
      delay(500).then(() => {
        throw new Error("control plane request did not settle after dispose");
      }),
    ]),
  );
});

test("control plane dispose ignores websocket close failures", async () => {
  const originalWebSocket = globalThis.WebSocket;
  globalThis.WebSocket = createOpenThrowingCloseWebSocket();
  const adapter = new WebSocketTerminalRuntimeControlPlane("ws://127.0.0.1/terminal-gateway/control");
  const request = adapter.handshakeInfo();

  try {
    await delay(0);

    assert.doesNotThrow(() => {
      adapter.dispose();
    });
    await assert.rejects(request, /Terminal control plane disposed/);
  } finally {
    globalThis.WebSocket = originalWebSocket;
  }
});

async function reserveLoopbackPort() {
  const server = createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("Failed to reserve loopback port");
  }

  const port = address.port;
  await new Promise((resolve) => {
    server.close(() => resolve(undefined));
  });
  return port;
}

function createHandshakeInfo() {
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
      available_backends: ["native"],
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
}

function createOpenThrowingCloseWebSocket() {
  return class OpenThrowingCloseWebSocket {
    static CONNECTING = 0;
    static OPEN = 1;
    static CLOSING = 2;
    static CLOSED = 3;
    readyState = OpenThrowingCloseWebSocket.CONNECTING;
    #listeners = new Map();

    constructor() {
      queueMicrotask(() => {
        this.readyState = OpenThrowingCloseWebSocket.OPEN;
        this.#emit("open", { type: "open" });
      });
    }

    addEventListener(type, listener) {
      const bucket = this.#listeners.get(type) ?? new Set();
      bucket.add(listener);
      this.#listeners.set(type, bucket);
    }

    removeEventListener(type, listener) {
      this.#listeners.get(type)?.delete(listener);
    }

    send() {}

    close() {
      throw new Error("simulated close failure");
    }

    #emit(type, event) {
      for (const listener of this.#listeners.get(type) ?? []) {
        listener.call(this, event);
      }
    }
  };
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
