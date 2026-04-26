export const TERMINAL_WORKSPACE_INSPECTOR_MODES = {
  collapsed: "collapsed",
  hidden: "hidden",
  inline: "inline",
} as const;

export type TerminalWorkspaceInspectorMode =
  (typeof TERMINAL_WORKSPACE_INSPECTOR_MODES)[keyof typeof TERMINAL_WORKSPACE_INSPECTOR_MODES];

export interface TerminalWorkspaceInspectorState {
  mode: TerminalWorkspaceInspectorMode;
  renderCollapsedInspector: boolean;
  renderInlineInspector: boolean;
  renderInspector: boolean;
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
