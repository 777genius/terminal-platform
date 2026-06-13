import type {
  WorkspaceGatewayControlClientMessage,
  WorkspaceGatewayControlServerResponse,
} from "@terminal-platform/workspace-adapter-websocket/protocol";
import { decodeWorkspaceWebSocketPayload } from "@terminal-platform/workspace-adapter-websocket";
import type { WorkspacePaneHistoryRequestOptions } from "@terminal-platform/workspace-contracts";

import { createGatewayErrorEnvelope } from "./errors.js";
import { validateControlClientMessage } from "./validation.js";
import type {
  WorkspaceGatewayFaultInjectionPort,
  WorkspaceRuntimeClientPort,
} from "./types.js";

export async function dispatchWorkspaceGatewayControlPayload(options: {
  readonly raw: string;
  readonly runtime: WorkspaceRuntimeClientPort;
  readonly faultInjection?: WorkspaceGatewayFaultInjectionPort | null;
}): Promise<WorkspaceGatewayControlServerResponse> {
  let message: WorkspaceGatewayControlClientMessage;

  try {
    message = validateControlClientMessage(
      decodeWorkspaceWebSocketPayload<unknown>(options.raw),
    );
  } catch (error) {
    return {
      type: "response",
      requestId: "invalid-request",
      method: "workspace_handshake",
      ok: false,
      error: createGatewayErrorEnvelope(error),
    };
  }

  try {
    await options.faultInjection?.beforeControlRequest?.(message);
    const result = await dispatchWorkspaceGatewayControlRequest(options.runtime, message);
    return {
      type: "response",
      requestId: message.requestId,
      method: message.method,
      ok: true,
      result,
    } as WorkspaceGatewayControlServerResponse;
  } catch (error) {
    return {
      type: "response",
      requestId: message.requestId,
      method: message.method,
      ok: false,
      error: createGatewayErrorEnvelope(error),
    };
  }
}

export async function dispatchWorkspaceGatewayControlRequest(
  runtime: WorkspaceRuntimeClientPort,
  message: WorkspaceGatewayControlClientMessage,
): Promise<unknown> {
  switch (message.method) {
    case "workspace_handshake":
      return runtime.handshake();
    case "workspace_list_sessions":
      return runtime.listSessions();
    case "workspace_list_saved_sessions":
      return runtime.listSavedSessions();
    case "workspace_discover_sessions":
      return runtime.discoverSessions(message.payload.backend);
    case "workspace_backend_capabilities":
      return runtime.getBackendCapabilities(message.payload.backend);
    case "workspace_create_session":
      return runtime.createSession(message.payload.backend, message.payload.request);
    case "workspace_import_session":
      return runtime.importSession(message.payload.route, message.payload.title ?? null);
    case "workspace_saved_session":
      return runtime.getSavedSession(message.payload.sessionId);
    case "workspace_command_history":
      return runtime.listCommandHistory?.(
        message.payload.sessionId ?? null,
        message.payload.limit ?? null,
      ) ?? [];
    case "workspace_pane_history": {
      if (!runtime.getPaneHistory) {
        throw new Error("pane history is not supported by this runtime");
      }

      const paneHistoryOptions: WorkspacePaneHistoryRequestOptions = {
        fromEventSeq: message.payload.fromEventSeq ?? null,
        maxSegments: message.payload.maxSegments ?? null,
        maxBytes: message.payload.maxBytes ?? null,
      };
      return runtime.getPaneHistory(
        message.payload.sessionId,
        message.payload.paneId,
        paneHistoryOptions,
      );
    }
    case "workspace_prune_saved_sessions":
      return runtime.pruneSavedSessions(message.payload.keepLatest);
    case "workspace_restore_saved_session":
      return runtime.restoreSavedSession(message.payload.sessionId);
    case "workspace_delete_saved_session":
      return runtime.deleteSavedSession(message.payload.sessionId);
    case "workspace_attach_session":
      return runtime.attachSession(message.payload.sessionId);
    case "workspace_topology_snapshot":
      return runtime.getTopologySnapshot(message.payload.sessionId);
    case "workspace_screen_snapshot":
      return runtime.getScreenSnapshot(message.payload.sessionId, message.payload.paneId);
    case "workspace_screen_delta":
      return runtime.getScreenDelta(
        message.payload.sessionId,
        message.payload.paneId,
        message.payload.fromSequence,
      );
    case "workspace_dispatch_mux_command":
      return runtime.dispatchMuxCommand(message.payload.sessionId, message.payload.command);
  }
}
