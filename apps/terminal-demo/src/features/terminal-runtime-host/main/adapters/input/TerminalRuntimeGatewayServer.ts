import { once } from "node:events";
import { randomUUID } from "node:crypto";
import type { IncomingMessage } from "node:http";
import { WebSocketServer, WebSocket } from "ws";
import type { TerminalDiscoveredSession } from "@features/terminal-workspace-kernel/contracts";
import { buildDiscoveredSessionDegradedSemantics } from "@features/terminal-workspace-kernel/contracts";
import type {
  TerminalGatewayControlClientMessage,
  TerminalGatewayControlRequestMap,
  TerminalGatewayControlServerResponse,
  TerminalGatewayErrorEnvelope,
  TerminalGatewayStreamClientMessage,
  TerminalGatewayStreamServerMessage,
} from "../../../contracts/terminal-gateway-protocol.js";
import {
  type TerminalRuntimeDiscoveredSession,
  TerminalRuntimeControlService,
  TerminalRuntimeSessionStreamService,
} from "../../../core/application/index.js";
import type { TerminalPlatformClientProvider } from "../../infrastructure/TerminalPlatformClientProvider.js";

interface TerminalRuntimeGatewayServerOptions {
  runtimeSlug: string;
  controlService: TerminalRuntimeControlService;
  sessionStreamService: TerminalRuntimeSessionStreamService;
  clientProvider: TerminalPlatformClientProvider;
}

interface ControlConnectionRecord {
  socket: WebSocket;
  importHandles: Map<string, TerminalRuntimeDiscoveredSession>;
}

interface LegacyStreamSubscriptionRecord {
  kind: "legacy_session_state";
  sessionId: string;
  handle: Awaited<ReturnType<TerminalRuntimeSessionStreamService["watchSessionState"]>> | null;
}

type StreamSubscriptionRecord = LegacyStreamSubscriptionRecord;

interface StreamConnectionRecord {
  socket: WebSocket;
  subscriptions: Map<string, StreamSubscriptionRecord>;
}

interface GatewayControlRequestMessage {
  type: "request";
  requestId: string;
  method: string;
  payload: unknown;
}

type GatewayControlResponseMessage =
  | {
      type: "response";
      requestId: string;
      method: string;
      ok: true;
      result: unknown;
    }
  | {
      type: "response";
      requestId: string;
      method: string;
      ok: false;
      error: TerminalGatewayErrorEnvelope;
    };

export class TerminalRuntimeGatewayServer {
  readonly #runtimeSlug: string;
  readonly #token = randomUUID();
  readonly #controlService: TerminalRuntimeControlService;
  readonly #sessionStreamService: TerminalRuntimeSessionStreamService;
  readonly #clientProvider: TerminalPlatformClientProvider;
  readonly #server: WebSocketServer;
  readonly #controlConnections = new Set<ControlConnectionRecord>();
  readonly #streamConnections = new Set<StreamConnectionRecord>();

