export const TERMINAL_WORKSPACE_INSPECTOR_MODES = {
  collapsed: "collapsed",
  hidden: "hidden",
  inline: "inline",
} as const;

export const TERMINAL_WORKSPACE_NAVIGATION_MODES = {
  collapsed: "collapsed",
  hidden: "hidden",
  inline: "inline",
} as const;

export type TerminalWorkspaceInspectorMode =
  (typeof TERMINAL_WORKSPACE_INSPECTOR_MODES)[keyof typeof TERMINAL_WORKSPACE_INSPECTOR_MODES];

export type TerminalWorkspaceNavigationMode =
  (typeof TERMINAL_WORKSPACE_NAVIGATION_MODES)[keyof typeof TERMINAL_WORKSPACE_NAVIGATION_MODES];

export interface TerminalWorkspaceInspectorState {
  mode: TerminalWorkspaceInspectorMode;
  renderCollapsedInspector: boolean;
  renderInlineInspector: boolean;
  renderInspector: boolean;
  summaryLabel: string;
}

export interface TerminalWorkspaceNavigationState {
  mode: TerminalWorkspaceNavigationMode;
  renderCollapsedNavigation: boolean;
  renderInlineNavigation: boolean;
  renderNavigation: boolean;
  summaryLabel: string;
}

export function resolveTerminalWorkspaceInspectorState(
  requestedMode: string | null | undefined,
): TerminalWorkspaceInspectorState {
  const mode = normalizeTerminalWorkspaceInspectorMode(requestedMode);

  return {
    mode,
    renderCollapsedInspector: mode === TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed,
    renderInlineInspector: mode === TERMINAL_WORKSPACE_INSPECTOR_MODES.inline,
    renderInspector: mode !== TERMINAL_WORKSPACE_INSPECTOR_MODES.hidden,
    summaryLabel: "Layout and tools",
  };
}

export function resolveTerminalWorkspaceNavigationState(
  requestedMode: string | null | undefined,
): TerminalWorkspaceNavigationState {
  const mode = normalizeTerminalWorkspaceNavigationMode(requestedMode);

  return {
    mode,
    renderCollapsedNavigation: mode === TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed,
    renderInlineNavigation: mode === TERMINAL_WORKSPACE_NAVIGATION_MODES.inline,
    renderNavigation: mode !== TERMINAL_WORKSPACE_NAVIGATION_MODES.hidden,
    summaryLabel: "Sessions and saved layouts",
  };
}

function normalizeTerminalWorkspaceInspectorMode(
  requestedMode: string | null | undefined,
): TerminalWorkspaceInspectorMode {
  const normalized = requestedMode?.trim();
  if (
    normalized === TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed
    || normalized === TERMINAL_WORKSPACE_INSPECTOR_MODES.hidden
    || normalized === TERMINAL_WORKSPACE_INSPECTOR_MODES.inline
  ) {
    return normalized;
  }

  return TERMINAL_WORKSPACE_INSPECTOR_MODES.inline;
}

function normalizeTerminalWorkspaceNavigationMode(
  requestedMode: string | null | undefined,
): TerminalWorkspaceNavigationMode {
  const normalized = requestedMode?.trim();
  if (
    normalized === TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed
    || normalized === TERMINAL_WORKSPACE_NAVIGATION_MODES.hidden
    || normalized === TERMINAL_WORKSPACE_NAVIGATION_MODES.inline
  ) {
    return normalized;
  }

  return TERMINAL_WORKSPACE_NAVIGATION_MODES.inline;
}
