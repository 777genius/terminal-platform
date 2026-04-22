import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  BackendKind,
  CreateSessionRequest,
  DeleteSavedSessionResult,
  DiscoveredSession,
  Handshake,
  MuxCommand,
  MuxCommandResult,
  PaneId,
  PruneSavedSessionsResult,
  RestoredSession,
  SavedSessionRecord,
  SavedSessionSummary,
  ScreenDelta,
  ScreenSnapshot,
  SessionId,
  SessionRoute,
  SessionSummary,
  SubscriptionEvent,
  SubscriptionMeta,
  SubscriptionSpec,
  TopologySnapshot,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSubscription, WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";
import { WorkspaceError } from "@terminal-platform/workspace-contracts";

import { decodeWorkspaceWebSocketPayload, encodeWorkspaceWebSocketPayload } from "./json-codec.js";
import type {
  WorkspaceGatewayControlClientMessage,
  WorkspaceGatewayControlRequestMap,
  WorkspaceGatewayControlServerResponse,
  WorkspaceGatewayStreamClientMessage,
  WorkspaceGatewayStreamServerMessage,
} from "./protocol.js";

const INITIAL_CONNECT_MAX_ATTEMPTS = 6;
const RECONNECT_BACKOFF_MS = [100, 200, 400, 800, 1_600, 2_000] as const;

export interface CreateWorkspaceWebSocketTransportOptions {
  controlUrl: string;
  streamUrl?: string;
  protocols?: string[];
  webSocketFactory?: (
    url: string,
    protocols?: string[],
  ) => WebSocket;
}

interface Deferred<T> {
  promise: Promise<T>;
  resolve(value: T | PromiseLike<T>): void;
  reject(reason?: unknown): void;
}

interface PendingControlRequest<RecordKey extends keyof WorkspaceGatewayControlRequestMap> {
  method: RecordKey;
  resolve(value: WorkspaceGatewayControlRequestMap[RecordKey]["response"]): void;
  reject(error: Error): void;
}

interface SubscriptionRecord {
  requestedId: string;
  sessionId: SessionId;
  spec: SubscriptionSpec;
  active: boolean;
  disposed: boolean;
  ackSettled: boolean;
  meta: SubscriptionMeta | null;
  ack: Deferred<SubscriptionMeta>;
  queue: SubscriptionEvent[];
  waiters: Deferred<SubscriptionEvent | null>[];
  closed: Deferred<void>;
}

export function createWorkspaceWebSocketTransport(
  options: CreateWorkspaceWebSocketTransportOptions,
): WorkspaceTransportClient {
  return new WorkspaceWebSocketTransport(options);
}

export { decodeWorkspaceWebSocketPayload, encodeWorkspaceWebSocketPayload } from "./json-codec.js";
export type {
  WorkspaceGatewayControlClientMessage,
  WorkspaceGatewayControlMethod,
  WorkspaceGatewayControlRequestMap,
  WorkspaceGatewayControlServerResponse,
  WorkspaceGatewayErrorEnvelope,
  WorkspaceGatewayStreamClientMessage,
  WorkspaceGatewayStreamServerMessage,
} from "./protocol.js";

