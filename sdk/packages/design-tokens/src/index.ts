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
    "--tp-font-family-ui": "\"Inter\", \"Avenir Next\", \"Segoe UI\", sans-serif",
    "--tp-font-family-mono": "\"Berkeley Mono\", \"JetBrains Mono\", monospace",
    "--tp-radius-sm": "6px",
    "--tp-radius-md": "10px",
    "--tp-radius-lg": "14px",
    "--tp-space-2": "0.5rem",
    "--tp-space-3": "0.75rem",
    "--tp-space-4": "1rem",
    "--tp-space-5": "1.25rem",
    "--tp-shadow-panel": "0 18px 60px rgba(0, 0, 0, 0.28)",
  },
};

export const terminalPlatformThemeManifests = [terminalPlatformDefaultTheme] as const;

export const terminalPlatformDefaultThemeCssText = `:host, :root {
  --tp-color-bg: ${terminalPlatformDefaultTheme.tokens["--tp-color-bg"]};
  --tp-color-bg-inset: ${terminalPlatformDefaultTheme.tokens["--tp-color-bg-inset"]};
  --tp-color-panel: ${terminalPlatformDefaultTheme.tokens["--tp-color-panel"]};
  --tp-color-panel-raised: ${terminalPlatformDefaultTheme.tokens["--tp-color-panel-raised"]};
  --tp-color-border: ${terminalPlatformDefaultTheme.tokens["--tp-color-border"]};
  --tp-color-border-strong: ${terminalPlatformDefaultTheme.tokens["--tp-color-border-strong"]};
  --tp-color-text: ${terminalPlatformDefaultTheme.tokens["--tp-color-text"]};
  --tp-color-text-muted: ${terminalPlatformDefaultTheme.tokens["--tp-color-text-muted"]};
  --tp-color-accent: ${terminalPlatformDefaultTheme.tokens["--tp-color-accent"]};
  --tp-color-accent-soft: ${terminalPlatformDefaultTheme.tokens["--tp-color-accent-soft"]};
  --tp-color-success: ${terminalPlatformDefaultTheme.tokens["--tp-color-success"]};
  --tp-color-warning: ${terminalPlatformDefaultTheme.tokens["--tp-color-warning"]};
  --tp-color-danger: ${terminalPlatformDefaultTheme.tokens["--tp-color-danger"]};
  --tp-color-danger-soft: ${terminalPlatformDefaultTheme.tokens["--tp-color-danger-soft"]};
  --tp-font-family-ui: ${terminalPlatformDefaultTheme.tokens["--tp-font-family-ui"]};
  --tp-font-family-mono: ${terminalPlatformDefaultTheme.tokens["--tp-font-family-mono"]};
  --tp-radius-sm: ${terminalPlatformDefaultTheme.tokens["--tp-radius-sm"]};
  --tp-radius-md: ${terminalPlatformDefaultTheme.tokens["--tp-radius-md"]};
  --tp-radius-lg: ${terminalPlatformDefaultTheme.tokens["--tp-radius-lg"]};
  --tp-space-2: ${terminalPlatformDefaultTheme.tokens["--tp-space-2"]};
  --tp-space-3: ${terminalPlatformDefaultTheme.tokens["--tp-space-3"]};
  --tp-space-4: ${terminalPlatformDefaultTheme.tokens["--tp-space-4"]};
  --tp-space-5: ${terminalPlatformDefaultTheme.tokens["--tp-space-5"]};
  --tp-shadow-panel: ${terminalPlatformDefaultTheme.tokens["--tp-shadow-panel"]};
}`;
