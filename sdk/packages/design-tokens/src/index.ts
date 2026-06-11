export const TERMINAL_PLATFORM_THEME_ATTRIBUTE = "data-tp-theme" as const;

export const TERMINAL_PLATFORM_TOKEN_TIERS = {
  semantic: "semantic",
  component: "component",
} as const;

export const TERMINAL_PLATFORM_TOKEN_CATEGORIES = {
  color: "color",
  terminalColor: "terminal-color",
  typography: "typography",
  radius: "radius",
  spacing: "spacing",
  elevation: "elevation",
} as const;

export type TerminalPlatformTokenTier =
  (typeof TERMINAL_PLATFORM_TOKEN_TIERS)[keyof typeof TERMINAL_PLATFORM_TOKEN_TIERS];

export type TerminalPlatformTokenCategory =
  (typeof TERMINAL_PLATFORM_TOKEN_CATEGORIES)[keyof typeof TERMINAL_PLATFORM_TOKEN_CATEGORIES];

export interface TerminalPlatformTokenDefinition {
  readonly name: string;
  readonly tier: TerminalPlatformTokenTier;
  readonly category: TerminalPlatformTokenCategory;
}

export const terminalPlatformTokenDefinitions = [
  { name: "--tp-color-bg", tier: "semantic", category: "color" },
  { name: "--tp-color-bg-inset", tier: "semantic", category: "color" },
  { name: "--tp-color-panel", tier: "semantic", category: "color" },
  { name: "--tp-color-panel-raised", tier: "semantic", category: "color" },
  { name: "--tp-color-border", tier: "semantic", category: "color" },
  { name: "--tp-color-border-strong", tier: "semantic", category: "color" },
  { name: "--tp-color-text", tier: "semantic", category: "color" },
  { name: "--tp-color-text-muted", tier: "semantic", category: "color" },
  { name: "--tp-color-accent", tier: "semantic", category: "color" },
  { name: "--tp-color-accent-soft", tier: "semantic", category: "color" },
  { name: "--tp-color-success", tier: "semantic", category: "color" },
  { name: "--tp-color-warning", tier: "semantic", category: "color" },
  { name: "--tp-color-danger", tier: "semantic", category: "color" },
  { name: "--tp-color-danger-soft", tier: "semantic", category: "color" },
  { name: "--tp-terminal-color-bg", tier: "component", category: "terminal-color" },
  { name: "--tp-terminal-color-bg-raised", tier: "component", category: "terminal-color" },
  { name: "--tp-terminal-color-border", tier: "component", category: "terminal-color" },
  { name: "--tp-terminal-color-text", tier: "component", category: "terminal-color" },
  { name: "--tp-terminal-color-text-muted", tier: "component", category: "terminal-color" },
  { name: "--tp-terminal-color-accent", tier: "component", category: "terminal-color" },
  { name: "--tp-font-family-ui", tier: "semantic", category: "typography" },
  { name: "--tp-font-family-mono", tier: "semantic", category: "typography" },
  { name: "--tp-radius-sm", tier: "semantic", category: "radius" },
  { name: "--tp-radius-md", tier: "semantic", category: "radius" },
  { name: "--tp-radius-lg", tier: "semantic", category: "radius" },
  { name: "--tp-space-2", tier: "semantic", category: "spacing" },
  { name: "--tp-space-3", tier: "semantic", category: "spacing" },
  { name: "--tp-space-4", tier: "semantic", category: "spacing" },
  { name: "--tp-space-5", tier: "semantic", category: "spacing" },
  { name: "--tp-shadow-panel", tier: "semantic", category: "elevation" },
] as const satisfies readonly TerminalPlatformTokenDefinition[];

export type TerminalPlatformTokenName = (typeof terminalPlatformTokenDefinitions)[number]["name"];

export type TerminalPlatformBuiltInThemeTokens = Record<TerminalPlatformTokenName, string>;

export interface ThemeManifest {
  id: string;
  displayName: string;
  tokens: Record<string, string>;
}

export interface TerminalPlatformBuiltInThemeManifest extends ThemeManifest {
  tokens: TerminalPlatformBuiltInThemeTokens;
}

export interface TerminalPlatformThemeCssTextOptions {
  rootSelector?: string;
  themeAttribute?: string;
}

export interface TerminalPlatformThemeCssDeclarationOptions {
  indent?: string;
}

export const terminalPlatformDefaultTheme = {
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
} satisfies TerminalPlatformBuiltInThemeManifest;

export const terminalPlatformLightTheme = {
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
} satisfies TerminalPlatformBuiltInThemeManifest;

export const terminalPlatformThemeManifests = [
  terminalPlatformDefaultTheme,
  terminalPlatformLightTheme,
] as const;

export const terminalPlatformThemeCssText = createTerminalPlatformThemeCssText(terminalPlatformThemeManifests);

export const terminalPlatformDefaultThemeCssText = terminalPlatformThemeCssText;

export function createTerminalPlatformThemeCssText(
  themes: readonly ThemeManifest[] = terminalPlatformThemeManifests,
  options: TerminalPlatformThemeCssTextOptions = {},
): string {
  const rootSelector = options.rootSelector ?? ":host, :root";
  const themeAttribute = options.themeAttribute ?? TERMINAL_PLATFORM_THEME_ATTRIBUTE;

  return [
    createTerminalPlatformThemeCssRule(rootSelector, terminalPlatformDefaultTheme),
    ...themes.map((theme) =>
      createTerminalPlatformThemeCssRule(
        `:host([${themeAttribute}="${theme.id}"]), :root[${themeAttribute}="${theme.id}"]`,
        theme,
      ),
    ),
  ].join("\n\n");
}

export function createTerminalPlatformThemeCssRule(selector: string, theme: ThemeManifest): string {
  return `${selector} {\n${createTerminalPlatformThemeCssDeclarations(theme)}\n}`;
}

export function createTerminalPlatformThemeCssDeclarations(
  theme: Pick<ThemeManifest, "tokens">,
  options: TerminalPlatformThemeCssDeclarationOptions = {},
): string {
  const indent = options.indent ?? "  ";

  return listTerminalPlatformThemeTokenEntries(theme)
    .map(([token, value]) => `${indent}${token}: ${value};`)
    .join("\n");
}

export function listTerminalPlatformThemeTokenEntries(
  theme: Pick<ThemeManifest, "tokens">,
): Array<readonly [string, string]> {
  const knownTokens = new Set<string>();
  const orderedEntries: Array<readonly [string, string]> = [];

  for (const definition of terminalPlatformTokenDefinitions) {
    knownTokens.add(definition.name);
    const value = theme.tokens[definition.name];
    if (typeof value === "string") {
      orderedEntries.push([definition.name, value]);
    }
  }

  const extensionEntries = Object.entries(theme.tokens)
    .filter(([token]) => !knownTokens.has(token))
    .sort(([left], [right]) => left.localeCompare(right));

  return [...orderedEntries, ...extensionEntries];
}

export function listMissingTerminalPlatformThemeTokens(theme: Pick<ThemeManifest, "tokens">): TerminalPlatformTokenName[] {
  return terminalPlatformTokenDefinitions
    .filter((definition) => typeof theme.tokens[definition.name] !== "string")
    .map((definition) => definition.name);
}

export function listTerminalPlatformTokensByCategory(
  category: TerminalPlatformTokenCategory,
): TerminalPlatformTokenDefinition[] {
  return terminalPlatformTokenDefinitions.filter((definition) => definition.category === category);
}