class WorkspaceWebSocketTransport implements WorkspaceTransportClient {
  readonly #controlUrl: string;
  readonly #streamUrl: string;
  readonly #protocols: string[] | undefined;
  readonly #webSocketFactory: (
    url: string,
    protocols?: string[],
  ) => WebSocket;
  #closed = false;
  #controlSocket: WebSocket | null = null;
  #controlConnectPromise: Promise<WebSocket> | null = null;
  #streamSocket: WebSocket | null = null;
  #streamConnectPromise: Promise<WebSocket> | null = null;
  #streamReconnectLoopPromise: Promise<void> | null = null;
  #streamReconnectTimer: ReturnType<typeof setTimeout> | null = null;
  readonly #pendingControlRequests = new Map<
    string,
    PendingControlRequest<keyof WorkspaceGatewayControlRequestMap>
  >();
  readonly #subscriptions = new Map<string, SubscriptionRecord>();

  constructor(options: CreateWorkspaceWebSocketTransportOptions) {
    this.#controlUrl = options.controlUrl;
    this.#streamUrl = options.streamUrl ?? deriveStreamUrl(options.controlUrl);
    this.#protocols = options.protocols;
    this.#webSocketFactory = options.webSocketFactory ?? defaultWebSocketFactory;
  }

  async handshake(): Promise<Handshake> {
    return this.request("workspace_handshake", undefined);
  }

  async listSessions(): Promise<SessionSummary[]> {
    return this.request("workspace_list_sessions", undefined);
  }

  async listSavedSessions(): Promise<SavedSessionSummary[]> {
    return this.request("workspace_list_saved_sessions", undefined);
  }

  async discoverSessions(backend: BackendKind): Promise<DiscoveredSession[]> {
    return this.request("workspace_discover_sessions", { backend });
  }

  async getBackendCapabilities(backend: BackendKind): Promise<BackendCapabilitiesInfo> {
    return this.request("workspace_backend_capabilities", { backend });
  }

  async createSession(backend: BackendKind, request: CreateSessionRequest): Promise<SessionSummary> {
    if (backend !== "native") {
      throw unsupportedCreateBackend(backend);
    }

    return this.request("workspace_create_session", { backend, request });
  }

  async importSession(route: SessionRoute, title?: string | null): Promise<SessionSummary> {
    return this.request("workspace_import_session", { route, title: title ?? null });
  }

  async getSavedSession(sessionId: SessionId): Promise<SavedSessionRecord> {
    return this.request("workspace_saved_session", { sessionId });
  }

  async deleteSavedSession(sessionId: SessionId): Promise<DeleteSavedSessionResult> {
    return this.request("workspace_delete_saved_session", { sessionId });
  }

  async pruneSavedSessions(keepLatest: number): Promise<PruneSavedSessionsResult> {
    return this.request("workspace_prune_saved_sessions", { keepLatest });
  }

  async restoreSavedSession(sessionId: SessionId): Promise<RestoredSession> {
    return this.request("workspace_restore_saved_session", { sessionId });
  }

  async attachSession(sessionId: SessionId): Promise<AttachedSession> {
    return this.request("workspace_attach_session", { sessionId });
  }

  async getTopologySnapshot(sessionId: SessionId): Promise<TopologySnapshot> {
    return this.request("workspace_topology_snapshot", { sessionId });
  }

  async getScreenSnapshot(sessionId: SessionId, paneId: PaneId): Promise<ScreenSnapshot> {
    return this.request("workspace_screen_snapshot", { sessionId, paneId });
  }

  async getScreenDelta(
    sessionId: SessionId,
    paneId: PaneId,
    fromSequence: bigint,
  ): Promise<ScreenDelta> {
    return this.request("workspace_screen_delta", {
      sessionId,
      paneId,
      fromSequence,
    });
  }

  async dispatchMuxCommand(sessionId: SessionId, command: MuxCommand): Promise<MuxCommandResult> {
    return this.request("workspace_dispatch_mux_command", { sessionId, command });
  }

  async openSubscription(sessionId: SessionId, spec: SubscriptionSpec): Promise<WorkspaceSubscription> {
    this.assertOpen();
    const requestedId = createSubscriptionId();
    const record: SubscriptionRecord = {
      requestedId,
      sessionId,
      spec,
      active: false,
      disposed: false,
      ackSettled: false,
      meta: null,
      ack: createDeferred<SubscriptionMeta>(),
      queue: [],
      waiters: [],
      closed: createDeferred<void>(),
    };
    this.#subscriptions.set(requestedId, record);

    try {
      const socket = await this.ensureStreamConnectedWithRetry(INITIAL_CONNECT_MAX_ATTEMPTS);
      this.sendStream(socket, {
        type: "workspace_subscribe",
        subscriptionId: requestedId,
        sessionId,
        spec,
      });
      record.meta = await record.ack.promise;
    } catch (error) {
      this.finalizeSubscription(record, {
        notifyWaitersWithNull: true,
        rejectAckWith: toError(error),
      });
      throw error;
    }

    return new WebSocketWorkspaceSubscription(record, () => this.disposeSubscription(requestedId));
  }

  async close(): Promise<void> {
    if (this.#closed) {
      return;
    }

    this.#closed = true;
    this.clearStreamReconnectTimer();
    this.#streamReconnectLoopPromise = null;
    this.rejectAllControlRequests(new Error("workspace websocket transport closed"));

    const closeOperations = [
      closeSocket(this.#controlSocket),
      closeSocket(this.#streamSocket),
    ];
    this.#controlSocket = null;
    this.#controlConnectPromise = null;
    this.#streamSocket = null;
    this.#streamConnectPromise = null;

    for (const record of this.#subscriptions.values()) {
      this.finalizeSubscription(record, {
        notifyWaitersWithNull: true,
        rejectAckWith: new Error("workspace websocket transport closed"),
      });
    }

    await Promise.allSettled(closeOperations);
  }

  private async request<RecordKey extends keyof WorkspaceGatewayControlRequestMap>(
    method: RecordKey,
    payload: WorkspaceGatewayControlRequestMap[RecordKey]["payload"],
  ): Promise<WorkspaceGatewayControlRequestMap[RecordKey]["response"]> {
    this.assertOpen();
    const socket = await this.ensureControlConnected();
    const requestId = createSubscriptionId();

    return await new Promise<WorkspaceGatewayControlRequestMap[RecordKey]["response"]>((resolve, reject) => {
      this.#pendingControlRequests.set(requestId, {
        method,
        resolve,
        reject,
      });

      const envelope = {
        type: "request",
        requestId,
        method,
        payload,
      } as WorkspaceGatewayControlClientMessage;

      socket.send(encodeWorkspaceWebSocketPayload(envelope));
    });
  }

  private async ensureControlConnected(): Promise<WebSocket> {
    if (this.#controlSocket?.readyState === WebSocket.OPEN) {
      return this.#controlSocket;
    }

    this.#controlConnectPromise ??= new Promise<WebSocket>((resolve, reject) => {
      const socket = this.#webSocketFactory(this.#controlUrl, this.#protocols);
      const cleanup = () => {
        socket.removeEventListener("open", onOpen);
        socket.removeEventListener("error", onError);
      };
      const onOpen = () => {
        cleanup();
        this.#controlSocket = socket;
        this.#controlConnectPromise = null;
        resolve(socket);
      };
      const onError = () => {
        cleanup();
        this.#controlConnectPromise = null;
        reject(new Error("Failed to connect to workspace control plane"));
      };

      socket.addEventListener("open", onOpen, { once: true });
      socket.addEventListener("error", onError, { once: true });
      socket.addEventListener("message", (event) => {
        this.handleControlMessage(event.data.toString());
      });
      socket.addEventListener("close", () => {
        if (this.#controlSocket === socket) {
          this.#controlSocket = null;
        }
        this.rejectAllControlRequests(new Error("Workspace control plane connection closed"));
      });
    });

    return this.#controlConnectPromise;
  }

  private handleControlMessage(raw: string): void {
    const message = decodeWorkspaceWebSocketPayload<WorkspaceGatewayControlServerResponse>(raw);
    if (message.type !== "response") {
      return;
    }

    const request = this.#pendingControlRequests.get(message.requestId);
    if (!request) {
      return;
    }

    this.#pendingControlRequests.delete(message.requestId);
    if (!message.ok) {
      request.reject(toGatewayError(message.error));
      return;
    }

    request.resolve(message.result as WorkspaceGatewayControlRequestMap[typeof request.method]["response"]);
  }

  private rejectAllControlRequests(error: Error): void {
    for (const pending of this.#pendingControlRequests.values()) {
      pending.reject(error);
    }
    this.#pendingControlRequests.clear();
  }

  private async ensureStreamConnectedWithRetry(maxAttempts: number): Promise<WebSocket> {
    let attempt = 0;
    let lastError: Error | null = null;

    while (!this.#closed && attempt < maxAttempts) {
      try {
        return await this.ensureStreamConnected();
      } catch (error) {
        lastError = toError(error);
        attempt += 1;
        if (attempt >= maxAttempts) {
          break;
        }
        await this.waitBeforeRetry(attempt);
      }
    }

    throw lastError ?? new Error("Failed to connect to workspace stream plane");
  }

  private async ensureStreamConnected(): Promise<WebSocket> {
    if (this.#streamSocket?.readyState === WebSocket.OPEN) {
      return this.#streamSocket;
    }

    this.#streamConnectPromise ??= new Promise<WebSocket>((resolve, reject) => {
      const socket = this.#webSocketFactory(this.#streamUrl, this.#protocols);
      const cleanup = () => {
        socket.removeEventListener("open", onOpen);
        socket.removeEventListener("error", onError);
      };
      const onOpen = () => {
        cleanup();
        this.#streamSocket = socket;
        this.#streamConnectPromise = null;
        resolve(socket);
      };
      const onError = () => {
        cleanup();
        this.#streamConnectPromise = null;
        reject(new Error("Failed to connect to workspace stream plane"));
      };

      socket.addEventListener("open", onOpen, { once: true });
      socket.addEventListener("error", onError, { once: true });
      socket.addEventListener("message", (event) => {
        this.handleStreamMessage(event.data.toString());
      });
      socket.addEventListener("close", () => {
        this.handleStreamSocketClosed(socket);
      });
    });

    return this.#streamConnectPromise;
  }

  private handleStreamSocketClosed(socket: WebSocket): void {
    if (this.#streamSocket !== socket) {
      return;
    }

    this.#streamSocket = null;
    this.#streamConnectPromise = null;

    if (this.#closed) {
      return;
    }

    for (const record of this.#subscriptions.values()) {
      record.active = false;
    }

    if (this.#subscriptions.size > 0) {
      this.ensureStreamReconnectLoop();
    }
  }

  private ensureStreamReconnectLoop(): void {
    if (this.#streamReconnectLoopPromise || this.#closed || this.#subscriptions.size === 0) {
      return;
    }

    this.#streamReconnectLoopPromise = this.runStreamReconnectLoop().finally(() => {
      this.#streamReconnectLoopPromise = null;
    });
  }

  private async runStreamReconnectLoop(): Promise<void> {
    let attempt = 0;

    while (!this.#closed && this.#subscriptions.size > 0 && !this.#streamSocket) {
      try {
        const socket = await this.ensureStreamConnected();
        for (const record of this.#subscriptions.values()) {
          this.sendStream(socket, {
            type: "workspace_subscribe",
            subscriptionId: record.requestedId,
            sessionId: record.sessionId,
            spec: record.spec,
          });
        }
        return;
      } catch {
        attempt += 1;
        await this.waitBeforeRetry(attempt);
      }
    }
  }

  private handleStreamMessage(raw: string): void {
    const message = decodeWorkspaceWebSocketPayload<WorkspaceGatewayStreamServerMessage>(raw);
    const record = this.#subscriptions.get(message.subscriptionId);
    if (!record) {
      return;
    }

    switch (message.type) {
      case "workspace_subscription_ack":
        record.active = true;
        record.meta = message.meta;
        if (!record.ackSettled) {
          record.ackSettled = true;
          record.ack.resolve(message.meta);
        }
        return;
      case "workspace_subscription_rejected":
        this.finalizeSubscription(record, {
          notifyWaitersWithNull: true,
          rejectAckWith: toGatewayError(message.error),
        });
        return;
      case "workspace_subscription_event":
        this.pushSubscriptionEvent(record, message.event);
        return;
      case "workspace_subscription_error":
        this.finalizeSubscription(record, {
          notifyWaitersWithNull: true,
          rejectAckWith: toGatewayError(message.error),
        });
        return;
      case "workspace_subscription_closed":
        this.finalizeSubscription(record, {
          notifyWaitersWithNull: true,
        });
        return;
      default:
        assertNever(message);
    }
  }

  private pushSubscriptionEvent(record: SubscriptionRecord, event: SubscriptionEvent): void {
    const waiter = record.waiters.shift();
    if (waiter) {
      waiter.resolve(event);
      return;
    }

    record.queue.push(event);
  }

  private async disposeSubscription(subscriptionId: string): Promise<void> {
    const record = this.#subscriptions.get(subscriptionId);
    if (!record) {
      return;
    }

    const socket = this.#streamSocket;
    if (socket?.readyState === WebSocket.OPEN) {
      this.sendStream(socket, {
        type: "workspace_unsubscribe",
        subscriptionId,
      });
      await record.closed.promise;
      return;
    }

    this.finalizeSubscription(record, {
      notifyWaitersWithNull: true,
    });
  }

  private finalizeSubscription(
    record: SubscriptionRecord,
    options: {
      notifyWaitersWithNull: boolean;
      rejectAckWith?: Error;
    },
  ): void {
    if (this.#subscriptions.get(record.requestedId) !== record) {
      return;
    }

    this.#subscriptions.delete(record.requestedId);
    record.active = false;
    record.disposed = true;

    if (!record.ackSettled) {
      record.ackSettled = true;
      record.ack.reject(
        options.rejectAckWith
          ?? new Error(`Workspace subscription ${record.requestedId} closed before activation`),
      );
    }

    if (options.notifyWaitersWithNull) {
      for (const waiter of record.waiters) {
        waiter.resolve(null);
      }
      record.waiters = [];
    }

    record.closed.resolve();
  }

  private async waitBeforeRetry(attempt: number): Promise<void> {
    const backoffMs = RECONNECT_BACKOFF_MS[Math.min(attempt - 1, RECONNECT_BACKOFF_MS.length - 1)];

    await new Promise<void>((resolve) => {
      this.clearStreamReconnectTimer();
      this.#streamReconnectTimer = setTimeout(() => {
        this.#streamReconnectTimer = null;
        resolve();
      }, backoffMs);
    });
  }

  private clearStreamReconnectTimer(): void {
    if (this.#streamReconnectTimer) {
      clearTimeout(this.#streamReconnectTimer);
      this.#streamReconnectTimer = null;
    }
  }

  private sendStream(socket: WebSocket, message: WorkspaceGatewayStreamClientMessage): void {
    socket.send(encodeWorkspaceWebSocketPayload(message));
  }

  private assertOpen(): void {
    if (this.#closed) {
      throw new WorkspaceError({
        code: "disposed",
        message: "workspace websocket transport is closed",
        recoverable: false,
      });
    }
  }
}

