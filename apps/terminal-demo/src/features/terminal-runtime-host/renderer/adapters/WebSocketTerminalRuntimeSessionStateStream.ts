import type {
  TerminalGatewayStreamClientMessage,
  TerminalGatewayStreamServerMessage,
} from "../../contracts/terminal-gateway-protocol.js";
import type { TerminalSessionState } from "@features/terminal-workspace-kernel/contracts";
import type {
  TerminalWorkspaceSessionStateStreamPort,
  TerminalWorkspaceSessionStateSubscription,
  TerminalWorkspaceSessionStreamHealth,
} from "../../core/application/index.js";

const INITIAL_CONNECT_MAX_ATTEMPTS = 6;
const RECONNECT_BACKOFF_MS = [100, 200, 400, 800, 1_600, 2_000] as const;

interface Deferred<T> {
  promise: Promise<T>;
  resolve(value: T | PromiseLike<T>): void;
  reject(reason?: unknown): void;
}

interface SessionStateSubscriptionRecord {
  subscriptionId: string;
  sessionId: string;
  active: boolean;
  subscribeSettled: boolean;
  onState(state: TerminalSessionState): void;
  onStatusChange: ((health: TerminalWorkspaceSessionStreamHealth) => void) | undefined;
  onError: ((error: Error) => void) | undefined;
  onClosed: (() => void) | undefined;
  subscribed: Deferred<void>;
  closed: Deferred<void>;
}

export class WebSocketTerminalRuntimeSessionStateStream implements TerminalWorkspaceSessionStateStreamPort {
  readonly #url: string;
  #socket: WebSocket | null = null;
  #connectPromise: Promise<WebSocket> | null = null;
  #disposed = false;
  #reconnectLoopPromise: Promise<void> | null = null;
  #reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  readonly #subscriptions = new Map<string, SessionStateSubscriptionRecord>();

  constructor(url: string) {
    this.#url = url;
  }

  async subscribeSessionState(
    sessionId: string,
    handlers: {
      onState(state: TerminalSessionState): void;
      onStatusChange?(health: TerminalWorkspaceSessionStreamHealth): void;
      onError?(error: Error): void;
      onClosed?(): void;
    },
  ): Promise<TerminalWorkspaceSessionStateSubscription> {
    if (this.#disposed) {
      throw new Error("Terminal session stream is disposed");
    }

    const subscriptionId = crypto.randomUUID();
    const record: SessionStateSubscriptionRecord = {
      subscriptionId,
      sessionId,
      active: false,
      subscribeSettled: false,
      onState: handlers.onState,
      onStatusChange: handlers.onStatusChange,
      onError: handlers.onError,
      onClosed: handlers.onClosed,
      subscribed: createDeferred<void>(),
      closed: createDeferred<void>(),
    };
    this.#subscriptions.set(subscriptionId, record);

    try {
      this.notifyStatusChange(record, {
        phase: "connecting",
        reconnectAttempts: 0,
        lastError: null,
      });
      const socket = await this.ensureConnectedWithRetry(INITIAL_CONNECT_MAX_ATTEMPTS);
      this.sendSubscribe(socket, record);
      await record.subscribed.promise;
    } catch (error) {
      this.notifyStatusChange(record, {
        phase: "error",
        reconnectAttempts: INITIAL_CONNECT_MAX_ATTEMPTS,
        lastError: toError(error).message,
      });
      this.finalizeSubscription(record, {
        notifyClosed: false,
        rejectPendingWith: toError(error),
      });
      throw error;
    }

    return {
      subscriptionId,
      dispose: async () => {
        await this.disposeSubscription(subscriptionId);
      },
    };
  }

  dispose(): void {
    this.#disposed = true;
    this.clearReconnectTimer();
    this.#reconnectLoopPromise = null;

    if (this.#socket && this.#socket.readyState === WebSocket.OPEN) {
      this.#socket.close(1000, "Disposed");
    }

    this.#socket = null;
    this.#connectPromise = null;

    for (const record of this.#subscriptions.values()) {
      this.finalizeSubscription(record, {
        notifyClosed: false,
        rejectPendingWith: new Error("Terminal session stream disposed"),
      });
    }
  }

  private async disposeSubscription(subscriptionId: string): Promise<void> {
    const record = this.#subscriptions.get(subscriptionId);
    if (!record) {
      return;
    }

    if (this.#socket?.readyState === WebSocket.OPEN) {
      this.send(this.#socket, {
        type: "stream_unsubscribe_session_state",
        subscriptionId,
        sessionId: record.sessionId,
      });
      await record.closed.promise;
      return;
    }

