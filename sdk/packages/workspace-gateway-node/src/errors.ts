import type { WorkspaceGatewayErrorEnvelope } from "@terminal-platform/workspace-adapter-websocket/protocol";
import { WorkspaceError, type WorkspaceErrorCode } from "@terminal-platform/workspace-contracts";

const PUBLIC_WORKSPACE_ERROR_CODES = new Set<WorkspaceErrorCode>([
  "bootstrap_failed",
  "transport_failed",
  "storage_pressure",
  "protocol_error",
  "session_not_found",
  "pane_not_found",
  "subscription_failed",
  "unsupported_capability",
  "disposed",
]);

export function createGatewayErrorEnvelope(error: unknown): WorkspaceGatewayErrorEnvelope {
  const message = readErrorMessage(error);
  const code = readPublicErrorCode(error);

  return code ? { message, code } : { message };
}

export function createProtocolError(message: string): WorkspaceError {
  return new WorkspaceError({
    code: "protocol_error",
    message,
    recoverable: false,
  });
}

export function createSubscriptionError(message: string): WorkspaceError {
  return new WorkspaceError({
    code: "subscription_failed",
    message,
    recoverable: true,
  });
}

function readErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }

  if (typeof error === "string" && error.trim()) {
    return error;
  }

  return "Workspace gateway request failed";
}

function readPublicErrorCode(error: unknown): WorkspaceErrorCode | null {
  if (error instanceof WorkspaceError) {
    return error.code;
  }

  if (error instanceof Error && "code" in error && typeof error.code === "string") {
    return isPublicWorkspaceErrorCode(error.code) ? error.code : null;
  }

  return null;
}

function isPublicWorkspaceErrorCode(code: string): code is WorkspaceErrorCode {
  return PUBLIC_WORKSPACE_ERROR_CODES.has(code as WorkspaceErrorCode);
}
