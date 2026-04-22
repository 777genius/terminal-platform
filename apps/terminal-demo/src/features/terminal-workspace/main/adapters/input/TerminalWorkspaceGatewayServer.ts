import { once } from "node:events";
import { randomUUID } from "node:crypto";
import type { IncomingMessage } from "node:http";
import { WebSocketServer, WebSocket } from "ws";
import type { TerminalDiscoveredSession } from "../../../contracts/terminal-workspace-contracts.js";
import type {
  TerminalGatewayControlClientMessage,
  TerminalGatewayControlRequestMap,
  TerminalGatewayErrorEnvelope,
  TerminalGatewayControlServerResponse,
  TerminalGatewayStreamClientMessage,
  TerminalGatewayStreamServerMessage,
} from "../../../contracts/terminal-gateway-protocol.js";
import {
  type TerminalRuntimeDiscoveredSession,
  TerminalWorkspaceControlService,
  TerminalWorkspaceSessionStreamService,
} from "../../../core/application/index.js";
import { buildDiscoveredSessionDegradedSemantics } from "../../../core/domain/index.js";
import type {
  WorkspaceGatewayControlClientMessage,
  WorkspaceGatewayControlRequestMap,
  WorkspaceGatewayControlServerResponse,
  WorkspaceGatewayStreamClientMessage,
  WorkspaceGatewayStreamServerMessage,
} from "@terminal-platform/workspace-adapter-websocket/protocol";
import type { SubscriptionEvent } from "@terminal-platform/runtime-types";
import type { TerminalPlatformClientProvider } from "../../infrastructure/TerminalPlatformClientProvider.js";

interface TerminalWorkspaceGatewayServerOptions {
  runtimeSlug: string;
  controlService: TerminalWorkspaceControlService;
  sessionStreamService: TerminalWorkspaceSessionStreamService;
  clientProvider: TerminalPlatformClientProvider;
}

interface ControlConnectionRecord {
  socket: WebSocket;
  importHandles: Map<string, TerminalRuntimeDiscoveredSession>;
}

interface LegacyStreamSubscriptionRecord {
  kind: "legacy_session_state";
  sessionId: string;
  handle: Awaited<ReturnType<TerminalWorkspaceSessionStreamService["watchSessionState"]>> | null;
}

interface WorkspaceStreamSubscriptionRecord {
  kind: "workspace_subscription";
  sessionId: string;
  handle: {
    subscriptionId: string;
    nextEvent(): Promise<SubscriptionEvent | null>;
    close(): Promise<void>;
  } | null;
}

type StreamSubscriptionRecord = LegacyStreamSubscriptionRecord | WorkspaceStreamSubscriptionRecord;

interface StreamConnectionRecord {
  socket: WebSocket;
  subscriptions: Map<string, StreamSubscriptionRecord>;
}

export class TerminalWorkspaceGatewayServer {
  readonly #runtimeSlug: string;
  readonly #token = randomUUID();
  readonly #controlService: TerminalWorkspaceControlService;
  readonly #sessionStreamService: TerminalWorkspaceSessionStreamService;
  readonly #clientProvider: TerminalPlatformClientProvider;
  readonly #server: WebSocketServer;
  readonly #controlConnections = new Set<ControlConnectionRecord>();
  readonly #streamConnections = new Set<StreamConnectionRecord>();

