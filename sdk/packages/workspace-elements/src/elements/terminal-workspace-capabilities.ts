import type { BackendCapabilitiesInfo, BackendKind } from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

export type TerminalWorkspaceCapabilityStatus = "known" | "unknown";

export interface TerminalWorkspaceCapabilityState {
  backend: BackendKind | null;
  capabilities: BackendCapabilitiesInfo | null;
  status: TerminalWorkspaceCapabilityStatus;
}

export function resolveActiveBackendCapabilities(snapshot: WorkspaceSnapshot): TerminalWorkspaceCapabilityState {
  const backend = resolveActiveBackend(snapshot);
  if (!backend) {
    return {
      backend: null,
      capabilities: null,
      status: "unknown",
    };
  }

  const capabilities = snapshot.catalog.backendCapabilities[backend] ?? null;
  return {
    backend,
    capabilities,
    status: capabilities ? "known" : "unknown",
  };
}

export function resolveWorkspaceCapability(
  snapshot: WorkspaceSnapshot,
  key: keyof BackendCapabilitiesInfo["capabilities"],
  options: {
    missingBackend: boolean;
    pendingCapabilities: boolean;
  },
): { enabled: boolean; status: TerminalWorkspaceCapabilityStatus } {
  const state = resolveActiveBackendCapabilities(snapshot);
  if (!state.backend) {
    return {
      enabled: options.missingBackend,
      status: state.status,
    };
  }

  if (!state.capabilities) {
    return {
      enabled: options.pendingCapabilities,
      status: state.status,
    };
  }

  return {
    enabled: state.capabilities.capabilities[key],
    status: state.status,
  };
}

function resolveActiveBackend(snapshot: WorkspaceSnapshot): BackendKind | null {
  if (snapshot.attachedSession) {
    return snapshot.attachedSession.session.route.backend;
  }

  const activeSessionId = snapshot.selection.activeSessionId;
  if (!activeSessionId) {
    return null;
  }

  return snapshot.catalog.sessions.find((session) => session.session_id === activeSessionId)?.route.backend ?? null;
}
