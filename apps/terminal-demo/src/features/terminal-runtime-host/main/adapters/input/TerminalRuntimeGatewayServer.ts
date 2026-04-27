import { once } from "node:events";
import { randomUUID } from "node:crypto";
import type { IncomingMessage } from "node:http";
import { WebSocketServer, WebSocket } from "ws";
import type {
  TerminalBackendKind,
  TerminalCreateNativeSessionInput,
  TerminalDiscoveredSession,
  TerminalImportSessionInput,
  TerminalMuxCommand,
  TerminalShellLaunchSpec,
  TerminalSplitDirection,
} from "@features/terminal-workspace-kernel/contracts";
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

type GatewayPayloadRecord = Record<string, unknown>;
type TerminalPlatformClient = Awaited<ReturnType<TerminalPlatformClientProvider["getClient"]>>;
type TerminalPlatformCreateSessionRequest = Parameters<TerminalPlatformClient["createNativeSession"]>[0];
type TerminalPlatformSessionRoute = Parameters<TerminalPlatformClient["importSession"]>[0];
type TerminalPlatformMuxCommand = Parameters<TerminalPlatformClient["dispatchMuxCommand"]>[1];

const TERMINAL_BACKEND_KINDS = new Set<TerminalBackendKind>(["native", "tmux", "zellij"]);
const TERMINAL_MUX_COMMAND_KINDS = new Set<TerminalMuxCommand["kind"]>([
  "split_pane",
  "close_pane",
  "focus_pane",
  "resize_pane",
  "new_tab",
  "close_tab",
  "focus_tab",
  "rename_tab",
  "send_input",
  "send_paste",
  "detach",
  "save_session",
  "override_layout",
]);
const TERMINAL_SPLIT_DIRECTIONS = new Set<TerminalSplitDirection>(["horizontal", "vertical"]);

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
        const backend = readBackendPayload(payload);
        this.clearImportHandlesForBackend(connection, backend);
        const discovered = await this.#controlService.discoverSessions(backend);
        return discovered.map((session) => this.registerImportHandle(connection, session));
      }
      case "backend_capabilities":
        return this.#controlService.backendCapabilities(readBackendPayload(payload));
      case "create_native_session":
        return this.#controlService.createNativeSession(readCreateNativeSessionPayload(payload));
      case "import_session": {
        const importPayload = readImportSessionPayload(payload);
        const discovered = connection.importHandles.get(importPayload.importHandle);
        if (!discovered) {
          throw new Error(`Unknown import handle ${importPayload.importHandle}`);
        }

        return this.#controlService.importSession(
          importPayload.title
            ? {
                route: discovered.route,
                title: importPayload.title,
              }
            : {
                route: discovered.route,
              },
        );
      }
      case "restore_saved_session":
        return this.#controlService.restoreSavedSession(readStringPayload(payload, "sessionId"));
      case "delete_saved_session":
        return this.#controlService.deleteSavedSession(readStringPayload(payload, "sessionId"));
      case "dispatch_mux_command":
        return this.#controlService.dispatchMuxCommand(
          readStringPayload(payload, "sessionId"),
          readMuxCommandPayload(payload),
        );
      case "workspace_handshake": {
        const client = await this.#clientProvider.getClient();
        return (await client.handshakeInfo()).handshake;
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
        return client.discoverSessions(readBackendPayload(payload));
      }
      case "workspace_backend_capabilities": {
        const client = await this.#clientProvider.getClient();
        return client.backendCapabilities(readBackendPayload(payload));
      }
      case "workspace_create_session": {
        const backend = readBackendPayload(payload);
        if (backend !== "native") {
          throw new Error(`Unsupported backend ${backend}`);
        }

        const client = await this.#clientProvider.getClient();
        return client.createNativeSession(readOptionalObjectPayload<TerminalPlatformCreateSessionRequest>(
          payload,
          "request",
        ));
      }
      case "workspace_import_session": {
        const client = await this.#clientProvider.getClient();
        return client.importSession(
          readObjectPayload<TerminalPlatformSessionRoute>(payload, "route"),
          readOptionalStringPayload(payload, "title") ?? null,
        );
      }
      case "workspace_saved_session": {
        const client = await this.#clientProvider.getClient();
        return client.savedSession(readStringPayload(payload, "sessionId"));
      }
      case "workspace_prune_saved_sessions": {
        const client = await this.#clientProvider.getClient();
        return client.pruneSavedSessions(readNumberPayload(payload, "keepLatest"));
      }
      case "workspace_restore_saved_session": {
        const client = await this.#clientProvider.getClient();
        return client.restoreSavedSession(readStringPayload(payload, "sessionId"));
      }
      case "workspace_delete_saved_session": {
        const client = await this.#clientProvider.getClient();
        return client.deleteSavedSession(readStringPayload(payload, "sessionId"));
      }
      case "workspace_attach_session": {
        const client = await this.#clientProvider.getClient();
        return client.attachSession(readStringPayload(payload, "sessionId"));
      }
      case "workspace_topology_snapshot": {
        const client = await this.#clientProvider.getClient();
        return client.topologySnapshot(readStringPayload(payload, "sessionId"));
      }
      case "workspace_screen_snapshot": {
        const client = await this.#clientProvider.getClient();
        return client.screenSnapshot(
          readStringPayload(payload, "sessionId"),
          readStringPayload(payload, "paneId"),
        );
      }
      case "workspace_screen_delta": {
        const client = await this.#clientProvider.getClient();
        return client.screenDelta(
          readStringPayload(payload, "sessionId"),
          readStringPayload(payload, "paneId"),
          readNumberPayload(payload, "fromSequence"),
        );
      }
      case "workspace_dispatch_mux_command": {
        const client = await this.#clientProvider.getClient();
        return client.dispatchMuxCommand(
          readStringPayload(payload, "sessionId"),
          readObjectPayload<TerminalPlatformMuxCommand>(payload, "command"),
        );
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
  const parsed = decodeGatewayPayload(payload);

  if (!isGatewayPayloadRecord(parsed) || parsed.type !== "request") {
    throw new Error("Gateway control payload must be a request envelope");
  }

  if (typeof parsed.requestId !== "string" || parsed.requestId.length === 0) {
    throw new Error("Gateway control requestId must be a non-empty string");
  }

  if (typeof parsed.method !== "string" || parsed.method.length === 0) {
    throw new Error("Gateway control method must be a non-empty string");
  }

  return {
    type: "request",
    requestId: parsed.requestId,
    method: parsed.method,
    payload: parsed.payload,
  };
}

