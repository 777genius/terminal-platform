import type { ThemeManifest } from "@terminal-platform/design-tokens";
import type { TerminalPlatformTerminalFontScale } from "@terminal-platform/workspace-core";

export interface TerminalToolbarThemeOptionPresentation {
  isActive: boolean;
  label: string;
  title: string;
}

export interface TerminalToolbarFontScaleOptionPresentation {
  isActive: boolean;
  label: string;
  title: string;
}

export interface TerminalToolbarLineWrapOptionPresentation {
  isActive: boolean;
  label: string;
  nextValue: boolean;
  title: string;
}

const TERMINAL_TOOLBAR_THEME_NAME_PREFIX = "Terminal Platform ";

const fontScaleLabels: Record<TerminalPlatformTerminalFontScale, string> = {
  compact: "Compact",
  default: "Default",
  large: "Large",
};

export function resolveTerminalToolbarThemeOption(
  theme: Pick<ThemeManifest, "displayName" | "id">,
  activeThemeId: string,
): TerminalToolbarThemeOptionPresentation {
  const label = resolveThemeLabel(theme.displayName);
  const isActive = theme.id === activeThemeId;

  return {
    isActive,
    label,
    title: isActive ? `${label} theme is active.` : `Switch workspace theme to ${label}.`,
  };
}

export function resolveTerminalToolbarFontScaleOption(
  fontScale: TerminalPlatformTerminalFontScale,
  activeFontScale: TerminalPlatformTerminalFontScale,
): TerminalToolbarFontScaleOptionPresentation {
  const label = fontScaleLabels[fontScale];
  const normalizedLabel = label.toLowerCase();
  const isActive = fontScale === activeFontScale;

  return {
    isActive,
    label,
    title: isActive
      ? `${label} terminal font size is active.`
      : `Set terminal font size to ${normalizedLabel}.`,
  };
}

export function resolveTerminalToolbarLineWrapOption(
  lineWrap: boolean,
): TerminalToolbarLineWrapOptionPresentation {
  return {
    isActive: lineWrap,
    label: lineWrap ? "Wrap on" : "Wrap off",
    nextValue: !lineWrap,
    title: lineWrap ? "Disable terminal line wrapping." : "Enable terminal line wrapping.",
  };
}

function resolveThemeLabel(displayName: string): string {
  const label = displayName.startsWith(TERMINAL_TOOLBAR_THEME_NAME_PREFIX)
    ? displayName.slice(TERMINAL_TOOLBAR_THEME_NAME_PREFIX.length)
    : displayName;

  return label.trim() || displayName;
}
