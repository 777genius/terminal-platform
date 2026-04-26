export const TERMINAL_PLATFORM_THEME_ATTRIBUTE = "data-tp-theme" as const;

export interface ThemeManifest {
  id: string;
  displayName: string;
  tokens: Record<string, string>;
}

export const terminalPlatformDefaultTheme: ThemeManifest = {
  id: "terminal-platform-default",
  displayName: "Terminal Platform Default",
  tokens: {
    "--tp-color-bg": "#0f1117",
    "--tp-color-bg-inset": "#090c12",
    "--tp-color-panel": "#171b24",
    "--tp-color-panel-raised": "#1d2330",
    "--tp-color-border": "#2a3242",
    "--tp-color-border-strong": "#3b475c",
    "--tp-color-text": "#e8edf6",
    "--tp-color-text-muted": "#9ba7bd",
    "--tp-color-accent": "#7dd3fc",
    "--tp-color-accent-soft": "rgba(125, 211, 252, 0.14)",
    "--tp-color-success": "#86efac",
    "--tp-color-warning": "#fcd34d",
    "--tp-color-danger": "#fca5a5",
    "--tp-color-danger-soft": "rgba(252, 165, 165, 0.14)",
    "--tp-terminal-color-bg": "#05070b",
    "--tp-terminal-color-bg-raised": "#0b111a",
    "--tp-terminal-color-border": "#263247",
    "--tp-terminal-color-text": "#e8edf6",
    "--tp-terminal-color-text-muted": "#9ba7bd",
    "--tp-terminal-color-accent": "#7dd3fc",
    "--tp-font-family-ui": "\"Inter\", \"Avenir Next\", \"Segoe UI\", sans-serif",
    "--tp-font-family-mono": "\"Berkeley Mono\", \"JetBrains Mono\", monospace",
    "--tp-radius-sm": "6px",
    "--tp-radius-md": "8px",
    "--tp-radius-lg": "8px",
    "--tp-space-2": "0.5rem",
    "--tp-space-3": "0.75rem",
    "--tp-space-4": "1rem",
    "--tp-space-5": "1.25rem",
    "--tp-shadow-panel": "0 18px 60px rgba(0, 0, 0, 0.28)",
  },
};

export const terminalPlatformLightTheme: ThemeManifest = {
  id: "terminal-platform-light",
  displayName: "Terminal Platform Light",
  tokens: {
    "--tp-color-bg": "#f6f8fb",
    "--tp-color-bg-inset": "#e8edf5",
    "--tp-color-panel": "#ffffff",
    "--tp-color-panel-raised": "#eef3f8",
    "--tp-color-border": "#cfd8e5",
    "--tp-color-border-strong": "#9aa8bb",
    "--tp-color-text": "#172033",
    "--tp-color-text-muted": "#657086",
    "--tp-color-accent": "#0f7ea8",
    "--tp-color-accent-soft": "rgba(15, 126, 168, 0.12)",
    "--tp-color-success": "#1f8f55",
    "--tp-color-warning": "#a15c06",
    "--tp-color-danger": "#c23838",
    "--tp-color-danger-soft": "rgba(194, 56, 56, 0.12)",
    "--tp-terminal-color-bg": "#05070b",
    "--tp-terminal-color-bg-raised": "#0d1320",
    "--tp-terminal-color-border": "#334155",
    "--tp-terminal-color-text": "#f4f7fb",
    "--tp-terminal-color-text-muted": "#aab5c7",
    "--tp-terminal-color-accent": "#38bdf8",
    "--tp-font-family-ui": "\"Inter\", \"Avenir Next\", \"Segoe UI\", sans-serif",
    "--tp-font-family-mono": "\"Berkeley Mono\", \"JetBrains Mono\", monospace",
    "--tp-radius-sm": "6px",
    "--tp-radius-md": "8px",
    "--tp-radius-lg": "8px",
    "--tp-space-2": "0.5rem",
    "--tp-space-3": "0.75rem",
    "--tp-space-4": "1rem",
    "--tp-space-5": "1.25rem",
    "--tp-shadow-panel": "0 18px 48px rgba(23, 32, 51, 0.12)",
  },
};

export const terminalPlatformThemeManifests = [
  terminalPlatformDefaultTheme,
  terminalPlatformLightTheme,
] as const;

export const terminalPlatformThemeCssText = [
  createThemeCssRule(":host, :root", terminalPlatformDefaultTheme),
  ...terminalPlatformThemeManifests.map((theme) =>
    createThemeCssRule(
      `:host([${TERMINAL_PLATFORM_THEME_ATTRIBUTE}="${theme.id}"]), :root[${TERMINAL_PLATFORM_THEME_ATTRIBUTE}="${theme.id}"]`,
      theme,
    ),
  ),
].join("\n\n");

export const terminalPlatformDefaultThemeCssText = terminalPlatformThemeCssText;

function createThemeCssRule(selector: string, theme: ThemeManifest): string {
  const declarations = Object.entries(theme.tokens)
    .map(([token, value]) => `  ${token}: ${value};`)
    .join("\n");

  return `${selector} {\n${declarations}\n}`;
}
