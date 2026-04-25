import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import { resolveWorkspaceCapability, type TerminalWorkspaceCapabilityStatus } from "./terminal-workspace-capabilities.js";

export type TerminalCommandDockCapabilityStatus = TerminalWorkspaceCapabilityStatus;

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
  const capability = resolveWorkspaceCapability(snapshot, "pane_paste_write", {
    missingBackend: false,
    pendingCapabilities: true,
  });
  return {
    canPaste: capability.enabled,
    status: capability.status,
  };
}
