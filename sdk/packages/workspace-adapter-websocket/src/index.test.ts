import { randomUUID } from "node:crypto";
import { createServer } from "node:net";

import { WebSocket as NodeWebSocket, WebSocketServer, type WebSocket } from "ws";
import { afterEach, describe, expect, it } from "vitest";

import { createMemoryWorkspaceTransport } from "@terminal-platform/workspace-adapter-memory";
import type { WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";

import { decodeWorkspaceWebSocketPayload, encodeWorkspaceWebSocketPayload } from "./json-codec.js";
import {
  createWorkspaceWebSocketTransport,
  type WorkspaceGatewayControlClientMessage,
  type WorkspaceGatewayControlServerResponse,
  type WorkspaceGatewayStreamClientMessage,
  type WorkspaceGatewayStreamServerMessage,
} from "./index.js";

const canBindLoopback = await probeLoopbackTcp();

describe.skipIf(!canBindLoopback)("workspace websocket adapter", () => {
  const cleanups: Array<() => Promise<void>> = [];

  afterEach(async () => {
    while (cleanups.length > 0) {
      const cleanup = cleanups.pop();
      await cleanup?.();
    }
  });

  it("round-trips control plane requests against a websocket host", async () => {
    const fixture = createMemoryWorkspaceTransport();
    const gateway = await startWorkspaceGateway(fixture);
    cleanups.push(() => gateway.dispose());

    const transport = createWorkspaceWebSocketTransport({
      controlUrl: gateway.controlUrl,
      streamUrl: gateway.streamUrl,
      webSocketFactory: createNodeWebSocket,
    });
    cleanups.push(() => transport.close());

    const sessions = await transport.listSessions();
    const saved = await transport.listSavedSessions();
    const attached = await transport.attachSession(sessions[0]!.session_id);
    const topology = await transport.getTopologySnapshot(sessions[0]!.session_id);
    const screen = await transport.getScreenSnapshot(
      sessions[0]!.session_id,
      attached.focused_screen!.pane_id,
    );
    const delta = await transport.getScreenDelta(
      sessions[0]!.session_id,
      attached.focused_screen!.pane_id,
      screen.sequence,
    );

    expect(sessions).toHaveLength(1);
    expect(saved).toHaveLength(1);
    expect(attached.session.session_id).toBe(sessions[0]!.session_id);
    expect(topology.session_id).toBe(sessions[0]!.session_id);
    expect(delta.to_sequence).toBe(screen.sequence);
  });

  it("retries transient control plane connection failures", async () => {
    const fixture = createMemoryWorkspaceTransport();
    const gateway = await startWorkspaceGateway(fixture);
    cleanups.push(() => gateway.dispose());

    let attempts = 0;
    const transport = createWorkspaceWebSocketTransport({
      controlUrl: gateway.controlUrl,
      streamUrl: gateway.streamUrl,
      webSocketFactory: (url, protocols) => {
        attempts += 1;
        if (attempts <= 2) {
          return createFailingWebSocket();
        }

        return createNodeWebSocket(url, protocols);
      },
    });
    cleanups.push(() => transport.close());

    await expect(transport.listSessions()).resolves.toHaveLength(1);
    expect(attempts).toBe(3);
  });

  it("preserves storage pressure gateway errors for workspace diagnostics", async () => {
    const storagePressureError = new Error("simulated storage pressure") as Error & { code: string };
    storagePressureError.code = "storage_pressure";
    const fixture = {
      ...createMemoryWorkspaceTransport(),
      dispatchMuxCommand: async () => {
        throw storagePressureError;
      },
    } as WorkspaceTransportClient;
    const gateway = await startWorkspaceGateway(fixture);
    cleanups.push(() => gateway.dispose());

    const transport = createWorkspaceWebSocketTransport({
      controlUrl: gateway.controlUrl,
      streamUrl: gateway.streamUrl,
      webSocketFactory: createNodeWebSocket,
    });
    cleanups.push(() => transport.close());

    const session = (await transport.listSessions())[0]!;
    await expect(transport.dispatchMuxCommand(session.session_id, {
      kind: "send_input",
      pane_id: "pane-1",
      data: "echo should-fail\r",
    })).rejects.toMatchObject({
      code: "storage_pressure",
      message: "simulated storage pressure",
      recoverable: true,
    });
  });

  it("streams subscription events over the websocket stream plane", async () => {
    const fixture = createMemoryWorkspaceTransport();
    const gateway = await startWorkspaceGateway(fixture);
    cleanups.push(() => gateway.dispose());

    const transport = createWorkspaceWebSocketTransport({
      controlUrl: gateway.controlUrl,
      streamUrl: gateway.streamUrl,
      webSocketFactory: createNodeWebSocket,
    });
    cleanups.push(() => transport.close());

    const session = (await transport.listSessions())[0]!;
    const attached = await transport.attachSession(session.session_id);
    const subscription = await transport.openSubscription(session.session_id, {
      kind: "pane_surface",
      pane_id: attached.focused_screen!.pane_id,
    });
    const firstEvent = await subscription.nextEvent();

    expect(subscription.meta().subscription_id).toMatch(/^memory-subscription-/);
    expect(firstEvent?.kind).toBe("screen_delta");

    await subscription.close();
  });
});

describe("workspace websocket adapter failure handling", () => {
  it("rejects control requests when websocket send throws synchronously", async () => {
    const transport = createWorkspaceWebSocketTransport({
      controlUrl: "ws://127.0.0.1/terminal-gateway/control",
      streamUrl: "ws://127.0.0.1/terminal-gateway/stream",
      webSocketFactory: createOpenThenThrowingSendWebSocket,
    });

    await expect(transport.listSessions()).rejects.toThrow("simulated websocket send failure");
    await transport.close();
  });

  it("settles pending control retry waits when the transport closes", async () => {
    let attempts = 0;
    const transport = createWorkspaceWebSocketTransport({
      controlUrl: "ws://127.0.0.1/terminal-gateway/control",
      streamUrl: "ws://127.0.0.1/terminal-gateway/stream",
      webSocketFactory: () => {
        attempts += 1;
        return createFailingWebSocket();
      },
    });
    const pending = transport.listSessions();

    await sleep(20);
    expect(attempts).toBeGreaterThan(0);
    await transport.close();

    await expect(withTimeout(pending, 500)).rejects.toThrow("Failed to connect to workspace control plane");
  });

  it("does not hang when websocket close never emits a close event", async () => {
    const transport = createWorkspaceWebSocketTransport({
      controlUrl: "ws://127.0.0.1/terminal-gateway/control",
      streamUrl: "ws://127.0.0.1/terminal-gateway/stream",
      webSocketFactory: createOpenNeverClosingWebSocket,
    });
    const pending = transport.listSessions();
    void pending.catch(() => undefined);

    await sleep(20);
    await expect(withTimeout(transport.close(), 750)).resolves.toBeUndefined();
    await expect(pending).rejects.toThrow("workspace websocket transport closed");
  });
});

async function startWorkspaceGateway(transport: WorkspaceTransportClient): Promise<{
  controlUrl: string;
  streamUrl: string;
  dispose(): Promise<void>;
}> {
  const wss = new WebSocketServer({
    host: "127.0.0.1",
    port: 0,
  });
  const token = randomUUID();
  const streamSubscriptions = new Map<string, Awaited<ReturnType<WorkspaceTransportClient["openSubscription"]>>>();

  wss.on("connection", (socket, request) => {
    const url = new URL(request.url ?? "/", "ws://127.0.0.1");
    if (url.searchParams.get("token") !== token) {
      socket.close(1008, "Unauthorized");
      return;
    }

    const plane = url.pathname.endsWith("/stream") ? "stream" : "control";
    if (plane === "control") {
      socket.on("message", (payload) => {
        void handleControlMessage(socket, payload.toString(), transport);
      });
      return;
    }

    socket.on("message", (payload) => {
      void handleStreamMessage(socket, payload.toString(), transport, streamSubscriptions);
    });
    socket.on("close", () => {
      void Promise.allSettled(
        [...streamSubscriptions.values()].map(async (subscription) => {
          await subscription.close();
        }),
      );
      streamSubscriptions.clear();
    });
  });

  await new Promise<void>((resolve, reject) => {
    const cleanup = () => {
      wss.off("error", onError);
      wss.off("listening", onListening);
    };
    const onError = (error: Error) => {
      cleanup();
      reject(error);
    };
    const onListening = () => {
      cleanup();
      resolve();
    };

    wss.once("error", onError);
    wss.once("listening", onListening);
  });

  const address = wss.address();
  if (!address || typeof address === "string") {
    throw new Error("expected TCP address");
  }

  return {
    controlUrl: `ws://127.0.0.1:${address.port}/terminal-gateway/control?token=${token}`,
    streamUrl: `ws://127.0.0.1:${address.port}/terminal-gateway/stream?token=${token}`,
    async dispose() {
      for (const subscription of streamSubscriptions.values()) {
        await subscription.close();
      }
      await new Promise<void>((resolve, reject) => {
        wss.close((error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      });
      await transport.close();
    },
  };
}

async function handleControlMessage(
  socket: WebSocket,
  raw: string,
  transport: WorkspaceTransportClient,
): Promise<void> {
  const message = decodeWorkspaceWebSocketPayload<WorkspaceGatewayControlClientMessage>(raw);
  try {
    const result = await dispatchControl(transport, message);
    const response: WorkspaceGatewayControlServerResponse = {
      type: "response",
      requestId: message.requestId,
      method: message.method,
      ok: true,
      result,
    } as WorkspaceGatewayControlServerResponse;
    socket.send(encodeWorkspaceWebSocketPayload(response));
  } catch (error) {
    const response: WorkspaceGatewayControlServerResponse = {
      type: "response",
      requestId: message.requestId,
      method: message.method,
      ok: false,
      error: {
        message: error instanceof Error ? error.message : String(error),
        ...readErrorCode(error),
      },
    };
    socket.send(encodeWorkspaceWebSocketPayload(response));
  }
}

function readErrorCode(error: unknown): { code?: string } {
  return error instanceof Error && "code" in error && typeof error.code === "string"
    ? { code: error.code }
    : {};
}

async function dispatchControl(
  transport: WorkspaceTransportClient,
  message: WorkspaceGatewayControlClientMessage,
): Promise<unknown> {
  switch (message.method) {
    case "workspace_handshake":
      return transport.handshake();
    case "workspace_list_sessions":
      return transport.listSessions();
    case "workspace_list_saved_sessions":
      return transport.listSavedSessions();
    case "workspace_discover_sessions":
      return transport.discoverSessions(message.payload.backend);
    case "workspace_backend_capabilities":
      return transport.getBackendCapabilities(message.payload.backend);
    case "workspace_create_session":
      return transport.createSession(message.payload.backend, message.payload.request);
    case "workspace_import_session":
      return transport.importSession(message.payload.route, message.payload.title ?? null);
    case "workspace_saved_session":
      return transport.getSavedSession(message.payload.sessionId);
    case "workspace_command_history":
      return transport.listCommandHistory?.(message.payload.sessionId ?? null, message.payload.limit ?? null) ?? [];
    case "workspace_pane_history":
      if (!transport.getPaneHistory) {
        throw new Error("pane history is not supported by this transport");
      }
      return transport.getPaneHistory(
        message.payload.sessionId,
        message.payload.paneId,
        {
          fromEventSeq: message.payload.fromEventSeq ?? null,
          maxSegments: message.payload.maxSegments ?? null,
          maxBytes: message.payload.maxBytes ?? null,
        },
      );
    case "workspace_prune_saved_sessions":
      return transport.pruneSavedSessions(message.payload.keepLatest);
    case "workspace_restore_saved_session":
      return transport.restoreSavedSession(message.payload.sessionId);
    case "workspace_delete_saved_session":
      return transport.deleteSavedSession(message.payload.sessionId);
    case "workspace_attach_session":
      return transport.attachSession(message.payload.sessionId);
    case "workspace_topology_snapshot":
      return transport.getTopologySnapshot(message.payload.sessionId);
    case "workspace_screen_snapshot":
      return transport.getScreenSnapshot(message.payload.sessionId, message.payload.paneId);
    case "workspace_screen_delta":
      return transport.getScreenDelta(
        message.payload.sessionId,
        message.payload.paneId,
        message.payload.fromSequence,
      );
    case "workspace_dispatch_mux_command":
      return transport.dispatchMuxCommand(message.payload.sessionId, message.payload.command);
    default:
      return assertNever(message);
  }
}

async function handleStreamMessage(
  socket: WebSocket,
  raw: string,
  transport: WorkspaceTransportClient,
  subscriptions: Map<string, Awaited<ReturnType<WorkspaceTransportClient["openSubscription"]>>>,
): Promise<void> {
  const message = decodeWorkspaceWebSocketPayload<WorkspaceGatewayStreamClientMessage>(raw);
  switch (message.type) {
    case "workspace_subscribe": {
      const subscription = await transport.openSubscription(message.sessionId, message.spec);
      subscriptions.set(message.subscriptionId, subscription);
      const ack: WorkspaceGatewayStreamServerMessage = {
        type: "workspace_subscription_ack",
        subscriptionId: message.subscriptionId,
        meta: {
          ...subscription.meta(),
          subscription_id: `memory-subscription-${message.subscriptionId}`,
        },
      };
      socket.send(encodeWorkspaceWebSocketPayload(ack));
      void pumpSubscription(socket, message.subscriptionId, subscription, subscriptions);
      return;
    }
    case "workspace_unsubscribe": {
      const subscription = subscriptions.get(message.subscriptionId);
      if (!subscription) {
        return;
      }
      subscriptions.delete(message.subscriptionId);
      await subscription.close();
      socket.send(
        encodeWorkspaceWebSocketPayload({
          type: "workspace_subscription_closed",
          subscriptionId: message.subscriptionId,
        } satisfies WorkspaceGatewayStreamServerMessage),
      );
      return;
    }
    default:
      assertNever(message);
  }
}

async function pumpSubscription(
  socket: WebSocket,
  subscriptionId: string,
  subscription: Awaited<ReturnType<WorkspaceTransportClient["openSubscription"]>>,
  subscriptions: Map<string, Awaited<ReturnType<WorkspaceTransportClient["openSubscription"]>>>,
): Promise<void> {
  try {
    while (subscriptions.get(subscriptionId) === subscription) {
      const event = await subscription.nextEvent();
      if (!event) {
        break;
      }
      socket.send(
        encodeWorkspaceWebSocketPayload({
          type: "workspace_subscription_event",
          subscriptionId,
          event,
        } satisfies WorkspaceGatewayStreamServerMessage),
      );
    }
  } finally {
    if (subscriptions.get(subscriptionId) === subscription) {
      subscriptions.delete(subscriptionId);
      socket.send(
        encodeWorkspaceWebSocketPayload({
          type: "workspace_subscription_closed",
          subscriptionId,
        } satisfies WorkspaceGatewayStreamServerMessage),
      );
      await subscription.close();
    }
  }
}

async function probeLoopbackTcp(): Promise<boolean> {
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

function assertNever(value: never): never {
  throw new Error(`Unsupported message: ${JSON.stringify(value)}`);
}

function createNodeWebSocket(url: string, protocols?: string[]): globalThis.WebSocket {
  return new NodeWebSocket(url, protocols) as unknown as globalThis.WebSocket;
}

function createFailingWebSocket(): globalThis.WebSocket {
  const listeners = new Map<string, Set<EventListener>>();
  const socket = {
    readyState: 3,
    addEventListener(type: string, listener: EventListener) {
      const bucket = listeners.get(type) ?? new Set<EventListener>();
      bucket.add(listener);
      listeners.set(type, bucket);
    },
    removeEventListener(type: string, listener: EventListener) {
      listeners.get(type)?.delete(listener);
    },
    close() {
      emit("close");
    },
  } as unknown as globalThis.WebSocket;

  const emit = (type: string) => {
    for (const listener of listeners.get(type) ?? []) {
      listener.call(socket, { type } as Event);
    }
  };

  queueMicrotask(() => {
    emit("error");
  });

  return socket;
}

function createOpenThenThrowingSendWebSocket(): globalThis.WebSocket {
  const listeners = new Map<string, Set<EventListener>>();
  const socket = {
    readyState: 0,
    addEventListener(type: string, listener: EventListener) {
      const bucket = listeners.get(type) ?? new Set<EventListener>();
      bucket.add(listener);
      listeners.set(type, bucket);
    },
    removeEventListener(type: string, listener: EventListener) {
      listeners.get(type)?.delete(listener);
    },
    send() {
      throw new Error("simulated websocket send failure");
    },
    close() {
      socket.readyState = 3;
      emit("close");
    },
  } as unknown as globalThis.WebSocket & { readyState: number };

  const emit = (type: string) => {
    for (const listener of listeners.get(type) ?? []) {
      listener.call(socket, { type } as Event);
    }
  };

  queueMicrotask(() => {
    socket.readyState = 1;
    emit("open");
  });

  return socket;
}

function createOpenNeverClosingWebSocket(): globalThis.WebSocket {
  const listeners = new Map<string, Set<EventListener>>();
  const socket = {
    readyState: 0,
    addEventListener(type: string, listener: EventListener) {
      const bucket = listeners.get(type) ?? new Set<EventListener>();
      bucket.add(listener);
      listeners.set(type, bucket);
    },
    removeEventListener(type: string, listener: EventListener) {
      listeners.get(type)?.delete(listener);
    },
    send() {},
    close() {
      socket.readyState = 2;
    },
  } as unknown as globalThis.WebSocket & { readyState: number };

  const emit = (type: string) => {
    for (const listener of listeners.get(type) ?? []) {
      listener.call(socket, { type } as Event);
    }
  };

  queueMicrotask(() => {
    socket.readyState = 1;
    emit("open");
  });

  return socket;
}

async function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = setTimeout(() => reject(new Error(`timed out after ${timeoutMs}ms`)), timeoutMs);
      }),
    ]);
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