class WebSocketWorkspaceSubscription implements WorkspaceSubscription {
  readonly #record: SubscriptionRecord;
  readonly #disposeRecord: () => Promise<void>;

  constructor(record: SubscriptionRecord, disposeRecord: () => Promise<void>) {
    this.#record = record;
    this.#disposeRecord = disposeRecord;
  }

  meta(): SubscriptionMeta {
    if (!this.#record.meta) {
      throw new Error(`Workspace subscription ${this.#record.requestedId} is not active yet`);
    }

    return this.#record.meta;
  }

  async nextEvent(): Promise<SubscriptionEvent | null> {
    const queued = this.#record.queue.shift();
    if (queued) {
      return queued;
    }

    if (this.#record.disposed) {
      return null;
    }

    const deferred = createDeferred<SubscriptionEvent | null>();
    this.#record.waiters.push(deferred);
    return deferred.promise;
  }

  async close(): Promise<void> {
    await this.#disposeRecord();
  }
}

function deriveStreamUrl(controlUrl: string): string {
  const url = new URL(controlUrl);
  if (url.pathname === "/terminal-gateway" || url.pathname === "/terminal-gateway/control") {
    url.pathname = "/terminal-gateway/stream";
    return url.toString();
  }

  throw new Error(`Unsupported workspace control URL path: ${url.pathname}`);
}

