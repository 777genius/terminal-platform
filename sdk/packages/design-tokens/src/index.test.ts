import { describe, expect, it } from "vitest";

import {
  TERMINAL_PLATFORM_THEME_ATTRIBUTE,
  terminalPlatformDefaultTheme,
  terminalPlatformDefaultThemeCssText,
  terminalPlatformLightTheme,
  terminalPlatformThemeCssText,
  terminalPlatformThemeManifests,
} from "./index.js";

describe("terminal platform themes", () => {
  it("ships default and light manifests with matching token contracts", () => {
    const defaultTokenKeys = Object.keys(terminalPlatformDefaultTheme.tokens).sort();

    expect(terminalPlatformThemeManifests.map((theme) => theme.id)).toEqual([
      "terminal-platform-default",
      "terminal-platform-light",
    ]);
    expect(Object.keys(terminalPlatformLightTheme.tokens).sort()).toEqual(defaultTokenKeys);
  });

  it("emits attribute-scoped css rules for shadow-dom hosts", () => {
    expect(terminalPlatformDefaultThemeCssText).toBe(terminalPlatformThemeCssText);
    expect(terminalPlatformThemeCssText).toContain(":host, :root");
    expect(terminalPlatformThemeCssText).toContain(
      `:host([${TERMINAL_PLATFORM_THEME_ATTRIBUTE}="terminal-platform-light"])`,
    );
    expect(terminalPlatformThemeCssText).toContain("--tp-color-bg: #f6f8fb;");
    expect(terminalPlatformThemeCssText).toContain("--tp-terminal-color-text: #f4f7fb;");
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
