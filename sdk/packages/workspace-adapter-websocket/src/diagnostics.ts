import {
  WorkspaceError,
  type WorkspaceErrorCode,
} from "@terminal-platform/workspace-contracts";

import type { WorkspaceGatewayErrorEnvelope } from "./protocol.js";

export const WORKSPACE_WEBSOCKET_DIAGNOSTIC_CODES = {
  controlConnectFailed: "websocket_control_connect_failed",
  controlConnectionClosed: "websocket_control_connection_closed",
  gatewayError: "websocket_gateway_error",
  sendFailed: "websocket_send_failed",
  streamConnectFailed: "websocket_stream_connect_failed",
  streamConnectionClosed: "websocket_stream_connection_closed",
  subscriptionClosedBeforeActivation: "websocket_subscription_closed_before_activation",
  subscriptionCloseTimeout: "websocket_subscription_close_timeout",
  transportClosed: "websocket_transport_closed",
} as const;

export type WorkspaceWebSocketDiagnosticCode =
  (typeof WORKSPACE_WEBSOCKET_DIAGNOSTIC_CODES)[keyof typeof WORKSPACE_WEBSOCKET_DIAGNOSTIC_CODES];

export type WorkspaceWebSocketDiagnosticPlane = "control" | "stream";
export type WorkspaceWebSocketDiagnosticPhase =
  | "connect"
  | "dispose"
  | "gateway"
  | "request"
  | "response"
  | "subscription";

export interface WorkspaceWebSocketDiagnosticInput {
  code: WorkspaceWebSocketDiagnosticCode;
  message: string;
  phase: WorkspaceWebSocketDiagnosticPhase;
  plane?: WorkspaceWebSocketDiagnosticPlane;
  workspaceErrorCode?: WorkspaceErrorCode;
  recoverable?: boolean;
  gatewayCode?: WorkspaceErrorCode;
  cause?: unknown;
}

export interface WorkspaceWebSocketDiagnostic {
  code: WorkspaceWebSocketDiagnosticCode;
  workspaceErrorCode: WorkspaceErrorCode;
  message: string;
  severity: "error";
  recoverable: boolean;
  phase: WorkspaceWebSocketDiagnosticPhase;
  plane?: WorkspaceWebSocketDiagnosticPlane;
  gatewayCode?: string;
  cause?: unknown;
}

const knownWorkspaceErrorCodes = new Set<WorkspaceErrorCode>([
  "bootstrap_failed",
  "disposed",
  "pane_not_found",
  "protocol_error",
  "session_not_found",
  "storage_pressure",
  "subscription_failed",
  "transport_failed",
  "unsupported_capability",
]);

export function mapWorkspaceWebSocketDiagnostic(
  input: WorkspaceWebSocketDiagnosticInput,
): WorkspaceWebSocketDiagnostic {
  const diagnostic: WorkspaceWebSocketDiagnostic = {
    code: input.code,
    workspaceErrorCode: input.workspaceErrorCode ?? "transport_failed",
    message: input.message,
    severity: "error",
    recoverable: input.recoverable ?? true,
    phase: input.phase,
    ...(input.plane ? { plane: input.plane } : {}),
    ...(input.gatewayCode ? { gatewayCode: input.gatewayCode } : {}),
    ...(input.cause !== undefined ? { cause: input.cause } : {}),
  };

  return diagnostic;
}

export function createWorkspaceWebSocketDiagnosticError(
  input: WorkspaceWebSocketDiagnosticInput,
): WorkspaceError {
  const diagnostic = mapWorkspaceWebSocketDiagnostic(input);

  return new WorkspaceError({
    code: diagnostic.workspaceErrorCode,
    message: diagnostic.message,
    recoverable: diagnostic.recoverable,
    ...(diagnostic.cause !== undefined ? { cause: diagnostic.cause } : {}),
  });
}

export function mapWorkspaceGatewayError(
  error: WorkspaceGatewayErrorEnvelope,
  options: {
    phase: WorkspaceWebSocketDiagnosticPhase;
    plane?: WorkspaceWebSocketDiagnosticPlane;
  },
): WorkspaceWebSocketDiagnostic {
  const gatewayCode = readKnownWorkspaceGatewayErrorCode(error.code);

  return mapWorkspaceWebSocketDiagnostic({
    code: WORKSPACE_WEBSOCKET_DIAGNOSTIC_CODES.gatewayError,
    message: error.message,
    phase: options.phase,
    ...(options.plane ? { plane: options.plane } : {}),
    ...(gatewayCode ? { gatewayCode } : {}),
    workspaceErrorCode: gatewayCode ?? "transport_failed",
    recoverable: true,
  });
}

export function createWorkspaceGatewayError(
  error: WorkspaceGatewayErrorEnvelope,
  options: {
    phase: WorkspaceWebSocketDiagnosticPhase;
    plane?: WorkspaceWebSocketDiagnosticPlane;
  },
): WorkspaceError {
  const diagnostic = mapWorkspaceGatewayError(error, options);

  return new WorkspaceError({
    code: diagnostic.workspaceErrorCode,
    message: diagnostic.message,
    recoverable: diagnostic.recoverable,
  });
}

export function normalizeWorkspaceGatewayErrorCode(code: string | undefined): WorkspaceErrorCode {
  return readKnownWorkspaceGatewayErrorCode(code) ?? "transport_failed";
}

function readKnownWorkspaceGatewayErrorCode(code: string | undefined): WorkspaceErrorCode | null {
  return knownWorkspaceErrorCodes.has(code as WorkspaceErrorCode)
    ? code as WorkspaceErrorCode
    : null;
}
