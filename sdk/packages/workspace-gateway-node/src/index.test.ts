import { createServer } from "node:net";

import { WebSocket as NodeWebSocket } from "ws";
import { afterEach, describe, expect, it } from "vitest";

import { createMemoryWorkspaceTransport } from "@terminal-platform/workspace-adapter-memory";
import {
  decodeWorkspaceWebSocketPayload,
  encodeWorkspaceWebSocketPayload,
  createWorkspaceWebSocketTransport,
  type WorkspaceGatewayControlServerResponse,
  type WorkspaceGatewayStreamServerMessage,
} from "@terminal-platform/workspace-adapter-websocket";
import type {
  WorkspaceSubscription,
  WorkspaceTransportClient,
} from "@terminal-platform/workspace-contracts";

import {
  WorkspaceGatewayNodeServer,
  dispatchWorkspaceGatewayControlRequest,
  startWorkspaceGatewayNodeServer,
  type WorkspaceGatewayNodeServerHandle,
  type WorkspaceRuntimeClientPort,
} from "./index.js";

const canBindLoopback = await probeLoopbackTcp();

describe.skipIf(!canBindLoopback)("workspace gateway node", () => {
  const cleanups: Array<() => Promise<void>> = [];

  afterEach(async () => {
    while (cleanups.length > 0) {
      const cleanup = cleanups.pop();
      await cleanup?.();
    }
  });

  it("serves the public websocket transport over control and stream planes", async () => {
    const runtime = createMemoryWorkspaceTransport();
    const gateway = await startWorkspaceGatewayNodeServer({ runtime });
    cleanups.push(() => gateway.dispose());

    const transport = createWorkspaceWebSocketTransport({
      controlUrl: gateway.controlUrl,
      streamUrl: gateway.streamUrl,
      webSocketFactory: createNodeWebSocket,
    });
    cleanups.push(() => transport.close());

    const sessions = await transport.listSessions();
    const attached = await transport.attachSession(sessions[0]!.session_id);
    const topology = await transport.getTopologySnapshot(sessions[0]!.session_id);
    const screen = await transport.getScreenSnapshot(
      sessions[0]!.session_id,
      attached.focused_screen!.pane_id,
    );
    const subscription = await transport.openSubscription(sessions[0]!.session_id, {
      kind: "pane_surface",
      pane_id: attached.focused_screen!.pane_id,
    });
    const event = await subscription.nextEvent();

    expect(gateway).toBeInstanceOf(WorkspaceGatewayNodeServer);
    expect(sessions).toHaveLength(1);
    expect(topology.session_id).toBe(sessions[0]!.session_id);
    expect(screen.surface.lines.length).toBeGreaterThan(0);
    expect(event?.kind).toBe("screen_delta");

    await subscription.close();
  });

  it("rejects unauthorized clients before they reach runtime ports", async () => {
    const runtime = createCountingRuntime(createMemoryWorkspaceTransport());
    const gateway = await startWorkspaceGatewayNodeServer({ runtime });
    cleanups.push(() => gateway.dispose());

    const unauthorizedUrl = new URL(gateway.controlUrl);
    unauthorizedUrl.searchParams.set("token", "wrong");
    const socket = new NodeWebSocket(unauthorizedUrl);
    cleanups.push(() => closeNodeWebSocket(socket));

    const close = await waitForSocketClose(socket);
    expect(close.code).toBe(1008);
    expect(runtime.calls).toHaveLength(0);
  });

  it("rejects malformed control payloads before runtime calls", async () => {
    const runtime = createCountingRuntime(createMemoryWorkspaceTransport());
    const gateway = await startWorkspaceGatewayNodeServer({ runtime });
    cleanups.push(() => gateway.dispose());

    const socket = await openNodeWebSocket(gateway.controlUrl);
    cleanups.push(() => closeNodeWebSocket(socket));

    socket.send(JSON.stringify({
      type: "request",
      requestId: "bad-backend",
      method: "workspace_backend_capabilities",
      payload: { backend: "screen" },
    }));

    const response = decodeWorkspaceWebSocketPayload<WorkspaceGatewayControlServerResponse>(
      await waitForSocketMessage(socket),
    );
    expect(response.ok).toBe(false);
    expect(response.error.code).toBe("protocol_error");
    expect(response.error.message).toContain("backend");
    expect(runtime.calls).toHaveLength(0);
  });

  it("settles unsubscribe when runtime subscription close hangs", async () => {
    const subscription = createHangingSubscription();
    const runtime = {
      ...createMemoryWorkspaceTransport(),
      openSubscription: async () => subscription,
    } satisfies WorkspaceRuntimeClientPort;
    const gateway = await startWorkspaceGatewayNodeServer({
      runtime,
      closeTimeoutMs: 20,
    });
    cleanups.push(() => gateway.dispose());

    const socket = await openNodeWebSocket(gateway.streamUrl);
    cleanups.push(() => closeNodeWebSocket(socket));

    socket.send(encodeWorkspaceWebSocketPayload({
      type: "workspace_subscribe",
      subscriptionId: "sub-1",
      sessionId: "session-1",
      spec: {
        kind: "session_topology",
      },
    }));
    const ack = decodeWorkspaceWebSocketPayload<WorkspaceGatewayStreamServerMessage>(
      await waitForSocketMessage(socket),
    );
    expect(ack.type).toBe("workspace_subscription_ack");

    socket.send(encodeWorkspaceWebSocketPayload({
      type: "workspace_unsubscribe",
      subscriptionId: "sub-1",
    }));
    const closed = decodeWorkspaceWebSocketPayload<WorkspaceGatewayStreamServerMessage>(
      await waitForSocketMessage(socket),
    );

    expect(closed).toEqual({
      type: "workspace_subscription_closed",
      subscriptionId: "sub-1",
    });
    expect(subscription.closeCalls).toBe(1);
  });
});