  private constructor(options: TerminalWorkspaceGatewayServerOptions) {
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
    options: TerminalWorkspaceGatewayServerOptions,
  ): Promise<TerminalWorkspaceGatewayServer> {
    const server = new TerminalWorkspaceGatewayServer(options);
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
    let message: TerminalGatewayControlClientMessage | WorkspaceGatewayControlClientMessage;

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
      const result = isWorkspaceControlMessage(message)
        ? await this.dispatchWorkspaceControlRequest(message)
        : await this.dispatchControlRequest(connection, message);
      this.sendSuccessResponse(connection.socket, message.requestId, message.method, result);
    } catch (error) {
      this.sendControl(connection.socket, {
        type: "response",
        requestId: message.requestId,
        method: message.method,
        ok: false,
        error: serializeError(error),
      } as TerminalGatewayControlServerResponse | WorkspaceGatewayControlServerResponse);
    }
  }

  private async handleStreamMessage(
    connection: StreamConnectionRecord,
    payload: string,
  ): Promise<void> {
    let message: TerminalGatewayStreamClientMessage | WorkspaceGatewayStreamClientMessage;

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
      case "workspace_subscribe":
        await this.subscribeWorkspaceStream(connection, message.subscriptionId, message.sessionId, message.spec);
        return;
      case "workspace_unsubscribe":
        await this.unsubscribeWorkspaceStream(connection, message.subscriptionId);
        return;
      default:
        assertNever(message);
    }
  }

  private async dispatchControlRequest(
    connection: ControlConnectionRecord,
    message: TerminalGatewayControlClientMessage,
  ) {
    switch (message.method) {
      case "handshake_info":
        return this.#controlService.handshakeInfo();
      case "list_sessions":
        return this.#controlService.listSessions();
      case "list_saved_sessions":
        return this.#controlService.listSavedSessions();
      case "discover_sessions": {
        this.clearImportHandlesForBackend(connection, message.payload.backend);
        const discovered = await this.#controlService.discoverSessions(message.payload.backend);
        return discovered.map((session) => this.registerImportHandle(connection, session));
      }
      case "backend_capabilities":
        return this.#controlService.backendCapabilities(message.payload.backend);
      case "create_native_session":
        return this.#controlService.createNativeSession(message.payload);
      case "import_session": {
        const discovered = connection.importHandles.get(message.payload.importHandle);
        if (!discovered) {
          throw new Error(`Unknown import handle ${message.payload.importHandle}`);
        }

        return this.#controlService.importSession(
          message.payload.title
            ? {
                route: discovered.route,
                title: message.payload.title,
              }
            : {
                route: discovered.route,
              },
        );
      }
      case "restore_saved_session":
        return this.#controlService.restoreSavedSession(message.payload.sessionId);
      case "delete_saved_session":
        return this.#controlService.deleteSavedSession(message.payload.sessionId);
      case "dispatch_mux_command":
        return this.#controlService.dispatchMuxCommand(
          message.payload.sessionId,
          message.payload.command,
        );
      default:
        throw new Error("Unsupported gateway control method");
    }
  }

  private async dispatchWorkspaceControlRequest(
    message: WorkspaceGatewayControlClientMessage,
  ) {
    const client = await this.#clientProvider.getClient();

    switch (message.method) {
      case "workspace_handshake":
        return (await client.handshakeInfo()).handshake;
      case "workspace_list_sessions":
        return client.listSessions();
      case "workspace_list_saved_sessions":
        return client.listSavedSessions();
      case "workspace_discover_sessions":
        return client.discoverSessions(message.payload.backend);
      case "workspace_backend_capabilities":
        return client.backendCapabilities(message.payload.backend);
      case "workspace_create_session":
        if (message.payload.backend !== "native") {
          throw new Error(`Unsupported backend for workspace_create_session: ${message.payload.backend}`);
        }
        return client.createNativeSession(message.payload.request);
      case "workspace_import_session":
        return client.importSession(message.payload.route, message.payload.title ?? null);
      case "workspace_saved_session":
        return client.savedSession(message.payload.sessionId);
      case "workspace_prune_saved_sessions":
        return client.pruneSavedSessions(message.payload.keepLatest);
      case "workspace_restore_saved_session":
        return client.restoreSavedSession(message.payload.sessionId);
      case "workspace_delete_saved_session":
        return client.deleteSavedSession(message.payload.sessionId);
      case "workspace_attach_session":
        return client.attachSession(message.payload.sessionId);
      case "workspace_topology_snapshot":
        return client.topologySnapshot(message.payload.sessionId);
      case "workspace_screen_snapshot":
        return client.screenSnapshot(message.payload.sessionId, message.payload.paneId);
      case "workspace_screen_delta":
        return client.screenDelta(
          message.payload.sessionId,
          message.payload.paneId,
          toSafeSequenceNumber(message.payload.fromSequence),
        );
      case "workspace_dispatch_mux_command":
        return client.dispatchMuxCommand(message.payload.sessionId, message.payload.command);
      default:
        throw new Error("Unsupported workspace gateway control method");
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
    if (!record || record.kind !== "legacy_session_state" || !record.handle) {
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

  private async subscribeWorkspaceStream(
    connection: StreamConnectionRecord,
    subscriptionId: string,
    sessionId: string,
    spec: import("@terminal-platform/runtime-types").SubscriptionSpec,
  ): Promise<void> {
    if (connection.subscriptions.has(subscriptionId)) {
      this.sendStream(connection.socket, {
        type: "workspace_subscription_rejected",
        subscriptionId,
        error: {
          message: `Subscription ${subscriptionId} already exists`,
          code: "duplicate_subscription",
        },
      });
      return;
    }

    const record: WorkspaceStreamSubscriptionRecord = {
      kind: "workspace_subscription",
      sessionId,
      handle: null,
    };
    connection.subscriptions.set(subscriptionId, record);

    try {
      const client = await this.#clientProvider.getClient();
      const handle = await client.openSubscription(sessionId, spec);

      if (connection.subscriptions.get(subscriptionId) !== record) {
        await handle.close();
        return;
      }

      record.handle = handle;
      this.sendStream(connection.socket, {
        type: "workspace_subscription_ack",
        subscriptionId,
        meta: {
          subscription_id: handle.subscriptionId,
        },
      });
      void this.pumpWorkspaceSubscription(connection, subscriptionId, record);
    } catch (error) {
      connection.subscriptions.delete(subscriptionId);
      this.sendStream(connection.socket, {
        type: "workspace_subscription_rejected",
        subscriptionId,
        error: serializeError(error),
      });
    }
  }

  private async unsubscribeWorkspaceStream(
    connection: StreamConnectionRecord,
    subscriptionId: string,
  ): Promise<void> {
    const record = connection.subscriptions.get(subscriptionId);
    if (!record || record.kind !== "workspace_subscription" || !record.handle) {
      connection.subscriptions.delete(subscriptionId);
      this.sendStream(connection.socket, {
        type: "workspace_subscription_closed",
        subscriptionId,
      });
      return;
    }

    connection.subscriptions.delete(subscriptionId);
    await record.handle.close();
    this.sendStream(connection.socket, {
      type: "workspace_subscription_closed",
      subscriptionId,
    });
  }

  private async pumpWorkspaceSubscription(
    connection: StreamConnectionRecord,
    subscriptionId: string,
    record: WorkspaceStreamSubscriptionRecord,
  ): Promise<void> {
    if (!record.handle) {
      return;
    }

    try {
      while (connection.subscriptions.get(subscriptionId) === record) {
        const event = await record.handle.nextEvent();
        if (!event) {
          break;
        }

        this.sendStream(connection.socket, {
          type: "workspace_subscription_event",
          subscriptionId,
          event,
        });
      }
    } catch (error) {
      if (connection.subscriptions.get(subscriptionId) === record) {
        this.sendStream(connection.socket, {
          type: "workspace_subscription_error",
          subscriptionId,
          error: serializeError(error),
        });
      }
    } finally {
      if (connection.subscriptions.get(subscriptionId) === record) {
        connection.subscriptions.delete(subscriptionId);
        await record.handle.close();
        this.sendStream(connection.socket, {
          type: "workspace_subscription_closed",
          subscriptionId,
        });
      }
    }
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
      .map((record) => {
        if (!record.handle) {
          return null;
        }

        return record.kind === "legacy_session_state"
          ? record.handle.dispose()
          : record.handle.close();
      })
      .filter(Boolean);
    connection.subscriptions.clear();
    await Promise.allSettled(stops);
  }

  private sendControl(
    socket: WebSocket,
    message: TerminalGatewayControlServerResponse | WorkspaceGatewayControlServerResponse,
  ): void {
    this.send(socket, message);
  }

  private sendStream(
    socket: WebSocket,
    message: TerminalGatewayStreamServerMessage | WorkspaceGatewayStreamServerMessage,
  ): void {
    this.send(socket, message);
  }

  private send(
    socket: WebSocket,
    message:
      | TerminalGatewayControlServerResponse
      | WorkspaceGatewayControlServerResponse
      | TerminalGatewayStreamServerMessage
      | WorkspaceGatewayStreamServerMessage,
  ): void {
    if (socket.readyState !== WebSocket.OPEN) {
      return;
    }

    socket.send(encodeGatewayPayload(message));
  }

  private sendSuccessResponse<
    RecordKey extends keyof (TerminalGatewayControlRequestMap & WorkspaceGatewayControlRequestMap),
  >(
    socket: WebSocket,
    requestId: string,
    method: RecordKey,
    result: (
      TerminalGatewayControlRequestMap
      & WorkspaceGatewayControlRequestMap
    )[RecordKey]["response"],
  ): void {
    this.sendControl(socket, {
      type: "response",
      requestId,
      method,
      ok: true,
      result,
    } as TerminalGatewayControlServerResponse | WorkspaceGatewayControlServerResponse);
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

function parseControlClientMessage(
  payload: string,
): TerminalGatewayControlClientMessage | WorkspaceGatewayControlClientMessage {
  const parsed = decodeGatewayPayload<Partial<TerminalGatewayControlClientMessage | WorkspaceGatewayControlClientMessage>>(payload);

  if (parsed.type !== "request") {
    throw new Error("Gateway control payload must be a request envelope");
  }

  if (typeof parsed.requestId !== "string" || parsed.requestId.length === 0) {
    throw new Error("Gateway control requestId must be a non-empty string");
  }

  if (typeof parsed.method !== "string" || parsed.method.length === 0) {
    throw new Error("Gateway control method must be a non-empty string");
  }

  return parsed as TerminalGatewayControlClientMessage | WorkspaceGatewayControlClientMessage;
}

function parseStreamClientMessage(
  payload: string,
): TerminalGatewayStreamClientMessage | WorkspaceGatewayStreamClientMessage {
  const parsed = decodeGatewayPayload<
    Partial<TerminalGatewayStreamClientMessage | WorkspaceGatewayStreamClientMessage>
  >(payload);
  if (typeof parsed.type !== "string" || parsed.type.length === 0) {
    throw new Error("Gateway stream message type must be a non-empty string");
  }

  if (typeof parsed.subscriptionId !== "string" || parsed.subscriptionId.length === 0) {
    throw new Error("Gateway stream subscriptionId must be a non-empty string");
  }

  if (parsed.type === "stream_subscribe_session_state") {
    if (typeof parsed.sessionId !== "string" || parsed.sessionId.length === 0) {
      throw new Error("Gateway stream subscribe requires a non-empty sessionId");
    }

    return parsed as TerminalGatewayStreamClientMessage;
  }

  if (parsed.type === "stream_unsubscribe_session_state") {
    if (typeof parsed.sessionId !== "string" || parsed.sessionId.length === 0) {
      throw new Error("Gateway stream unsubscribe requires a non-empty sessionId");
    }

    return parsed as TerminalGatewayStreamClientMessage;
  }

  if (parsed.type === "workspace_subscribe") {
    if (typeof parsed.sessionId !== "string" || parsed.sessionId.length === 0) {
      throw new Error("Workspace stream subscribe requires a non-empty sessionId");
    }

    return parsed as WorkspaceGatewayStreamClientMessage;
  }

  if (parsed.type === "workspace_unsubscribe") {
    return parsed as WorkspaceGatewayStreamClientMessage;
  }

  throw new Error("Unsupported gateway stream message");
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

function assertNever(value: never): never {
  throw new Error(`Unsupported gateway stream message: ${JSON.stringify(value)}`);
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

function isWorkspaceControlMessage(
  message: TerminalGatewayControlClientMessage | WorkspaceGatewayControlClientMessage,
): message is WorkspaceGatewayControlClientMessage {
  return message.method.startsWith("workspace_");
}

function toSafeSequenceNumber(value: bigint): number {
  if (value > BigInt(Number.MAX_SAFE_INTEGER) || value < BigInt(Number.MIN_SAFE_INTEGER)) {
    throw new Error(`Workspace screen sequence ${value.toString()} exceeds safe integer range`);
  }

  return Number(value);
}