function parseStreamClientMessage(payload: string): TerminalGatewayStreamClientMessage {
  const parsed = decodeGatewayPayload(payload);
  if (!isGatewayPayloadRecord(parsed)) {
    throw new Error("Gateway stream message must be an object");
  }

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

  return {
    type: parsed.type,
    subscriptionId: parsed.subscriptionId,
    sessionId: parsed.sessionId,
  };
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

function asGatewayPayload(value: unknown): GatewayPayloadRecord {
  if (!isGatewayPayloadRecord(value)) {
    return {};
  }

  return value;
}

function readBackendPayload(payload: GatewayPayloadRecord, key = "backend"): TerminalBackendKind {
  const value = readStringPayload(payload, key);
  if (!TERMINAL_BACKEND_KINDS.has(value as TerminalBackendKind)) {
    throw new Error(`Gateway payload ${key} must be one of: native, tmux, zellij`);
  }

  return value as TerminalBackendKind;
}

function readStringPayload(payload: GatewayPayloadRecord, key: string): string {
  const value = payload[key];
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`Gateway payload ${key} must be a non-empty string`);
  }

  return value;
}

function readOptionalStringPayload(payload: GatewayPayloadRecord, key: string): string | undefined {
  const value = payload[key];
  if (value == null) {
    return undefined;
  }

  if (typeof value !== "string") {
    throw new Error(`Gateway payload ${key} must be a string when provided`);
  }

  const normalized = value.trim();
  return normalized.length > 0 ? value : undefined;
}

function readNullableStringPayload(payload: GatewayPayloadRecord, key: string): string | null {
  const value = payload[key];
  if (value == null) {
    return null;
  }

  if (typeof value !== "string") {
    throw new Error(`Gateway payload ${key} must be a string or null`);
  }

  return value;
}

function readNumberPayload(payload: GatewayPayloadRecord, key: string): number {
  const value = payload[key];
  const parsed = typeof value === "bigint"
    ? Number(value)
    : typeof value === "string" || typeof value === "number"
      ? Number(value)
      : Number.NaN;

  if (!Number.isFinite(parsed)) {
    throw new Error(`Gateway payload ${key} must be a finite number`);
  }

  return parsed;
}

function readIntegerPayload(payload: GatewayPayloadRecord, key: string): number {
  const value = readNumberPayload(payload, key);
  if (!Number.isInteger(value)) {
    throw new Error(`Gateway payload ${key} must be an integer`);
  }

  return value;
}