  private constructor(options: TerminalRuntimeGatewayServerOptions) {
    this.#runtimeSlug = options.runtimeSlug;
    this.#controlService = options.controlService;
    this.#sessionStreamService = options.sessionStreamService;
    this.#clientProvider = options.clientProvider;
    this.#server = new WebSocketServer({
      host: "127.0.0.1",
      port: 0,
    });
    this.#server.on("connection", (socket: WebSocket, request: IncomingMessage) => {
      this.handleConnection(socket, request.url ?? "");
    });
  }

  static async start(
    options: TerminalRuntimeGatewayServerOptions,
  ): Promise<TerminalRuntimeGatewayServer> {
    const server = new TerminalRuntimeGatewayServer(options);
    await once(server.#server, "listening");
    return server;
  }

  get runtimeSlug(): string {
    return this.#runtimeSlug;
  }

  get controlPlaneUrl(): string {
    return this.buildPlaneUrl("control");
  }

  get sessionStreamUrl(): string {
    return this.buildPlaneUrl("stream");
  }

  async dispose(): Promise<void> {
    for (const connection of this.#streamConnections) {
      await this.disposeStreamConnection(connection);
      connection.socket.close();
    }

    for (const connection of this.#controlConnections) {
      this.disposeControlConnection(connection);
      connection.socket.close();
    }

    this.#streamConnections.clear();
    this.#controlConnections.clear();
    this.#server.close();
    await once(this.#server, "close");
  }

  private buildPlaneUrl(plane: "control" | "stream"): string {
    const address = this.#server.address();
    if (!address || typeof address === "string") {
      throw new Error("terminal demo gateway did not expose a TCP port");
    }

    return `ws://127.0.0.1:${address.port}/terminal-gateway/${plane}?token=${this.#token}`;
  }

  private handleConnection(socket: WebSocket, rawUrl: string): void {
    const url = new URL(rawUrl, "ws://127.0.0.1");
    const plane = parseGatewayPlane(url.pathname);
    if (!plane || url.searchParams.get("token") !== this.#token) {
      socket.close(1008, "Unauthorized gateway client");
      return;
    }

    if (plane === "control") {
      const connection: ControlConnectionRecord = {
        socket,
        importHandles: new Map(),
      };
      this.#controlConnections.add(connection);

      socket.on("message", (payload: WebSocket.RawData) => {
        void this.handleControlMessage(connection, payload.toString());
      });

      socket.on("close", () => {
        this.#controlConnections.delete(connection);
        this.disposeControlConnection(connection);
      });
      return;
    }

    const connection: StreamConnectionRecord = {
      socket,
      subscriptions: new Map(),
    };
    this.#streamConnections.add(connection);

    socket.on("message", (payload: WebSocket.RawData) => {
      void this.handleStreamMessage(connection, payload.toString());
    });

    socket.on("close", () => {
      this.#streamConnections.delete(connection);
      void this.disposeStreamConnection(connection);
    });
  }

  private async handleControlMessage(
    connection: ControlConnectionRecord,
    payload: string,
  ): Promise<void> {
    let message: GatewayControlRequestMessage;

    try {
      message = parseControlClientMessage(payload);
    } catch (error) {
      this.sendControl(connection.socket, {
        type: "response",
        requestId: "invalid-request",
        method: "list_sessions",
        ok: false,
        error: serializeError(error),
      });
      return;
    }

    try {
      const result = await this.dispatchControlRequest(connection, message);
      this.sendSuccessResponse(connection.socket, message.requestId, message.method, result);
    } catch (error) {
      this.sendControl(connection.socket, {
        type: "response",
        requestId: message.requestId,
        method: message.method,
        ok: false,
        error: serializeError(error),
      });
    }
  }

  private async handleStreamMessage(
    connection: StreamConnectionRecord,
    payload: string,
  ): Promise<void> {
    let message: TerminalGatewayStreamClientMessage;

    try {
      message = parseStreamClientMessage(payload);
    } catch {
      connection.socket.close(1008, "Invalid stream message");
      return;
    }

    switch (message.type) {
      case "stream_subscribe_session_state":
        await this.subscribeSessionState(connection, message.subscriptionId, message.sessionId);
        return;
      case "stream_unsubscribe_session_state":
        await this.unsubscribeSessionState(connection, message.subscriptionId, message.sessionId);
        return;
    }
  }

  private async dispatchControlRequest(
    connection: ControlConnectionRecord,
    message: GatewayControlRequestMessage,
  ): Promise<unknown> {
    const payload = asGatewayPayload(message.payload);

    switch (message.method) {
      case "handshake_info":
        return this.#controlService.handshakeInfo();
      case "list_sessions":
        return this.#controlService.listSessions();
      case "list_saved_sessions":
        return this.#controlService.listSavedSessions();
      case "discover_sessions": {
        this.clearImportHandlesForBackend(connection, payload.backend);
        const discovered = await this.#controlService.discoverSessions(payload.backend);
        return discovered.map((session) => this.registerImportHandle(connection, session));
      }
      case "backend_capabilities":
        return this.#controlService.backendCapabilities(payload.backend);
      case "create_native_session":
        return this.#controlService.createNativeSession(payload);
      case "import_session": {
        const discovered = connection.importHandles.get(payload.importHandle);
        if (!discovered) {
          throw new Error(`Unknown import handle ${payload.importHandle}`);
        }

        return this.#controlService.importSession(
          payload.title
            ? {
                route: discovered.route,
                title: payload.title,
              }
            : {
                route: discovered.route,
              },
        );
      }
      case "restore_saved_session":
        return this.#controlService.restoreSavedSession(payload.sessionId);
      case "delete_saved_session":
        return this.#controlService.deleteSavedSession(payload.sessionId);
      case "dispatch_mux_command":
        return this.#controlService.dispatchMuxCommand(
          payload.sessionId,
          payload.command,
        );
      case "workspace_handshake": {
        const client = await this.#clientProvider.getClient();
        return client.handshakeInfo();
      }
      case "workspace_list_sessions": {
        const client = await this.#clientProvider.getClient();
        return client.listSessions();
      }
      case "workspace_list_saved_sessions": {
        const client = await this.#clientProvider.getClient();
        return client.listSavedSessions();
      }
      case "workspace_discover_sessions": {
        const client = await this.#clientProvider.getClient();
        return client.discoverSessions(payload.backend);
      }
      case "workspace_backend_capabilities": {
        const client = await this.#clientProvider.getClient();
        return client.backendCapabilities(payload.backend);
      }
      case "workspace_create_session": {
        if (payload.backend !== "native") {
          throw new Error(`Unsupported backend ${payload.backend}`);
        }

        const client = await this.#clientProvider.getClient();
        return client.createNativeSession(payload.request ?? undefined);
      }
      case "workspace_import_session": {
        const client = await this.#clientProvider.getClient();
        return client.importSession(payload.route, payload.title ?? null);
      }
      case "workspace_saved_session": {
        const client = await this.#clientProvider.getClient();
        return client.savedSession(payload.sessionId);
      }
      case "workspace_prune_saved_sessions": {
        const client = await this.#clientProvider.getClient();
        return client.pruneSavedSessions(payload.keepLatest);
      }
      case "workspace_restore_saved_session": {
        const client = await this.#clientProvider.getClient();
        return client.restoreSavedSession(payload.sessionId);
      }
      case "workspace_delete_saved_session": {
        const client = await this.#clientProvider.getClient();
        return client.deleteSavedSession(payload.sessionId);
      }
      case "workspace_attach_session": {
        const client = await this.#clientProvider.getClient();
        return client.attachSession(payload.sessionId);
      }
      case "workspace_topology_snapshot": {
        const client = await this.#clientProvider.getClient();
        return client.topologySnapshot(payload.sessionId);
      }
      case "workspace_screen_snapshot": {
        const client = await this.#clientProvider.getClient();
        return client.screenSnapshot(payload.sessionId, payload.paneId);
      }
      case "workspace_screen_delta": {
        const client = await this.#clientProvider.getClient();
        return client.screenDelta(
          payload.sessionId,
          payload.paneId,
          Number(payload.fromSequence),
        );
      }
      case "workspace_dispatch_mux_command": {
        const client = await this.#clientProvider.getClient();
        return client.dispatchMuxCommand(payload.sessionId, payload.command);
      }
      default:
        throw new Error("Unsupported gateway control method");
    }
  }

  private async subscribeSessionState(
    connection: StreamConnectionRecord,
    subscriptionId: string,
    sessionId: string,
  ): Promise<void> {
    if (connection.subscriptions.has(subscriptionId)) {
      this.sendStream(connection.socket, {
        type: "stream_subscription_rejected",
        subscriptionId,
        sessionId,
        error: {
          message: `Subscription ${subscriptionId} already exists`,
          code: "duplicate_subscription",
        },
      });
      return;
    }

    const record: LegacyStreamSubscriptionRecord = {
      kind: "legacy_session_state",
      sessionId,
      handle: null,
    };
    connection.subscriptions.set(subscriptionId, record);

    try {
      const handle = await this.#sessionStreamService.watchSessionState(sessionId, {
        onState: (state) => {
          this.sendStream(connection.socket, {
            type: "session_state",
            subscriptionId,
            sessionId,
            state,
          });
        },
        onError: (error) => {
          this.sendStream(connection.socket, {
            type: "subscription_error",
            subscriptionId,
            sessionId,
            error: serializeError(error),
          });
        },
        onClosed: () => {
          connection.subscriptions.delete(subscriptionId);
          this.sendStream(connection.socket, {
            type: "subscription_closed",
            subscriptionId,
            sessionId,
          });
        },
      });

      if (connection.subscriptions.get(subscriptionId) !== record) {
        await handle.dispose();
        return;
      }

      record.handle = handle;
      this.sendStream(connection.socket, {
        type: "stream_subscription_ack",
        subscriptionId,
        sessionId,
      });
    } catch (error) {
      connection.subscriptions.delete(subscriptionId);
      this.sendStream(connection.socket, {
        type: "stream_subscription_rejected",
        subscriptionId,
        sessionId,
        error: serializeError(error),
      });
    }
  }

  private async unsubscribeSessionState(
    connection: StreamConnectionRecord,
    subscriptionId: string,
    sessionId: string,
  ): Promise<void> {
    const record = connection.subscriptions.get(subscriptionId);
    if (!record || !record.handle) {
      connection.subscriptions.delete(subscriptionId);
      this.sendStream(connection.socket, {
        type: "subscription_closed",
        subscriptionId,
        sessionId,
      });
      return;
    }

    await record.handle.dispose();
  }

  private registerImportHandle(
    connection: ControlConnectionRecord,
    session: TerminalRuntimeDiscoveredSession,
  ): TerminalDiscoveredSession {
    const importHandle = randomUUID();
    connection.importHandles.set(importHandle, session);

    return {
      importHandle,
      backend: session.route.backend,
      title: session.title,
      sourceLabel: session.route.external?.namespace ?? `${session.route.backend} import`,
      degradedSemantics: buildDiscoveredSessionDegradedSemantics({
        backend: session.route.backend,
      }),
    };
  }

  private clearImportHandlesForBackend(
    connection: ControlConnectionRecord,
    backend: TerminalDiscoveredSession["backend"],
  ): void {
    for (const [handle, session] of connection.importHandles.entries()) {
      if (session.route.backend === backend) {
        connection.importHandles.delete(handle);
      }
    }
  }

  private disposeControlConnection(connection: ControlConnectionRecord): void {
    connection.importHandles.clear();
  }

  private async disposeStreamConnection(connection: StreamConnectionRecord): Promise<void> {
    const stops = [...connection.subscriptions.values()]
      .map((record) => record.handle?.dispose() ?? null)
      .filter(Boolean);
    connection.subscriptions.clear();
    await Promise.allSettled(stops);
  }

  private sendControl(socket: WebSocket, message: GatewayControlResponseMessage): void {
    this.send(socket, message);
  }

  private sendStream(socket: WebSocket, message: TerminalGatewayStreamServerMessage): void {
    this.send(socket, message);
  }

  private send(
    socket: WebSocket,
    message: GatewayControlResponseMessage | TerminalGatewayStreamServerMessage,
  ): void {
    if (socket.readyState !== WebSocket.OPEN) {
      return;
    }

    socket.send(encodeGatewayPayload(message));
  }

  private sendSuccessResponse(
    socket: WebSocket,
    requestId: string,
    method: string,
    result: unknown,
  ): void {
    this.sendControl(socket, {
      type: "response",
      requestId,
      method,
      ok: true,
      result,
    });
  }
}

