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

export const TERMINAL_WORKSPACE_CHROME_TONES = {
  terminal: "terminal",
  workspace: "workspace",
} as const;

export const TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES = {
  panel: "panel",
  terminal: "terminal",
} as const;

export const TERMINAL_WORKSPACE_SECONDARY_DENSITIES = {
  compact: "compact",
  regular: "regular",
} as const;

export type TerminalWorkspaceInspectorMode =
  (typeof TERMINAL_WORKSPACE_INSPECTOR_MODES)[keyof typeof TERMINAL_WORKSPACE_INSPECTOR_MODES];

export type TerminalWorkspaceNavigationMode =
  (typeof TERMINAL_WORKSPACE_NAVIGATION_MODES)[keyof typeof TERMINAL_WORKSPACE_NAVIGATION_MODES];

export type TerminalWorkspaceLayoutPreset =
  (typeof TERMINAL_WORKSPACE_LAYOUT_PRESETS)[keyof typeof TERMINAL_WORKSPACE_LAYOUT_PRESETS];

export type TerminalWorkspaceChromeTone =
  (typeof TERMINAL_WORKSPACE_CHROME_TONES)[keyof typeof TERMINAL_WORKSPACE_CHROME_TONES];

export type TerminalWorkspaceSecondaryChromeMode =
  (typeof TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES)[keyof typeof TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES];

export type TerminalWorkspaceSecondaryDensity =
  (typeof TERMINAL_WORKSPACE_SECONDARY_DENSITIES)[keyof typeof TERMINAL_WORKSPACE_SECONDARY_DENSITIES];

export interface TerminalWorkspaceInspectorState {
  mode: TerminalWorkspaceInspectorMode;
  renderCollapsedInspector: boolean;
  renderInlineInspector: boolean;
  renderInspector: boolean;
  summaryActionClosedLabel: string;
  summaryActionOpenLabel: string;
  summaryLabel: string;
}

export interface TerminalWorkspaceNavigationState {
  mode: TerminalWorkspaceNavigationMode;
  renderCollapsedNavigation: boolean;
  renderInlineNavigation: boolean;
  renderNavigation: boolean;
  summaryActionClosedLabel: string;
  summaryActionOpenLabel: string;
  summaryLabel: string;
}

export interface TerminalWorkspaceChromeState {
  tone: TerminalWorkspaceChromeTone;
  secondaryChrome: TerminalWorkspaceSecondaryChromeMode;
  secondaryDensity: TerminalWorkspaceSecondaryDensity;
}

export interface TerminalWorkspaceLayoutState {
  preset: TerminalWorkspaceLayoutPreset;
  chrome: TerminalWorkspaceChromeState;
  inspector: TerminalWorkspaceInspectorState;
  navigation: TerminalWorkspaceNavigationState;
}

export interface TerminalWorkspaceLayoutOptions {
  layoutPreset?: string | null | undefined;
  inspectorMode?: string | null | undefined;
  navigationMode?: string | null | undefined;
}

export interface TerminalWorkspaceSecondarySummaryOptions {
  summaryLabel?: string | null | undefined;
}

export function resolveTerminalWorkspaceLayoutState(
  options: TerminalWorkspaceLayoutOptions = {},
): TerminalWorkspaceLayoutState {
  const preset = normalizeTerminalWorkspaceLayoutPreset(options.layoutPreset);

  if (preset === TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal) {
    return {
      preset,
      chrome: resolveTerminalWorkspaceChromeState(preset),
      inspector: resolveTerminalWorkspaceInspectorState(
        TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed,
        { summaryLabel: "Tools" },
      ),
      navigation: resolveTerminalWorkspaceNavigationState(
        TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed,
        { summaryLabel: "Sessions" },
      ),
    };
  }

  return {
    preset,
    chrome: resolveTerminalWorkspaceChromeState(preset),
    inspector: resolveTerminalWorkspaceInspectorState(options.inspectorMode),
    navigation: resolveTerminalWorkspaceNavigationState(options.navigationMode),
  };
}

export function resolveTerminalWorkspaceChromeState(
  layoutPreset: string | null | undefined,
): TerminalWorkspaceChromeState {
  const preset = normalizeTerminalWorkspaceLayoutPreset(layoutPreset);

  if (preset === TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal) {
    return {
      tone: TERMINAL_WORKSPACE_CHROME_TONES.terminal,
      secondaryChrome: TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES.terminal,
      secondaryDensity: TERMINAL_WORKSPACE_SECONDARY_DENSITIES.compact,
    };
  }

  return {
    tone: TERMINAL_WORKSPACE_CHROME_TONES.workspace,
    secondaryChrome: TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES.panel,
    secondaryDensity: TERMINAL_WORKSPACE_SECONDARY_DENSITIES.regular,
  };
}

export function resolveTerminalWorkspaceInspectorState(
  requestedMode: string | null | undefined,
  options: TerminalWorkspaceSecondarySummaryOptions = {},
): TerminalWorkspaceInspectorState {
  const mode = normalizeTerminalWorkspaceInspectorMode(requestedMode);

  return {
    mode,
    renderCollapsedInspector: mode === TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed,
    renderInlineInspector: mode === TERMINAL_WORKSPACE_INSPECTOR_MODES.inline,
    renderInspector: mode !== TERMINAL_WORKSPACE_INSPECTOR_MODES.hidden,
    summaryActionClosedLabel: "Open",
    summaryActionOpenLabel: "Close",
    summaryLabel: normalizeTerminalWorkspaceSummaryLabel(options.summaryLabel, "Layout and tools"),
  };
}

export function resolveTerminalWorkspaceNavigationState(
  requestedMode: string | null | undefined,
  options: TerminalWorkspaceSecondarySummaryOptions = {},
): TerminalWorkspaceNavigationState {
  const mode = normalizeTerminalWorkspaceNavigationMode(requestedMode);

  return {
    mode,
    renderCollapsedNavigation: mode === TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed,
    renderInlineNavigation: mode === TERMINAL_WORKSPACE_NAVIGATION_MODES.inline,
    renderNavigation: mode !== TERMINAL_WORKSPACE_NAVIGATION_MODES.hidden,
    summaryActionClosedLabel: "Open",
    summaryActionOpenLabel: "Close",
    summaryLabel: normalizeTerminalWorkspaceSummaryLabel(options.summaryLabel, "Sessions and saved layouts"),
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

function normalizeTerminalWorkspaceSummaryLabel(
  requestedLabel: string | null | undefined,
  fallback: string,
): string {
  const normalized = requestedLabel?.trim();
  return normalized ? normalized : fallback;
}