function readObjectPayload<T extends object>(payload: GatewayPayloadRecord, key: string): T {
  const value = payload[key];
  if (!isGatewayPayloadRecord(value)) {
    throw new Error(`Gateway payload ${key} must be an object`);
  }

  return value as T;
}

function readOptionalObjectPayload<T extends object>(
  payload: GatewayPayloadRecord,
  key: string,
): T | undefined {
  const value = payload[key];
  if (value == null) {
    return undefined;
  }

  if (!isGatewayPayloadRecord(value)) {
    throw new Error(`Gateway payload ${key} must be an object when provided`);
  }

  return value as T;
}

function readCreateNativeSessionPayload(payload: GatewayPayloadRecord): TerminalCreateNativeSessionInput {
  const title = readOptionalStringPayload(payload, "title");
  const launch = readOptionalLaunchSpecPayload(payload, "launch");

  return {
    ...(title ? { title } : {}),
    ...(launch ? { launch } : {}),
  };
}

function readImportSessionPayload(payload: GatewayPayloadRecord): TerminalImportSessionInput {
  const importHandle = readStringPayload(payload, "importHandle");
  const title = readOptionalStringPayload(payload, "title");

  return {
    importHandle,
    ...(title ? { title } : {}),
  };
}

function readOptionalLaunchSpecPayload(
  payload: GatewayPayloadRecord,
  key: string,
): TerminalShellLaunchSpec | undefined {
  const value = payload[key];
  if (value == null) {
    return undefined;
  }

  if (!isGatewayPayloadRecord(value)) {
    throw new Error(`Gateway payload ${key} must be an object when provided`);
  }

  const program = readStringPayload(value, "program");
  const args = value.args;
  if (!Array.isArray(args) || args.some((entry) => typeof entry !== "string")) {
    throw new Error(`Gateway payload ${key}.args must be a string array`);
  }

  const cwd = readOptionalStringPayload(value, "cwd");
  return {
    program,
    args,
    ...(cwd ? { cwd } : {}),
  };
}

function readMuxCommandPayload(payload: GatewayPayloadRecord): TerminalMuxCommand {
  const command = readObjectPayload<GatewayPayloadRecord>(payload, "command");
  const kind = readMuxCommandKind(command);

  switch (kind) {
    case "split_pane":
      return {
        kind,
        pane_id: readStringPayload(command, "pane_id"),
        direction: readSplitDirectionPayload(command),
      };
    case "close_pane":
    case "focus_pane":
      return {
        kind,
        pane_id: readStringPayload(command, "pane_id"),
      };
    case "resize_pane":
      return {
        kind,
        pane_id: readStringPayload(command, "pane_id"),
        rows: readIntegerPayload(command, "rows"),
        cols: readIntegerPayload(command, "cols"),
      };
    case "new_tab":
      return {
        kind,
        title: readNullableStringPayload(command, "title"),
      };
    case "close_tab":
    case "focus_tab":
      return {
        kind,
        tab_id: readStringPayload(command, "tab_id"),
      };
    case "rename_tab":
      return {
        kind,
        tab_id: readStringPayload(command, "tab_id"),
        title: readStringPayload(command, "title"),
      };
    case "send_input":
    case "send_paste":
      return {
        kind,
        pane_id: readStringPayload(command, "pane_id"),
        data: readStringPayload(command, "data"),
      };
    case "detach":
    case "save_session":
      return { kind };
    case "override_layout":
      return {
        kind,
        tab_id: readStringPayload(command, "tab_id"),
        root: readObjectPayload<Extract<TerminalMuxCommand, { kind: "override_layout" }>["root"]>(
          command,
          "root",
        ),
      };
  }
}

function readMuxCommandKind(payload: GatewayPayloadRecord): TerminalMuxCommand["kind"] {
  const kind = readStringPayload(payload, "kind");
  if (!TERMINAL_MUX_COMMAND_KINDS.has(kind as TerminalMuxCommand["kind"])) {
    throw new Error("Gateway payload command.kind is unsupported");
  }

  return kind as TerminalMuxCommand["kind"];
}

function readSplitDirectionPayload(payload: GatewayPayloadRecord): TerminalSplitDirection {
  const direction = readStringPayload(payload, "direction");
  if (!TERMINAL_SPLIT_DIRECTIONS.has(direction as TerminalSplitDirection)) {
    throw new Error("Gateway payload command.direction is unsupported");
  }

  return direction as TerminalSplitDirection;
}

function isGatewayPayloadRecord(value: unknown): value is GatewayPayloadRecord {
  return Boolean(value)
    && typeof value === "object"
    && !Array.isArray(value);
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

function decodeGatewayPayload(raw: string): unknown {
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
  }) as unknown;
}