function parseGatewayPlane(pathname: string): "control" | "stream" | null {
  switch (pathname) {
    case "/terminal-gateway":
    case "/terminal-gateway/control":
      return "control";
    case "/terminal-gateway/stream":
      return "stream";
    default:
      return null;
  }
}

function parseControlClientMessage(payload: string): GatewayControlRequestMessage {
  const parsed = decodeGatewayPayload<Partial<GatewayControlRequestMessage>>(payload);

  if (parsed.type !== "request") {
    throw new Error("Gateway control payload must be a request envelope");
  }

  if (typeof parsed.requestId !== "string" || parsed.requestId.length === 0) {
    throw new Error("Gateway control requestId must be a non-empty string");
  }

  if (typeof parsed.method !== "string" || parsed.method.length === 0) {
    throw new Error("Gateway control method must be a non-empty string");
  }

  return parsed as GatewayControlRequestMessage;
}

function parseStreamClientMessage(payload: string): TerminalGatewayStreamClientMessage {
  const parsed = decodeGatewayPayload<Partial<TerminalGatewayStreamClientMessage>>(payload);
  if (typeof parsed.type !== "string" || parsed.type.length === 0) {
    throw new Error("Gateway stream message type must be a non-empty string");
  }

  if (typeof parsed.subscriptionId !== "string" || parsed.subscriptionId.length === 0) {
    throw new Error("Gateway stream subscriptionId must be a non-empty string");
  }

  if (typeof parsed.sessionId !== "string" || parsed.sessionId.length === 0) {
    throw new Error("Gateway stream message requires a non-empty sessionId");
  }

  if (
    parsed.type !== "stream_subscribe_session_state"
    && parsed.type !== "stream_unsubscribe_session_state"
  ) {
    throw new Error("Unsupported gateway stream message");
  }

  return parsed as TerminalGatewayStreamClientMessage;
}

function serializeError(error: unknown): TerminalGatewayErrorEnvelope {
  if (error instanceof Error) {
    const code = "code" in error && typeof error.code === "string"
      ? error.code
      : undefined;

    return code
      ? {
          message: error.message,
          code,
        }
      : {
          message: error.message,
        };
  }

  return {
    message: typeof error === "string" ? error : "Unknown gateway error",
  };
}

function asGatewayPayload(value: unknown): Record<string, any> {
  if (!value || typeof value !== "object") {
    return {};
  }

  return value as Record<string, any>;
}

function encodeGatewayPayload(value: unknown): string {
  return JSON.stringify(value, (_key, candidate) => {
    if (typeof candidate === "bigint") {
      return {
        $bigint: candidate.toString(),
      };
    }

    return candidate;
  });
}

function decodeGatewayPayload<T>(raw: string): T {
  return JSON.parse(raw, (_key, candidate) => {
    if (
      candidate
      && typeof candidate === "object"
      && "$bigint" in candidate
      && typeof candidate.$bigint === "string"
    ) {
      return BigInt(candidate.$bigint);
    }

    return candidate;
  }) as T;
}
