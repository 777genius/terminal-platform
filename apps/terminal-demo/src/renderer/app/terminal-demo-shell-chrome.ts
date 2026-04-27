import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

export type TerminalDemoShellMode = "launcher" | "terminal";
export type TerminalDemoShellDensity = "browse" | "focus";
export type TerminalDemoShellCanvasTone = "workspace" | "terminal";

export interface TerminalDemoShellChromeState {
  hasActiveSession: boolean;
  mode: TerminalDemoShellMode;
  density: TerminalDemoShellDensity;
  canvasTone: TerminalDemoShellCanvasTone;
  showLauncherPanel: boolean;
  showWorkspaceHero: boolean;
  launcherTitle: string;
  advancedToolsLabel: string;
}

export function resolveTerminalDemoShellChromeState(
  snapshot: Pick<WorkspaceSnapshot, "attachedSession" | "catalog" | "selection">,
): TerminalDemoShellChromeState {
  const activeSessionId =
    snapshot.selection.activeSessionId
    ?? snapshot.catalog.sessions[0]?.session_id
    ?? snapshot.attachedSession?.session.session_id
    ?? null;
  const hasActiveSession = Boolean(activeSessionId);

  return {
    hasActiveSession,
    mode: hasActiveSession ? "terminal" : "launcher",
    density: hasActiveSession ? "focus" : "browse",
    canvasTone: hasActiveSession ? "terminal" : "workspace",
    showLauncherPanel: !hasActiveSession,
    showWorkspaceHero: !hasActiveSession,
    launcherTitle: "Session launcher",
    advancedToolsLabel: "Advanced tools",
  };
}