    this.finalizeSubscription(record, {
      notifyClosed: false,
      rejectPendingWith: new Error(`Session stream subscription ${subscriptionId} disposed`),
    });
  }

  private async ensureConnectedWithRetry(maxAttempts: number): Promise<WebSocket> {
    let attempt = 0;
    let lastError: Error | null = null;

    while (!this.#disposed && attempt < maxAttempts) {
      try {
        return await this.ensureConnected();
      } catch (error) {
        lastError = toError(error);
        attempt += 1;
        if (attempt >= maxAttempts) {
          break;
        }
        await this.waitBeforeRetry(attempt);
      }
    }

    throw lastError ?? new Error("Failed to connect to terminal session stream");
  }

  private async ensureConnected(): Promise<WebSocket> {
    if (this.#socket?.readyState === WebSocket.OPEN) {
      return this.#socket;
    }

    this.#connectPromise ??= new Promise<WebSocket>((resolve, reject) => {
      const socket = new WebSocket(this.#url);
      const cleanup = () => {
        socket.removeEventListener("open", onOpen);
        socket.removeEventListener("error", onError);
      };
      const onOpen = () => {
        cleanup();
        this.#socket = socket;
        this.#connectPromise = null;
        resolve(socket);
      };
      const onError = () => {
        cleanup();
        this.#connectPromise = null;
        reject(new Error("Failed to connect to terminal session stream"));
      };

      socket.addEventListener("open", onOpen, { once: true });
      socket.addEventListener("error", onError, { once: true });
      socket.addEventListener("message", (event) => {
        this.handleMessage(event.data.toString());
      });
      socket.addEventListener("close", () => {
        this.handleSocketClosed(socket);
      });
    });

    return this.#connectPromise;
  }

  private handleSocketClosed(socket: WebSocket): void {
    if (this.#socket !== socket) {
      return;
    }

    this.#socket = null;
    this.#connectPromise = null;

    if (this.#disposed) {
      return;
    }

    for (const record of this.#subscriptions.values()) {
      record.active = false;
      this.notifyStatusChange(record, {
        phase: "reconnecting",
        reconnectAttempts: 0,
        lastError: "Terminal session stream connection closed",
      });
    }

    if (this.#subscriptions.size > 0) {
      this.ensureReconnectLoop();
    }
  }

  private ensureReconnectLoop(): void {
    if (this.#reconnectLoopPromise || this.#disposed || this.#subscriptions.size === 0) {
      return;
    }

    // Keep live subscriptions attached to the daemon across transient stream disconnects.
    this.#reconnectLoopPromise = this.runReconnectLoop().finally(() => {
      this.#reconnectLoopPromise = null;
    });
  }

  private async runReconnectLoop(): Promise<void> {
    let attempt = 0;

    while (!this.#disposed && this.#subscriptions.size > 0 && !this.#socket) {
      try {
        const socket = await this.ensureConnected();
        this.resubscribeAll(socket);
        return;
      } catch (error) {
        attempt += 1;
        const nextHealth: TerminalWorkspaceSessionStreamHealth = {
          phase: "reconnecting",
          reconnectAttempts: attempt,
          lastError: toError(error).message,
        };
        for (const record of this.#subscriptions.values()) {
          this.notifyStatusChange(record, nextHealth);
        }
        await this.waitBeforeRetry(attempt);
      }
    }
  }

  private resubscribeAll(socket: WebSocket): void {
    for (const record of this.#subscriptions.values()) {
      this.sendSubscribe(socket, record);
    }
  }

  private sendSubscribe(socket: WebSocket, record: SessionStateSubscriptionRecord): void {
    this.send(socket, {
      type: "stream_subscribe_session_state",
      subscriptionId: record.subscriptionId,
      sessionId: record.sessionId,
    });
  }

  private handleMessage(raw: string): void {
    const message = JSON.parse(raw) as TerminalGatewayStreamServerMessage;
    const record = this.#subscriptions.get(message.subscriptionId);
    if (!record) {
      return;
    }

    switch (message.type) {
      case "stream_subscription_ack":
        record.active = true;
        this.notifyStatusChange(record, {
          phase: "ready",
          reconnectAttempts: 0,
          lastError: null,
        });
        if (!record.subscribeSettled) {
          record.subscribeSettled = true;
          record.subscribed.resolve();
        }
        return;
      case "stream_subscription_rejected":
        this.notifyStatusChange(record, {
          phase: "error",
          reconnectAttempts: 0,
          lastError: message.error.message,
        });
        this.finalizeSubscription(record, {
          notifyClosed: false,
          rejectPendingWith: toError(message.error.message),
        });
        return;
      case "session_state":
        if (record.active) {
          record.onState(message.state);
        }
        return;
      case "subscription_error":
        this.notifyStatusChange(record, {
          phase: "error",
          reconnectAttempts: 0,
          lastError: message.error.message,
        });
        record.onError?.(toError(message.error.message));
        return;
      case "subscription_closed":
        this.finalizeSubscription(record, {
          notifyClosed: true,
        });
        return;
      default:
        assertNever(message);
    }
  }

  private finalizeSubscription(
    record: SessionStateSubscriptionRecord,
    options: {
      notifyClosed: boolean;
      rejectPendingWith?: Error;
    },
  ): void {
    if (this.#subscriptions.get(record.subscriptionId) !== record) {
      return;
    }

    this.#subscriptions.delete(record.subscriptionId);
    record.active = false;

    if (!record.subscribeSettled) {
      record.subscribeSettled = true;
      record.subscribed.reject(
        options.rejectPendingWith
          ?? new Error(`Session stream subscription ${record.subscriptionId} closed before activation`),
      );
    } else if (options.notifyClosed) {
      record.onClosed?.();
    }

    record.closed.resolve();
  }

  private async waitBeforeRetry(attempt: number): Promise<void> {
    const backoffMs = RECONNECT_BACKOFF_MS[Math.min(attempt - 1, RECONNECT_BACKOFF_MS.length - 1)];

    await new Promise<void>((resolve) => {
      this.clearReconnectTimer();
      this.#reconnectTimer = setTimeout(() => {
        this.#reconnectTimer = null;
        resolve();
      }, backoffMs);
    });
  }

  private clearReconnectTimer(): void {
    if (this.#reconnectTimer) {
      clearTimeout(this.#reconnectTimer);
      this.#reconnectTimer = null;
    }
  }

  private send(socket: WebSocket, message: TerminalGatewayStreamClientMessage): void {
    socket.send(JSON.stringify(message));
  }

  private notifyStatusChange(
    record: SessionStateSubscriptionRecord,
    health: TerminalWorkspaceSessionStreamHealth,
  ): void {
    record.onStatusChange?.(health);
  }
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

function assertNever(value: never): never {
  throw new Error(`Unsupported session stream message: ${JSON.stringify(value)}`);
}
