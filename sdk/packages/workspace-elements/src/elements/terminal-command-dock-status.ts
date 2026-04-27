import type { TerminalCommandDockControlState } from "./terminal-command-dock-controls.js";
import type { TerminalCommandInputStatus } from "./terminal-command-input-status.js";
import { resolveTerminalEntityIdLabel } from "./terminal-identity.js";

export const TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS = {
  activePane: "active-pane",
  historyCount: "history-count",
  input: "input",
} as const;

export type TerminalCommandDockStatusBadgeId =
  (typeof TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS)[keyof typeof TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS];

export type TerminalCommandDockStatusPlacement = "panel" | "terminal";

export type TerminalCommandDockStatusTone = "idle" | "pending" | "ready";

export interface TerminalCommandDockStatusOptions {
  placement?: string | null;
}

export interface TerminalCommandDockStatusBadge {
  readonly id: TerminalCommandDockStatusBadgeId;
  readonly label: string;
  readonly testId: string;
  readonly title: string;
  readonly tone: TerminalCommandDockStatusTone;
}

export function resolveTerminalCommandDockStatusBadges(
  controls: TerminalCommandDockControlState,
  inputStatus: TerminalCommandInputStatus,
  options: TerminalCommandDockStatusOptions = {},
): readonly TerminalCommandDockStatusBadge[] {
  const placement = resolveTerminalCommandDockStatusPlacement(options.placement);
  const activePaneIdentity = controls.activePaneId
    ? resolveTerminalEntityIdLabel(controls.activePaneId, { prefix: "Pane" })
    : null;

  return [
    {
      id: TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.activePane,
      label: activePaneIdentity?.label ?? "No pane",
      testId: "tp-command-active-pane",
      title: activePaneIdentity?.title ?? "",
      tone: controls.activePaneId ? "ready" : "idle",
    },
    {
      id: TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.input,
      label: inputStatus.label,
      testId: "tp-command-input-status",
      title: inputStatus.title,
      tone: inputStatus.tone,
    },
    {
      id: TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.historyCount,
      label: formatCommandHistoryBadgeLabel(controls.commandHistory.length, placement),
      testId: "tp-command-history-count",
      title: formatCommandHistoryCountTitle(controls.commandHistory.length),
      tone: "idle",
    },
  ];
}

export function resolveTerminalCommandDockStatusPlacement(
  placement: string | null | undefined,
): TerminalCommandDockStatusPlacement {
  return placement === "terminal" ? "terminal" : "panel";
}

function formatCommandHistoryBadgeLabel(
  count: number,
  placement: TerminalCommandDockStatusPlacement,
): string {
  return placement === "terminal"
    ? `${count} cmd`
    : `${count} history`;
}

function formatCommandHistoryCountTitle(count: number): string {
  return `${count} command history ${count === 1 ? "entry" : "entries"}`;
}
