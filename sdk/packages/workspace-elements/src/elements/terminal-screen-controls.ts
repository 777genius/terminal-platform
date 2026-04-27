import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import { resolveWorkspaceCapability, type TerminalWorkspaceCapabilityStatus } from "./terminal-workspace-capabilities.js";

type FocusedScreen = NonNullable<WorkspaceSnapshot["attachedSession"]>["focused_screen"];

export interface TerminalScreenControlState {
  activeSessionId: string | null;
  activePaneId: string | null;
  screen: FocusedScreen | null;
  canCopyVisibleOutput: boolean;
  canUseDirectInput: boolean;
  canUseDirectPaste: boolean;
  inputCapabilityStatus: TerminalWorkspaceCapabilityStatus;
  pasteCapabilityStatus: TerminalWorkspaceCapabilityStatus;
}

export function resolveTerminalScreenControlState(snapshot: WorkspaceSnapshot): TerminalScreenControlState {
  const screen = snapshot.attachedSession?.focused_screen ?? null;
  const activeSessionId = snapshot.selection.activeSessionId ?? snapshot.attachedSession?.session.session_id ?? null;
  const activePaneId = snapshot.selection.activePaneId ?? screen?.pane_id ?? null;
  const inputCapability = resolveWorkspaceCapability(snapshot, "pane_input_write", {
    missingBackend: false,
    pendingCapabilities: true,
  });
  const pasteCapability = resolveWorkspaceCapability(snapshot, "pane_paste_write", {
    missingBackend: false,
    pendingCapabilities: true,
  });
  const hasInputTarget = Boolean(screen && activeSessionId && activePaneId);

  return {
    activeSessionId,
    activePaneId,
    screen,
    canCopyVisibleOutput: Boolean(screen),
    canUseDirectInput: Boolean(hasInputTarget && inputCapability.enabled),
    canUseDirectPaste: Boolean(hasInputTarget && pasteCapability.enabled),
    inputCapabilityStatus: inputCapability.status,
    pasteCapabilityStatus: pasteCapability.status,
  };
}
