import type { BackendKind } from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

export type TerminalCommandDockCapabilityStatus = "known" | "unknown";

export interface TerminalCommandDockControlState {
  activeSessionId: string | null;
  activePaneId: string | null;
  draft: string;
  commandHistory: string[];
  recentCommands: string[];
  canSend: boolean;
  canUsePane: boolean;
  canPasteClipboard: boolean;
  pasteCapabilityStatus: TerminalCommandDockCapabilityStatus;
}

export function resolveTerminalCommandDockControlState(
  snapshot: WorkspaceSnapshot,
  options: { pending: boolean },
): TerminalCommandDockControlState {
  const activeSessionId =
    snapshot.selection.activeSessionId ?? snapshot.attachedSession?.session.session_id ?? null;
  const activePaneId =
    snapshot.selection.activePaneId ?? snapshot.attachedSession?.focused_screen?.pane_id ?? null;
  const draft = activePaneId ? (snapshot.drafts[activePaneId] ?? "") : "";
  const canUsePane = Boolean(activeSessionId && activePaneId && !options.pending);
  const pasteCapability = resolvePasteCapability(snapshot);

  return {
    activeSessionId,
    activePaneId,
    draft,
    commandHistory: snapshot.commandHistory.entries,
    recentCommands: [...snapshot.commandHistory.entries].slice(-5).reverse(),
    canSend: Boolean(canUsePane && draft.trim().length > 0),
    canUsePane,
    canPasteClipboard: Boolean(canUsePane && pasteCapability.canPaste),
    pasteCapabilityStatus: pasteCapability.status,
  };
}

function resolvePasteCapability(
  snapshot: WorkspaceSnapshot,
): { canPaste: boolean; status: TerminalCommandDockCapabilityStatus } {
  const backend = resolveActiveBackend(snapshot);
  if (!backend) {
    return { canPaste: false, status: "unknown" };
  }

  const capabilities = snapshot.catalog.backendCapabilities[backend]?.capabilities;
  if (!capabilities) {
    return { canPaste: true, status: "unknown" };
  }

  return {
    canPaste: capabilities.pane_paste_write,
    status: "known",
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
