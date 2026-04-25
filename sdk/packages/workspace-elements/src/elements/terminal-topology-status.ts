import type { TerminalTopologyControlState } from "./terminal-topology-controls.js";

export interface TerminalTopologyStatus {
  readonly label: string;
  readonly tone: "idle" | "limited" | "pending" | "ready";
  readonly title: string;
  readonly capabilityLabel: string;
  readonly canMutateLayout: boolean;
}

export function resolveTerminalTopologyStatus(
  controls: TerminalTopologyControlState,
): TerminalTopologyStatus {
  if (!controls.activeSessionId) {
    return {
      label: "Pick a session",
      tone: "idle",
      title: "Select or start a session before running layout commands.",
      capabilityLabel: "No backend",
      canMutateLayout: false,
    };
  }

  if (!controls.activeTab) {
    return {
      label: "No topology",
      tone: "idle",
      title: "The active session has no tab topology snapshot yet.",
      capabilityLabel: capabilityLabel(controls),
      canMutateLayout: false,
    };
  }

  const canMutateLayout = Boolean(
    controls.canCreateTab
    || controls.canSplitPane
    || controls.canRenameTab
    || controls.canResizePane
    || controls.canClosePane
    || controls.canCloseTab,
  );

  if (controls.capabilityStatus === "unknown") {
    return {
      label: "Topology pending",
      tone: "pending",
      title: "Backend topology capabilities are still loading.",
      capabilityLabel: capabilityLabel(controls),
      canMutateLayout,
    };
  }

  if (!canMutateLayout) {
    return {
      label: "Read only layout",
      tone: "limited",
      title: "The active backend does not expose layout mutation commands.",
      capabilityLabel: capabilityLabel(controls),
      canMutateLayout,
    };
  }

  return {
    label: "Topology ready",
    tone: "ready",
    title: "Focused session accepts topology commands.",
    capabilityLabel: capabilityLabel(controls),
    canMutateLayout,
  };
}

function capabilityLabel(controls: TerminalTopologyControlState): string {
  return controls.capabilityStatus === "known" ? "Capabilities known" : "Capabilities pending";
}
