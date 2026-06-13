import { randomUUID } from "node:crypto";
import { once } from "node:events";
import type { IncomingMessage } from "node:http";

import { WebSocketServer, type WebSocket } from "ws";
import { encodeWorkspaceWebSocketPayload } from "@terminal-platform/workspace-adapter-websocket";

import { dispatchWorkspaceGatewayControlPayload } from "./dispatcher.js";
import { WorkspaceGatewayStreamConnection } from "./subscriptions.js";
import type {
  WorkspaceGatewayAuthPolicy,
  WorkspaceGatewayNodeServerHandle,
  WorkspaceGatewayNodeServerOptions,
  WorkspaceGatewayNodeServerUrls,
  WorkspaceGatewayPlane,
  WorkspaceGatewayFaultInjectionPort,
  WorkspaceGatewayLogger,
  WorkspaceRuntimeClientPort,
} from "./types.js";

const DEFAULT_HOST = "127.0.0.1";
const DEFAULT_PORT = 0;
const DEFAULT_CONTROL_PATH = "/workspace/control";
const DEFAULT_STREAM_PATH = "/workspace/stream";

interface NormalizedWorkspaceGatewayNodeServerOptions {
  readonly runtime: WorkspaceRuntimeClientPort;
  readonly host: string;
  readonly port: number;
  readonly controlPath: string;
  readonly streamPath: string;
  readonly logger: WorkspaceGatewayLogger | undefined;
  readonly closeTimeoutMs: number | undefined;
  readonly faultInjection: WorkspaceGatewayFaultInjectionPort | null;
}

export async function startWorkspaceGatewayNodeServer(
  options: WorkspaceGatewayNodeServerOptions,
): Promise<WorkspaceGatewayNodeServerHandle> {
  const server = new WorkspaceGatewayNodeServer(options);
  await server.start();
  return server;
}

export class WorkspaceGatewayNodeServer implements WorkspaceGatewayNodeServerHandle {
  readonly #options: NormalizedWorkspaceGatewayNodeServerOptions;
  readonly #token: string;
  readonly #authPolicy: WorkspaceGatewayAuthPolicy;
  readonly #server: WebSocketServer;
  readonly #controlSockets = new Set<WebSocket>();
  readonly #streamConnections = new Set<WorkspaceGatewayStreamConnection>();
  #started = false;
  #disposed = false;

  constructor(options: WorkspaceGatewayNodeServerOptions) {
    this.#options = {
      runtime: options.runtime,
      host: options.host ?? DEFAULT_HOST,
      port: options.port ?? DEFAULT_PORT,
      controlPath: normalizePath(options.controlPath ?? DEFAULT_CONTROL_PATH),
      streamPath: normalizePath(options.streamPath ?? DEFAULT_STREAM_PATH),
      logger: options.logger,
      closeTimeoutMs: options.closeTimeoutMs,
      faultInjection: options.faultInjection ?? null,
    };
    this.#token = options.token ?? randomUUID();
    this.#authPolicy = options.authPolicy ?? createTokenAuthPolicy(this.#token);
    this.#server = new WebSocketServer({
      host: this.#options.host,
      port: this.#options.port,
    });
    this.#server.on("connection", (socket, request) => {
      this.handleConnection(socket, request.url ?? "/", request);
    });
  }

  get controlUrl(): string {
    return this.urls.controlUrl;
  }

  get streamUrl(): string {
    return this.urls.streamUrl;
  }

  get urls(): WorkspaceGatewayNodeServerUrls {
    const address = this.#server.address();
    if (!address || typeof address === "string") {
      throw new Error("workspace gateway node server is not listening on a TCP address");
    }

    return {
      controlUrl: buildUrl(this.#options.host, address.port, this.#options.controlPath, this.#token),
      streamUrl: buildUrl(this.#options.host, address.port, this.#options.streamPath, this.#token),
    };
  }

  async start(): Promise<void> {
    if (this.#started) {
      return;
    }

    await once(this.#server, "listening");
    this.#started = true;
  }

  async dispose(): Promise<void> {
    if (this.#disposed) {
      return;
    }

    this.#disposed = true;
    await Promise.allSettled([...this.#streamConnections].map((connection) => connection.dispose("dispose")));
    this.#streamConnections.clear();

    for (const socket of this.#controlSockets) {
      closeSocket(socket);
    }
    this.#controlSockets.clear();

    await closeServer(this.#server);
    await this.#options.runtime.close();
  }

  private handleConnection(socket: WebSocket, rawUrl: string, request: IncomingMessage): void {
    if (this.#disposed) {
      closeSocket(socket);
      return;
    }

    const url = new URL(rawUrl, "ws://127.0.0.1");
    const plane = this.resolvePlane(url.pathname);
    if (!plane || !this.#authPolicy.authorize({ request, url, plane })) {
      socket.close(1008, "Unauthorized workspace gateway client");
      return;
    }

    if (plane === "control") {
      this.registerControlSocket(socket);
      return;
    }

    this.registerStreamSocket(socket);
  }

  private resolvePlane(pathname: string): WorkspaceGatewayPlane | null {
    const normalized = normalizePath(pathname);
    if (normalized === this.#options.controlPath) {
      return "control";
    }

    if (normalized === this.#options.streamPath) {
      return "stream";
    }

    return null;
  }

  private registerControlSocket(socket: WebSocket): void {
    this.#controlSockets.add(socket);
    socket.on("message", (payload) => {
      void this.handleControlPayload(socket, payload.toString());
    });
    socket.on("close", () => {
      this.#controlSockets.delete(socket);
    });
  }

  private async handleControlPayload(socket: WebSocket, raw: string): Promise<void> {
    const response = await dispatchWorkspaceGatewayControlPayload({
      raw,
      runtime: this.#options.runtime,
      ...(this.#options.faultInjection ? { faultInjection: this.#options.faultInjection } : {}),
    });

    try {
      socket.send(encodeWorkspaceWebSocketPayload(response));
    } catch (error) {
      this.#options.logger?.warn?.("workspace gateway control send failed", { error });
      closeSocket(socket);
    }
  }

  private registerStreamSocket(socket: WebSocket): void {
    const connectionOptions = {
      socket,
      runtime: this.#options.runtime,
      ...(this.#options.logger ? { logger: this.#options.logger } : {}),
      ...(this.#options.closeTimeoutMs ? { closeTimeoutMs: this.#options.closeTimeoutMs } : {}),
      ...(this.#options.faultInjection ? { faultInjection: this.#options.faultInjection } : {}),
    };
    const connection = new WorkspaceGatewayStreamConnection(connectionOptions);
    this.#streamConnections.add(connection);

    socket.on("message", (payload) => {
      connection.handlePayload(payload.toString());
    });
    socket.on("close", () => {
      this.#streamConnections.delete(connection);
      void connection.dispose("socket_close");
    });
  }
}

function createTokenAuthPolicy(token: string): WorkspaceGatewayAuthPolicy {
  return {
    authorize({ url }) {
      return url.searchParams.get("token") === token;
    },
  };
}

function buildUrl(host: string, port: number, path: string, token: string): string {
  const url = new URL(`ws://${host}:${port}${path}`);
  url.searchParams.set("token", token);
  return url.toString();
}

function normalizePath(path: string): string {
  if (!path.startsWith("/")) {
    return `/${path}`;
  }

  return path;
}

async function closeServer(server: WebSocketServer): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.close((error) => {
      if (error) {
        reject(error);
        return;
      }
      resolve();
    });
  });
}

function closeSocket(socket: WebSocket): void {
  try {
    socket.close();
  } catch {
    socket.terminate();
  }
}
