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
  canWriteInput: boolean;
  canPasteClipboard: boolean;
  inputCapabilityStatus: TerminalCommandDockCapabilityStatus;
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
  const inputCapability = resolveInputCapability(snapshot);
  const pasteCapability = resolvePasteCapability(snapshot);
  const canWriteInput = Boolean(canUsePane && inputCapability.canWrite);

  return {
    activeSessionId,
    activePaneId,
    draft,
    commandHistory: snapshot.commandHistory.entries,
    recentCommands: [...snapshot.commandHistory.entries].slice(-5).reverse(),
    canSend: Boolean(canWriteInput && draft.trim().length > 0),
    canUsePane,
    canWriteInput,
    canPasteClipboard: Boolean(canUsePane && pasteCapability.canPaste),
    inputCapabilityStatus: inputCapability.status,
    pasteCapabilityStatus: pasteCapability.status,
  };
}

function resolveInputCapability(
  snapshot: WorkspaceSnapshot,
): { canWrite: boolean; status: TerminalCommandDockCapabilityStatus } {
  const capability = resolveWorkspaceCapability(snapshot, "pane_input_write", {
    missingBackend: false,
    pendingCapabilities: true,
  });
  return {
    canWrite: capability.enabled,
    status: capability.status,
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
