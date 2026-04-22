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
    "--tp-color-panel": "#171b24",
    "--tp-color-border": "#2a3242",
    "--tp-color-text": "#e8edf6",
    "--tp-color-text-muted": "#9ba7bd",
    "--tp-color-accent": "#7dd3fc",
    "--tp-color-success": "#86efac",
    "--tp-color-warning": "#fcd34d",
    "--tp-color-danger": "#fca5a5",
    "--tp-font-family-mono": "\"Berkeley Mono\", \"JetBrains Mono\", monospace",
    "--tp-radius-sm": "6px",
    "--tp-radius-md": "10px",
    "--tp-space-2": "0.5rem",
    "--tp-space-3": "0.75rem",
    "--tp-space-4": "1rem",
  },
};

export const terminalPlatformThemeManifests = [terminalPlatformDefaultTheme] as const;

export const terminalPlatformDefaultThemeCssText = `:host, :root {
  --tp-color-bg: ${terminalPlatformDefaultTheme.tokens["--tp-color-bg"]};
  --tp-color-panel: ${terminalPlatformDefaultTheme.tokens["--tp-color-panel"]};
  --tp-color-border: ${terminalPlatformDefaultTheme.tokens["--tp-color-border"]};
  --tp-color-text: ${terminalPlatformDefaultTheme.tokens["--tp-color-text"]};
  --tp-color-text-muted: ${terminalPlatformDefaultTheme.tokens["--tp-color-text-muted"]};
  --tp-color-accent: ${terminalPlatformDefaultTheme.tokens["--tp-color-accent"]};
  --tp-color-success: ${terminalPlatformDefaultTheme.tokens["--tp-color-success"]};
  --tp-color-warning: ${terminalPlatformDefaultTheme.tokens["--tp-color-warning"]};
  --tp-color-danger: ${terminalPlatformDefaultTheme.tokens["--tp-color-danger"]};
  --tp-font-family-mono: ${terminalPlatformDefaultTheme.tokens["--tp-font-family-mono"]};
  --tp-radius-sm: ${terminalPlatformDefaultTheme.tokens["--tp-radius-sm"]};
  --tp-radius-md: ${terminalPlatformDefaultTheme.tokens["--tp-radius-md"]};
  --tp-space-2: ${terminalPlatformDefaultTheme.tokens["--tp-space-2"]};
  --tp-space-3: ${terminalPlatformDefaultTheme.tokens["--tp-space-3"]};
  --tp-space-4: ${terminalPlatformDefaultTheme.tokens["--tp-space-4"]};
}`;
