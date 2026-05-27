import type {
  TerminalGatewayControlClientMessage,
  TerminalGatewayControlRequestMap,
  TerminalGatewayControlServerResponse,
} from "../../contracts/terminal-gateway-protocol.js";
import type {
  TerminalBackendCapabilitiesInfo,
  TerminalDiscoveredSession,
  TerminalCreateNativeSessionInput,
  TerminalDeleteSavedSessionResponse,
  TerminalHandshakeInfo,
  TerminalImportSessionInput,
  TerminalMuxCommandResult,
  TerminalSavedSessionSummary,
  TerminalSessionSummary,
  TerminalBackendKind,
  TerminalMuxCommand,
} from "@features/terminal-workspace-kernel/contracts";
import type { TerminalWorkspaceControlGatewayPort } from "../../core/application/index.js";

const INITIAL_CONNECT_MAX_ATTEMPTS = 6;
const CONNECT_BACKOFF_MS = [100, 200, 400, 800, 1_600, 2_000] as const;

interface PendingRequest<RecordKey extends keyof TerminalGatewayControlRequestMap> {
  method: RecordKey;
  resolve(value: TerminalGatewayControlRequestMap[RecordKey]["response"]): void;
  reject(error: Error): void;
}

export class WebSocketTerminalRuntimeControlPlane implements TerminalWorkspaceControlGatewayPort {
  readonly #url: string;
  #socket: WebSocket | null = null;
  #connectPromise: Promise<WebSocket> | null = null;
  #rejectConnect: ((error: Error) => void) | null = null;
  #disposed = false;
  #connectRetryTimer: ReturnType<typeof setTimeout> | null = null;
  #resolveConnectRetry: (() => void) | null = null;
  readonly #pendingRequests = new Map<string, PendingRequest<keyof TerminalGatewayControlRequestMap>>();

  constructor(url: string) {
    this.#url = url;
  }

  async handshakeInfo(): Promise<TerminalHandshakeInfo> {
    return this.request("handshake_info", undefined);
  }

  async listSessions(): Promise<TerminalSessionSummary[]> {
    return this.request("list_sessions", undefined);
  }

  async listSavedSessions(): Promise<TerminalSavedSessionSummary[]> {
    return this.request("list_saved_sessions", undefined);
  }

  async discoverSessions(backend: TerminalBackendKind): Promise<TerminalDiscoveredSession[]> {
    return this.request("discover_sessions", { backend });
  }

  async backendCapabilities(backend: TerminalBackendKind): Promise<TerminalBackendCapabilitiesInfo> {
    return this.request("backend_capabilities", { backend });
  }

  async createNativeSession(input?: TerminalCreateNativeSessionInput): Promise<TerminalSessionSummary> {
    return this.request("create_native_session", input ?? {});
  }

  async importSession(input: TerminalImportSessionInput): Promise<TerminalSessionSummary> {
    return this.request("import_session", input);
  }

  async restoreSavedSession(sessionId: string): Promise<TerminalSessionSummary> {
    return this.request("restore_saved_session", { sessionId });
  }

  async deleteSavedSession(sessionId: string): Promise<TerminalDeleteSavedSessionResponse> {
    return this.request("delete_saved_session", { sessionId });
  }

  async dispatchMuxCommand(
    sessionId: string,
    command: TerminalMuxCommand,
  ): Promise<TerminalMuxCommandResult> {
    return this.request("dispatch_mux_command", { sessionId, command });
  }

