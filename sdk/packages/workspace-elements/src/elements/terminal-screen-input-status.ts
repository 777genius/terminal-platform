import type { TerminalScreenControlState } from "./terminal-screen-controls.js";

export type TerminalScreenInputActivity = "idle" | "failed";
export type TerminalScreenInputTone = "ready" | "pending" | "readonly" | "failed";

export interface TerminalScreenInputStatus {
  readonly label: string;
  readonly tone: TerminalScreenInputTone;
  readonly title: string;
}

export function resolveTerminalScreenInputStatus(
  controls: TerminalScreenControlState,
  activity: TerminalScreenInputActivity,
): TerminalScreenInputStatus {
  if (activity === "failed") {
    return {
      label: "Input failed",
      tone: "failed",
      title: "Last focused pane input failed. Try again or refresh the session.",
    };
  }

  if (!controls.screen || !controls.activeSessionId || !controls.activePaneId) {
    return {
      label: "No input",
      tone: "readonly",
      title: "Attach a session with a focused pane to enable input.",
    };
  }

  if (!controls.canUseDirectInput) {
    return {
      label: "Read only",
      tone: "readonly",
      title: "The active backend does not support direct focused pane input.",
    };
  }

  if (controls.inputCapabilityStatus === "unknown") {
    return {
      label: "Input pending",
      tone: "pending",
      title: "Backend input capability is still loading.",
    };
  }

  return {
    label: "Input ready",
    tone: "ready",
    title: "Focused pane accepts keyboard input.",
  };
}
