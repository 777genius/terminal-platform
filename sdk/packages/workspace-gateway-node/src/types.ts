import type { IncomingMessage } from "node:http";

import type {
  WorkspaceGatewayControlClientMessage,
  WorkspaceGatewayStreamClientMessage,
  WorkspaceGatewayStreamServerMessage,
} from "@terminal-platform/workspace-adapter-websocket/protocol";
import type { WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";

export type WorkspaceRuntimeClientPort = WorkspaceTransportClient;

export interface WorkspaceGatewayNodeServerUrls {
  readonly controlUrl: string;
  readonly streamUrl: string;
}

export interface WorkspaceGatewayNodeServerHandle extends WorkspaceGatewayNodeServerUrls {
  dispose(): Promise<void>;
}

export interface WorkspaceGatewayLogger {
  debug?(message: string, context?: Record<string, unknown>): void;
  info?(message: string, context?: Record<string, unknown>): void;
  warn?(message: string, context?: Record<string, unknown>): void;
  error?(message: string, context?: Record<string, unknown>): void;
}

export interface WorkspaceGatewayAuthRequest {
  readonly request: IncomingMessage;
  readonly url: URL;
  readonly plane: WorkspaceGatewayPlane;
}

export interface WorkspaceGatewayAuthPolicy {
  authorize(request: WorkspaceGatewayAuthRequest): boolean;
}

export interface WorkspaceGatewayFaultInjectionPort {
  beforeControlRequest?(message: WorkspaceGatewayControlClientMessage): Promise<void> | void;
  beforeSubscriptionOpen?(message: Extract<WorkspaceGatewayStreamClientMessage, { type: "workspace_subscribe" }>): Promise<void> | void;
  beforeSubscriptionClose?(subscriptionId: string): Promise<void> | void;
  beforeServerSend?(message: WorkspaceGatewayStreamServerMessage): Promise<void> | void;
}

export interface WorkspaceGatewayNodeServerOptions {
  readonly runtime: WorkspaceRuntimeClientPort;
  readonly host?: string;
  readonly port?: number;
  readonly controlPath?: string;
  readonly streamPath?: string;
  readonly token?: string;
  readonly authPolicy?: WorkspaceGatewayAuthPolicy;
  readonly logger?: WorkspaceGatewayLogger;
  readonly closeTimeoutMs?: number;
  readonly faultInjection?: WorkspaceGatewayFaultInjectionPort | null;
}

export type WorkspaceGatewayPlane = "control" | "stream";

export type WorkspaceGatewayCloseReason =
  | "dispose"
  | "socket_close"
  | "unsubscribe"
  | "send_failure"
  | "runtime_closed";