describe("workspace gateway node public api", () => {
  it("exports dispatcher helpers for focused host tests", async () => {
    const runtime = createMemoryWorkspaceTransport();
    const sessions = await dispatchWorkspaceGatewayControlRequest(runtime, {
      type: "request",
      requestId: "list",
      method: "workspace_list_sessions",
      payload: undefined,
    });

    expect(Array.isArray(sessions)).toBe(true);
  });
});

function createNodeWebSocket(url: string, protocols?: string[]): globalThis.WebSocket {
  return new NodeWebSocket(url, protocols) as unknown as globalThis.WebSocket;
}

function createCountingRuntime(runtime: WorkspaceTransportClient): WorkspaceTransportClient & { calls: string[] } {
  const calls: string[] = [];
  return {
    ...runtime,
    calls,
    async getBackendCapabilities(backend) {
      calls.push("getBackendCapabilities");
      return runtime.getBackendCapabilities(backend);
    },
    async close() {
      calls.push("close");
      await runtime.close();
    },
  };
}

function createHangingSubscription(): WorkspaceSubscription & { closeCalls: number } {
  return {
    closeCalls: 0,
    meta: () => ({ subscription_id: "hanging-subscription" }),
    nextEvent: () => new Promise(() => undefined),
    close() {
      this.closeCalls += 1;
      return new Promise(() => undefined);
    },
  };
}

async function openNodeWebSocket(url: string): Promise<NodeWebSocket> {
  const socket = new NodeWebSocket(url);
  await new Promise<void>((resolve, reject) => {
    socket.once("open", () => resolve());
    socket.once("error", reject);
  });
  return socket;
}

async function closeNodeWebSocket(socket: NodeWebSocket): Promise<void> {
  if (socket.readyState === NodeWebSocket.CLOSED) {
    return;
  }

  await new Promise<void>((resolve) => {
    socket.once("close", () => resolve());
    socket.close();
    setTimeout(resolve, 50);
  });
}

async function waitForSocketMessage(socket: NodeWebSocket): Promise<string> {
  return new Promise((resolve, reject) => {
    socket.once("message", (data) => resolve(data.toString()));
    socket.once("error", reject);
  });
}

async function waitForSocketClose(socket: NodeWebSocket): Promise<{ code: number; reason: string }> {
  return new Promise((resolve) => {
    socket.once("close", (code, reason) => {
      resolve({ code, reason: reason.toString() });
    });
  });
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
