import type { TerminalCommandDockControlState } from "./terminal-command-dock-controls.js";

export const TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS = {
  clearCommandHistory: "clear-command-history",
  refreshTerminal: "refresh-terminal",
  saveLayout: "save-layout",
} as const;

export type TerminalCommandDockSessionActionId =
  (typeof TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS)[keyof typeof TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS];

export type TerminalCommandDockSessionActionPlacement = "panel" | "terminal";

export interface TerminalCommandDockSessionActionOptions {
  historyClearConfirmationArmed?: boolean;
  pending?: boolean;
  placement?: string | null | undefined;
}

export interface TerminalCommandDockSessionActionPresentation {
  readonly ariaLabel: string;
  readonly confirming: boolean;
  readonly dangerous: boolean;
  readonly disabled: boolean;
  readonly historyCount: number | null;
  readonly id: TerminalCommandDockSessionActionId;
  readonly label: string;
  readonly testId: string;
  readonly title: string;
}

export function resolveTerminalCommandDockSessionActions(
  controls: TerminalCommandDockControlState,
  options: TerminalCommandDockSessionActionOptions = {},
): readonly TerminalCommandDockSessionActionPresentation[] {
  const placement = normalizeTerminalCommandDockSessionActionPlacement(options.placement);
  const compact = placement === "terminal";
  const pending = options.pending === true;
  const historyCount = controls.commandHistory.length;
  const historyCountLabel = formatCommandHistoryCount(historyCount);
  const confirmingHistoryClear = Boolean(
    options.historyClearConfirmationArmed
    && historyCount > 0
    && !pending
  );

  return [
    {
      id: TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.saveLayout,
      testId: "tp-save-layout",
      label: compact ? "Save" : "Save layout",
      title: resolveSaveLayoutTitle(controls),
      ariaLabel: "Save the focused session layout",
      disabled: !controls.canSaveLayout,
      dangerous: false,
      confirming: false,
      historyCount: null,
    },
    {
      id: TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.refreshTerminal,
      testId: "tp-refresh-terminal",
      label: compact ? "Refresh" : "Refresh terminal",
      title: "Refresh the active terminal session",
      ariaLabel: "Refresh the active terminal session",
      disabled: !controls.activeSessionId || pending,
      dangerous: false,
      confirming: false,
      historyCount: null,
    },
    {
      id: TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.clearCommandHistory,
      testId: "tp-clear-command-history",
      label: confirmingHistoryClear
        ? `Confirm clear ${historyCount}`
        : compact
          ? "Clear"
          : "Clear history",
      title: confirmingHistoryClear
        ? `Confirm clearing ${historyCountLabel}`
        : `Clear ${historyCountLabel}`,
      ariaLabel: confirmingHistoryClear
        ? `Confirm clearing ${historyCountLabel}`
        : `Clear ${historyCountLabel}`,
      disabled: historyCount === 0 || pending,
      dangerous: true,
      confirming: confirmingHistoryClear,
      historyCount,
    },
  ];
}

function resolveSaveLayoutTitle(controls: TerminalCommandDockControlState): string {
  if (controls.saveCapabilityStatus === "known" && !controls.canSaveLayout) {
    return "Save layout is not supported by the active backend";
  }

  if (controls.saveCapabilityStatus === "unknown") {
    return "Save layout is disabled until backend capabilities load";
  }

  return "Save the focused session layout";
}

function normalizeTerminalCommandDockSessionActionPlacement(
  placement: string | null | undefined,
): TerminalCommandDockSessionActionPlacement {
  return placement === "terminal" ? "terminal" : "panel";
}

function formatCommandHistoryCount(count: number): string {
  return `${count} command history ${count === 1 ? "entry" : "entries"}`;
}
