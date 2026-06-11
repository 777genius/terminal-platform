import { describe, expect, it } from "vitest";

import {
  TERMINAL_PLATFORM_TOKEN_CATEGORIES,
  TERMINAL_PLATFORM_THEME_ATTRIBUTE,
  createTerminalPlatformThemeCssDeclarations,
  createTerminalPlatformThemeCssRule,
  createTerminalPlatformThemeCssText,
  listMissingTerminalPlatformThemeTokens,
  listTerminalPlatformThemeTokenEntries,
  listTerminalPlatformTokensByCategory,
  terminalPlatformDefaultTheme,
  terminalPlatformDefaultThemeCssText,
  terminalPlatformLightTheme,
  terminalPlatformTokenDefinitions,
  terminalPlatformThemeCssText,
  terminalPlatformThemeManifests,
} from "./index.js";

describe("terminal platform themes", () => {
  it("ships default and light manifests with matching token contracts", () => {
    const defaultTokenKeys = Object.keys(terminalPlatformDefaultTheme.tokens).sort();

    expect(terminalPlatformTokenDefinitions).toHaveLength(defaultTokenKeys.length);
    expect(terminalPlatformThemeManifests.map((theme) => theme.id)).toEqual([
      "terminal-platform-default",
      "terminal-platform-light",
    ]);
    expect(Object.keys(terminalPlatformLightTheme.tokens).sort()).toEqual(defaultTokenKeys);
    expect(listMissingTerminalPlatformThemeTokens(terminalPlatformDefaultTheme)).toEqual([]);
    expect(listMissingTerminalPlatformThemeTokens(terminalPlatformLightTheme)).toEqual([]);
    expect(listTerminalPlatformTokensByCategory(TERMINAL_PLATFORM_TOKEN_CATEGORIES.terminalColor)
      .map((definition) => definition.name)).toEqual([
      "--tp-terminal-color-bg",
      "--tp-terminal-color-bg-raised",
      "--tp-terminal-color-border",
      "--tp-terminal-color-text",
      "--tp-terminal-color-text-muted",
      "--tp-terminal-color-accent",
    ]);
  });

  it("emits attribute-scoped css rules for shadow-dom hosts", () => {
    expect(terminalPlatformDefaultThemeCssText).toBe(terminalPlatformThemeCssText);
    expect(createTerminalPlatformThemeCssText(terminalPlatformThemeManifests)).toBe(terminalPlatformThemeCssText);
    expect(terminalPlatformThemeCssText).toContain(":host, :root");
    expect(terminalPlatformThemeCssText).toContain(
      `:host([${TERMINAL_PLATFORM_THEME_ATTRIBUTE}="terminal-platform-light"])`,
    );
    expect(terminalPlatformThemeCssText).toContain("--tp-color-bg: #f6f8fb;");
    expect(terminalPlatformThemeCssText).toContain("--tp-terminal-color-text: #f4f7fb;");
    expect(createTerminalPlatformThemeCssRule(".host", terminalPlatformLightTheme)).toContain(
      ".host {\n  --tp-color-bg: #f6f8fb;",
    );
    expect(createTerminalPlatformThemeCssDeclarations(terminalPlatformDefaultTheme, { indent: "" }))
      .toContain("--tp-shadow-panel: 0 18px 60px rgba(0, 0, 0, 0.28);");
  });

  it("orders known tokens by taxonomy and preserves host extension tokens", () => {
    const entries = listTerminalPlatformThemeTokenEntries({
      tokens: {
        "--tp-z-extension": "2",
        "--tp-color-bg": "#000000",
        "--tp-a-extension": "1",
      },
    });

    expect(entries).toEqual([
      ["--tp-color-bg", "#000000"],
      ["--tp-a-extension", "1"],
      ["--tp-z-extension", "2"],
    ]);
  });

  it("keeps embedded terminal surfaces legible across themes", () => {
    for (const theme of terminalPlatformThemeManifests) {
      const terminalBackground = theme.tokens["--tp-terminal-color-bg"];
      const terminalText = theme.tokens["--tp-terminal-color-text"];
      const terminalMutedText = theme.tokens["--tp-terminal-color-text-muted"];

      expect(contrastRatio(terminalBackground, terminalText)).toBeGreaterThanOrEqual(7);
      expect(contrastRatio(terminalBackground, terminalMutedText)).toBeGreaterThanOrEqual(4.5);
    }

    expect(terminalPlatformLightTheme.tokens["--tp-terminal-color-text"]).not.toBe(
      terminalPlatformLightTheme.tokens["--tp-color-text"],
    );
  });
});

function contrastRatio(backgroundHex: string, foregroundHex: string): number {
  const background = relativeLuminance(hexToRgb(backgroundHex));
  const foreground = relativeLuminance(hexToRgb(foregroundHex));
  const lighter = Math.max(background, foreground);
  const darker = Math.min(background, foreground);
  return (lighter + 0.05) / (darker + 0.05);
}

function relativeLuminance([red, green, blue]: readonly [number, number, number]): number {
  const [linearRed, linearGreen, linearBlue] = [red, green, blue].map((channel) => {
    const normalized = channel / 255;
    return normalized <= 0.03928
      ? normalized / 12.92
      : ((normalized + 0.055) / 1.055) ** 2.4;
  });

  return 0.2126 * linearRed + 0.7152 * linearGreen + 0.0722 * linearBlue;
}

function hexToRgb(hex: string): readonly [number, number, number] {
  const match = /^#(?<red>[0-9a-f]{2})(?<green>[0-9a-f]{2})(?<blue>[0-9a-f]{2})$/iu.exec(hex);
  if (!match?.groups) {
    throw new Error(`Expected a six-digit hex color, received ${hex}`);
  }

  return [
    Number.parseInt(match.groups.red, 16),
    Number.parseInt(match.groups.green, 16),
    Number.parseInt(match.groups.blue, 16),
  ];
}
