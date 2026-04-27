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

export const TERMINAL_WORKSPACE_LAYOUT_PRESETS = {
  classic: "classic",
  terminal: "terminal",
} as const;

export type TerminalWorkspaceInspectorMode =
  (typeof TERMINAL_WORKSPACE_INSPECTOR_MODES)[keyof typeof TERMINAL_WORKSPACE_INSPECTOR_MODES];

export type TerminalWorkspaceNavigationMode =
  (typeof TERMINAL_WORKSPACE_NAVIGATION_MODES)[keyof typeof TERMINAL_WORKSPACE_NAVIGATION_MODES];

export type TerminalWorkspaceLayoutPreset =
  (typeof TERMINAL_WORKSPACE_LAYOUT_PRESETS)[keyof typeof TERMINAL_WORKSPACE_LAYOUT_PRESETS];

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

export interface TerminalWorkspaceLayoutState {
  preset: TerminalWorkspaceLayoutPreset;
  inspector: TerminalWorkspaceInspectorState;
  navigation: TerminalWorkspaceNavigationState;
}

export interface TerminalWorkspaceLayoutOptions {
  layoutPreset?: string | null | undefined;
  inspectorMode?: string | null | undefined;
  navigationMode?: string | null | undefined;
}

export function resolveTerminalWorkspaceLayoutState(
  options: TerminalWorkspaceLayoutOptions = {},
): TerminalWorkspaceLayoutState {
  const preset = normalizeTerminalWorkspaceLayoutPreset(options.layoutPreset);

  if (preset === TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal) {
    return {
      preset,
      inspector: resolveTerminalWorkspaceInspectorState(TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed),
      navigation: resolveTerminalWorkspaceNavigationState(TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed),
    };
  }

  return {
    preset,
    inspector: resolveTerminalWorkspaceInspectorState(options.inspectorMode),
    navigation: resolveTerminalWorkspaceNavigationState(options.navigationMode),
  };
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

function normalizeTerminalWorkspaceLayoutPreset(
  requestedPreset: string | null | undefined,
): TerminalWorkspaceLayoutPreset {
  const normalized = requestedPreset?.trim();
  if (
    normalized === TERMINAL_WORKSPACE_LAYOUT_PRESETS.classic
    || normalized === TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal
  ) {
    return normalized;
  }

  return TERMINAL_WORKSPACE_LAYOUT_PRESETS.classic;
}