  dispose(): void {
    this.#disposed = true;
    this.clearConnectRetryTimer();
    this.rejectAll(new Error("Terminal control plane disposed"));
    this.#rejectConnect?.(new Error("Terminal control plane disposed"));
    this.#rejectConnect = null;
    if (
      this.#socket
      && (this.#socket.readyState === WebSocket.CONNECTING || this.#socket.readyState === WebSocket.OPEN)
    ) {
      closeWebSocketBestEffort(this.#socket);
    }
    this.#socket = null;
    this.#connectPromise = null;
  }

  private async request<RecordKey extends keyof TerminalGatewayControlRequestMap>(
    method: RecordKey,
    payload: TerminalGatewayControlRequestMap[RecordKey]["payload"],
  ): Promise<TerminalGatewayControlRequestMap[RecordKey]["response"]> {
    if (this.#disposed) {
      throw new Error("Terminal control plane disposed");
    }

    const socket = await this.ensureConnectedWithRetry(INITIAL_CONNECT_MAX_ATTEMPTS);
    const requestId = crypto.randomUUID();

    return await new Promise<TerminalGatewayControlRequestMap[RecordKey]["response"]>((resolve, reject) => {
      this.#pendingRequests.set(requestId, {
        method,
        resolve,
        reject,
      });

      const envelope = {
        type: "request",
        requestId,
        method,
        payload,
      } as TerminalGatewayControlClientMessage;

      try {
        socket.send(JSON.stringify(envelope));
      } catch (error) {
        this.#pendingRequests.delete(requestId);
        reject(toError(error));
      }
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

    throw lastError ?? new Error("Failed to connect to terminal control plane");
  }

  private async ensureConnected(): Promise<WebSocket> {
    if (this.#disposed) {
      throw new Error("Terminal control plane disposed");
    }

    if (this.#socket?.readyState === WebSocket.OPEN) {
      return this.#socket;
    }

    this.#connectPromise ??= new Promise<WebSocket>((resolve, reject) => {
      const socket = new WebSocket(this.#url);
      this.#socket = socket;
      this.#rejectConnect = reject;
      const cleanup = () => {
        socket.removeEventListener("open", onOpen);
        socket.removeEventListener("error", onError);
      };
      const onOpen = () => {
        cleanup();
        if (this.#disposed) {
          this.#socket = null;
          this.#connectPromise = null;
          this.#rejectConnect = null;
          closeWebSocketBestEffort(socket);
          reject(new Error("Terminal control plane disposed"));
          return;
        }

        this.#socket = socket;
        this.#connectPromise = null;
        this.#rejectConnect = null;
        resolve(socket);
      };
      const onError = () => {
        const isCurrentSocket = this.#socket === socket;
        cleanup();
        if (isCurrentSocket) {
          this.#socket = null;
          this.#connectPromise = null;
          this.#rejectConnect = null;
        }
        reject(new Error("Failed to connect to terminal control plane"));
      };

      socket.addEventListener("open", onOpen, { once: true });
      socket.addEventListener("error", onError, { once: true });
      socket.addEventListener("message", (event) => {
        try {
          this.handleMessage(event.data.toString());
        } catch (error) {
          this.handleProtocolError(socket, error);
        }
      });
      socket.addEventListener("close", () => {
        const isCurrentSocket = this.#socket === socket;
        cleanup();
        if (isCurrentSocket) {
          this.#socket = null;
          this.#connectPromise = null;
          this.#rejectConnect = null;
        }
        reject(new Error(this.#disposed
          ? "Terminal control plane disposed"
          : "Terminal control plane connection closed"));
        this.rejectAll(new Error("Terminal control plane connection closed"));
      });
    });

    return this.#connectPromise;
  }

  private handleMessage(raw: string): void {
    const message = JSON.parse(raw) as TerminalGatewayControlServerResponse;
    if (message.type !== "response") {
      return;
    }

    const request = this.#pendingRequests.get(message.requestId);
    if (!request) {
      return;
    }

    this.#pendingRequests.delete(message.requestId);
    if (!message.ok) {
      request.reject(toError(message.error.message));
      return;
    }

    request.resolve(message.result as TerminalGatewayControlRequestMap[typeof request.method]["response"]);
  }

  private handleProtocolError(socket: WebSocket, error: unknown): void {
    if (this.#socket !== socket) {
      return;
    }

    this.#socket = null;
    this.#connectPromise = null;
    this.#rejectConnect = null;
    this.rejectAll(new Error(`Terminal control plane protocol error - ${toError(error).message}`));
    closeWebSocketBestEffort(socket);
  }

  private rejectAll(error: Error): void {
    for (const request of this.#pendingRequests.values()) {
      request.reject(error);
    }
    this.#pendingRequests.clear();
  }

  private async waitBeforeRetry(attempt: number): Promise<void> {
    const backoffMs = CONNECT_BACKOFF_MS[Math.min(attempt - 1, CONNECT_BACKOFF_MS.length - 1)];

    await new Promise<void>((resolve) => {
      this.clearConnectRetryTimer();
      this.#resolveConnectRetry = resolve;
      this.#connectRetryTimer = setTimeout(() => {
        this.#connectRetryTimer = null;
        this.#resolveConnectRetry = null;
        resolve();
      }, backoffMs);
    });
  }

  private clearConnectRetryTimer(): void {
    if (this.#connectRetryTimer) {
      clearTimeout(this.#connectRetryTimer);
      this.#connectRetryTimer = null;
    }
    this.#resolveConnectRetry?.();
    this.#resolveConnectRetry = null;
  }
}

function closeWebSocketBestEffort(socket: WebSocket): void {
  try {
    socket.close(1000, "Disposed");
  } catch {
    // Dispose must remain best-effort because browser teardown can leave sockets unusable.
  }
}

function toError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
