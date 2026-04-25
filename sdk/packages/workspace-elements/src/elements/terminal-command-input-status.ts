import type { TerminalCommandDockControlState } from "./terminal-command-dock-controls.js";

export interface TerminalCommandInputStatus {
  readonly label: string;
  readonly tone: "idle" | "pending" | "ready";
  readonly title: string;
  readonly placeholder: string;
  readonly hint: string;
}

export function resolveTerminalCommandInputStatus(
  controls: TerminalCommandDockControlState,
): TerminalCommandInputStatus {
  if (!controls.activeSessionId || !controls.activePaneId) {
    return {
      label: "Pick a pane",
      tone: "idle",
      title: "Start or select a session, then choose a pane.",
      placeholder: "Select a pane first",
      hint: "Start or select a session, then choose a pane to enable input.",
    };
  }

  if (!controls.canUsePane) {
    return {
      label: "Sending",
      tone: "pending",
      title: "Command input is busy.",
      placeholder: "Command input is busy",
      hint: "Command input is busy. Wait for the current action to settle.",
    };
  }

  if (!controls.canWriteInput) {
    return {
      label: "Read only",
      tone: "idle",
      title: "The active backend does not support focused pane input writes.",
      placeholder: "Input is unavailable for this backend",
      hint: "The active backend is read only for command input.",
    };
  }

  if (controls.inputCapabilityStatus === "unknown") {
    return {
      label: "Input pending",
      tone: "pending",
      title: "Backend input capability is still loading.",
      placeholder: "Type shell input while capabilities load",
      hint: "Input capability is still loading. Commands are accepted optimistically.",
    };
  }

  return {
    label: "Ready",
    tone: "ready",
    title: "Focused pane accepts command input.",
    placeholder: "Type shell input for the focused pane",
    hint: "Enter sends the command. Shift+Enter inserts a newline.",
  };
}