function defaultWebSocketFactory(url: string, protocols?: string[]): WebSocket {
  if (typeof WebSocket !== "function") {
    throw new Error("Global WebSocket is not available. Provide webSocketFactory explicitly.");
  }

  return new WebSocket(url, protocols);
}

function createSubscriptionId(): string {
  if (typeof globalThis.crypto?.randomUUID === "function") {
    return globalThis.crypto.randomUUID();
  }

  throw new Error("crypto.randomUUID is not available");
}

function createDeferred<T>(): Deferred<T> {
  let resolve!: Deferred<T>["resolve"];
  let reject!: Deferred<T>["reject"];
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve;
    reject = innerReject;
  });

  return {
    promise,
    resolve,
    reject,
  };
}

function toError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

function toGatewayError(error: { message: string; code?: string }): WorkspaceError {
  return new WorkspaceError({
    code: "transport_failed",
    message: error.message,
    recoverable: true,
  });
}

function unsupportedCreateBackend(backend: BackendKind): WorkspaceError {
  return new WorkspaceError({
    code: "unsupported_capability",
    message: `workspace websocket gateway does not support createSession for backend ${backend}`,
    recoverable: false,
  });
}

async function closeSocket(socket: WebSocket | null): Promise<void> {
  if (!socket) {
    return;
  }

  if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
    await new Promise<void>((resolve) => {
      socket.addEventListener(
        "close",
        () => {
          resolve();
        },
        { once: true },
      );
      socket.close(1000, "Disposed");
    });
  }
}

function assertNever(value: never): never {
  throw new Error(`Unsupported workspace stream message: ${JSON.stringify(value)}`);
}
