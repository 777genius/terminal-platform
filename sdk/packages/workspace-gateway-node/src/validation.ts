import type {
  BackendKind,
  SessionId,
  PaneId,
} from "@terminal-platform/runtime-types";
import type {
  WorkspaceGatewayControlClientMessage,
  WorkspaceGatewayControlMethod,
  WorkspaceGatewayStreamClientMessage,
} from "@terminal-platform/workspace-adapter-websocket/protocol";

import { createProtocolError } from "./errors.js";

const WORKSPACE_CONTROL_METHODS = new Set<WorkspaceGatewayControlMethod>([
  "workspace_handshake",
  "workspace_list_sessions",
  "workspace_list_saved_sessions",
  "workspace_discover_sessions",
  "workspace_backend_capabilities",
  "workspace_create_session",
  "workspace_import_session",
  "workspace_saved_session",
  "workspace_command_history",
  "workspace_pane_history",
  "workspace_prune_saved_sessions",
  "workspace_restore_saved_session",
  "workspace_delete_saved_session",
  "workspace_attach_session",
  "workspace_topology_snapshot",
  "workspace_screen_snapshot",
  "workspace_screen_delta",
  "workspace_dispatch_mux_command",
]);

const BACKEND_KINDS = new Set<BackendKind>(["native", "tmux", "zellij"]);

export function validateControlClientMessage(value: unknown): WorkspaceGatewayControlClientMessage {
  const message = readRecord(value, "Control message");
  if (message.type !== "request") {
    throw createProtocolError("Control message type must be request");
  }

  const requestId = readRequiredString(message, "requestId");
  const method = readRequiredControlMethod(message);
  const payload = message.payload;

  validateControlPayload(method, payload);

  return {
    type: "request",
    requestId,
    method,
    payload,
  } as WorkspaceGatewayControlClientMessage;
}

export function validateStreamClientMessage(value: unknown): WorkspaceGatewayStreamClientMessage {
  const message = readRecord(value, "Stream message");
  if (message.type === "workspace_subscribe") {
    const subscriptionId = readRequiredString(message, "subscriptionId");
    const sessionId = readRequiredString(message, "sessionId") as SessionId;
    const spec = readRecord(message.spec, "Stream message spec");

    return {
      type: "workspace_subscribe",
      subscriptionId,
      sessionId,
      spec: spec as Extract<WorkspaceGatewayStreamClientMessage, { type: "workspace_subscribe" }>["spec"],
    };
  }

  if (message.type === "workspace_unsubscribe") {
    return {
      type: "workspace_unsubscribe",
      subscriptionId: readRequiredString(message, "subscriptionId"),
    };
  }

  throw createProtocolError("Stream message type must be workspace_subscribe or workspace_unsubscribe");
}

function validateControlPayload(method: WorkspaceGatewayControlMethod, payload: unknown): void {
  switch (method) {
    case "workspace_handshake":
    case "workspace_list_sessions":
    case "workspace_list_saved_sessions":
      if (payload !== undefined) {
        throw createProtocolError(`Control payload for ${method} must be undefined`);
      }
      return;
    case "workspace_discover_sessions":
    case "workspace_backend_capabilities":
      readBackendPayload(payload);
      return;
    case "workspace_create_session": {
      const record = readRecord(payload, "Control payload");
      readBackendPayload(record);
      readRecord(record.request, "Control payload request");
      return;
    }
    case "workspace_import_session": {
      const record = readRecord(payload, "Control payload");
      readRecord(record.route, "Control payload route");
      readOptionalString(record, "title");
      return;
    }
    case "workspace_saved_session":
    case "workspace_restore_saved_session":
    case "workspace_delete_saved_session":
    case "workspace_attach_session":
    case "workspace_topology_snapshot": {
      const record = readRecord(payload, "Control payload");
      readRequiredString(record, "sessionId");
      return;
    }
    case "workspace_command_history": {
      const record = readRecord(payload, "Control payload");
      readOptionalString(record, "sessionId");
      readOptionalNumber(record, "limit");
      return;
    }
    case "workspace_pane_history": {
      const record = readRecord(payload, "Control payload");
      readRequiredString(record, "sessionId");
      readRequiredString(record, "paneId");
      readOptionalNumberOrBigInt(record, "fromEventSeq");
      readOptionalNumber(record, "maxSegments");
      readOptionalNumber(record, "maxBytes");
      return;
    }
    case "workspace_prune_saved_sessions": {
      const record = readRecord(payload, "Control payload");
      readRequiredNumber(record, "keepLatest");
      return;
    }
    case "workspace_screen_snapshot": {
      const record = readRecord(payload, "Control payload");
      readRequiredString(record, "sessionId");
      readRequiredString(record, "paneId");
      return;
    }
    case "workspace_screen_delta": {
      const record = readRecord(payload, "Control payload");
      readRequiredString(record, "sessionId");
      readRequiredString(record, "paneId");
      readRequiredBigInt(record, "fromSequence");
      return;
    }
    case "workspace_dispatch_mux_command": {
      const record = readRecord(payload, "Control payload");
      readRequiredString(record, "sessionId");
      readRecord(record.command, "Control payload command");
      return;
    }
  }
}

function readBackendPayload(value: unknown): BackendKind {
  const record = readRecord(value, "Control payload");
  const backend = readRequiredString(record, "backend");
  if (!BACKEND_KINDS.has(backend as BackendKind)) {
    throw createProtocolError("Control payload backend must be native, tmux, or zellij");
  }

  return backend as BackendKind;
}

function readRequiredControlMethod(record: Record<string, unknown>): WorkspaceGatewayControlMethod {
  const method = readRequiredString(record, "method");
  if (!WORKSPACE_CONTROL_METHODS.has(method as WorkspaceGatewayControlMethod)) {
    throw createProtocolError(`Unsupported workspace gateway method: ${method}`);
  }

  return method as WorkspaceGatewayControlMethod;
}

function readRecord(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw createProtocolError(`${label} must be an object`);
  }

  return value as Record<string, unknown>;
}

function readRequiredString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string" || value.length === 0) {
    throw createProtocolError(`${key} must be a non-empty string`);
  }

  return value;
}

function readOptionalString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  if (value == null) {
    return null;
  }

  if (typeof value !== "string") {
    throw createProtocolError(`${key} must be a string or null`);
  }

  return value;
}

function readRequiredNumber(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw createProtocolError(`${key} must be a finite number`);
  }

  return value;
}

function readOptionalNumber(record: Record<string, unknown>, key: string): number | null {
  const value = record[key];
  if (value == null) {
    return null;
  }

  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw createProtocolError(`${key} must be a finite number or null`);
  }

  return value;
}

function readRequiredBigInt(record: Record<string, unknown>, key: string): bigint {
  const value = record[key];
  if (typeof value !== "bigint") {
    throw createProtocolError(`${key} must be a bigint`);
  }

  return value;
}

function readOptionalNumberOrBigInt(record: Record<string, unknown>, key: string): number | bigint | null {
  const value = record[key];
  if (value == null) {
    return null;
  }

  if (typeof value === "bigint") {
    return value;
  }

  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw createProtocolError(`${key} must be a finite number, bigint, or null`);
  }

  return value;
}
