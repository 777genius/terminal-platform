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

interface PendingRequest<RecordKey extends keyof TerminalGatewayControlRequestMap> {
  method: RecordKey;
  resolve(value: TerminalGatewayControlRequestMap[RecordKey]["response"]): void;
  reject(error: Error): void;
}

export class WebSocketTerminalRuntimeControlPlane implements TerminalWorkspaceControlGatewayPort {
  readonly #url: string;
  #socket: WebSocket | null = null;
  #connectPromise: Promise<WebSocket> | null = null;
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
    this.rejectAll(new Error("Terminal control plane disposed"));
    if (this.#socket && this.#socket.readyState === WebSocket.OPEN) {
      this.#socket.close(1000, "Disposed");
    }
    this.#socket = null;
    this.#connectPromise = null;
  }

  private async request<RecordKey extends keyof TerminalGatewayControlRequestMap>(
    method: RecordKey,
    payload: TerminalGatewayControlRequestMap[RecordKey]["payload"],
  ): Promise<TerminalGatewayControlRequestMap[RecordKey]["response"]> {
    const socket = await this.ensureConnected();
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

      socket.send(JSON.stringify(envelope));
    });
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
        reject(new Error("Failed to connect to terminal control plane"));
      };

      socket.addEventListener("open", onOpen, { once: true });
      socket.addEventListener("error", onError, { once: true });
      socket.addEventListener("message", (event) => {
        this.handleMessage(event.data.toString());
      });
      socket.addEventListener("close", () => {
        this.#socket = null;
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

  private rejectAll(error: Error): void {
    for (const request of this.#pendingRequests.values()) {
      request.reject(error);
    }
    this.#pendingRequests.clear();
  }
}

function toError(message: string): Error {
  return new Error(message);
}
