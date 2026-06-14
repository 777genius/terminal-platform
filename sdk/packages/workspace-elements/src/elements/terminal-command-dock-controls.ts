import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  resolveTerminalCommandRecentCommands,
  type TerminalCommandRecentCommandPresentation,
} from "./terminal-command-recent-commands.js";
import { resolveWorkspaceCapability, type TerminalWorkspaceCapabilityStatus } from "./terminal-workspace-capabilities.js";

export const TERMINAL_COMMAND_DOCK_DEFAULT_RECENT_COMMAND_LIMIT = 5;
export const TERMINAL_COMMAND_DOCK_TERMINAL_RECENT_COMMAND_LIMIT = 2;

export type TerminalCommandDockCapabilityStatus = TerminalWorkspaceCapabilityStatus;

export interface TerminalCommandDockControlState {
  activeSessionId: string | null;
  activePaneId: string | null;
  draft: string;
  commandHistory: string[];
  recentCommands: string[];
  recentCommandEntries: TerminalCommandRecentCommandPresentation[];
  canSend: boolean;
  canInterrupt: boolean;
  canUsePane: boolean;
  canWriteInput: boolean;
  canPasteClipboard: boolean;
  canSaveLayout: boolean;
  inputCapabilityStatus: TerminalCommandDockCapabilityStatus;
  pasteCapabilityStatus: TerminalCommandDockCapabilityStatus;
  saveCapabilityStatus: TerminalCommandDockCapabilityStatus;
}

export function resolveTerminalCommandDockControlState(
  snapshot: WorkspaceSnapshot,
  options: { pending: boolean; recentCommandLimit?: number | null },
): TerminalCommandDockControlState {
  const activeSessionId =
    snapshot.selection.activeSessionId ?? snapshot.attachedSession?.session.session_id ?? null;
  const activePaneId =
    snapshot.selection.activePaneId ?? snapshot.attachedSession?.focused_screen?.pane_id ?? null;
  const draft = activePaneId ? (snapshot.drafts[activePaneId] ?? "") : "";
  const canUsePane = Boolean(activeSessionId && activePaneId && !options.pending);
  const inputCapability = resolveInputCapability(snapshot);
  const pasteCapability = resolvePasteCapability(snapshot);
  const saveCapability = resolveSaveCapability(snapshot);
  const canWriteInput = Boolean(canUsePane && inputCapability.canWrite);
  const recentCommandLimit = normalizeRecentCommandLimit(options.recentCommandLimit);
  const recentCommandEntries = resolveTerminalCommandRecentCommands(snapshot.commandHistory.entries, recentCommandLimit);
  const hasTerminalHistory = hasFocusedPaneTerminalHistory(snapshot, activePaneId);

  return {
    activeSessionId,
    activePaneId,
    draft,
    commandHistory: snapshot.commandHistory.entries,
    recentCommands: recentCommandEntries.map((entry) => entry.value),
    recentCommandEntries,
    canSend: Boolean(canWriteInput && draft.trim().length > 0),
    canInterrupt: Boolean(canWriteInput && hasTerminalHistory),
    canUsePane,
    canWriteInput,
    canPasteClipboard: Boolean(canUsePane && pasteCapability.canPaste),
    canSaveLayout: Boolean(activeSessionId && !options.pending && saveCapability.canSave),
    inputCapabilityStatus: inputCapability.status,
    pasteCapabilityStatus: pasteCapability.status,
    saveCapabilityStatus: saveCapability.status,
  };
}

export function formatTerminalCommandSubmitInput(draft: string): string {
  return `${draft}\r`;
}

function normalizeRecentCommandLimit(value: number | null | undefined): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return TERMINAL_COMMAND_DOCK_DEFAULT_RECENT_COMMAND_LIMIT;
  }

  return Math.max(0, Math.trunc(value));
}

function hasFocusedPaneTerminalHistory(snapshot: WorkspaceSnapshot, paneId: string | null): boolean {
  if (!paneId) {
    return false;
  }

  const focusedScreen = snapshot.attachedSession?.focused_screen ?? null;
  if (
    focusedScreen?.pane_id === paneId
    && focusedScreen.surface.lines.some((line) => isTerminalHistoryLine(line.text))
  ) {
    return true;
  }

  return Boolean(
    snapshot.historicalPanes?.[paneId]?.lines.some((line) => isTerminalHistoryLine(line)),
  );
}

function isTerminalHistoryLine(line: string): boolean {
  const text = line.trim();
  return text.length > 0 && !isPromptOnlyTerminalLine(text);
}

function isPromptOnlyTerminalLine(text: string): boolean {
  return /(?:^|\s)[#$%>]\s*$/.test(text);
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

function resolveSaveCapability(
  snapshot: WorkspaceSnapshot,
): { canSave: boolean; status: TerminalCommandDockCapabilityStatus } {
  const capability = resolveWorkspaceCapability(snapshot, "explicit_session_save", {
    missingBackend: false,
    pendingCapabilities: false,
  });
  return {
    canSave: capability.enabled,
    status: capability.status,
  };
}
