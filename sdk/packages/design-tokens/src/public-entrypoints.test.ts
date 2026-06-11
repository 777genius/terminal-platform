import { describe, expect, it } from "vitest";

import {
  TERMINAL_PLATFORM_THEME_ATTRIBUTE as cssThemeAttribute,
  createTerminalPlatformThemeCssDeclarations,
  terminalPlatformThemeCssText,
} from "./css.js";
import {
  TERMINAL_PLATFORM_TOKEN_CATEGORIES,
  TERMINAL_PLATFORM_THEME_ATTRIBUTE as themesThemeAttribute,
  listMissingTerminalPlatformThemeTokens,
  terminalPlatformDefaultTheme,
  terminalPlatformTokenDefinitions,
  terminalPlatformThemeManifests,
  type ThemeManifest,
} from "./themes.js";

describe("design tokens public subpath entrypoints", () => {
  it("exposes css assets without requiring hosts to import theme manifests", () => {
    expect(cssThemeAttribute).toBe("data-tp-theme");
    expect(terminalPlatformThemeCssText).toContain("--tp-terminal-color-bg");
    expect(createTerminalPlatformThemeCssDeclarations({
      tokens: {
        "--tp-color-bg": "#000000",
      },
    })).toBe("  --tp-color-bg: #000000;");
  });

  it("exposes theme manifests without requiring hosts to import css text", () => {
    expect(themesThemeAttribute).toBe("data-tp-theme");
    expect(TERMINAL_PLATFORM_TOKEN_CATEGORIES.spacing).toBe("spacing");
    expect(terminalPlatformTokenDefinitions.some((definition) => definition.name === "--tp-space-4")).toBe(true);
    expect(terminalPlatformThemeManifests.map((theme) => theme.id)).toEqual([
      "terminal-platform-default",
      "terminal-platform-light",
    ]);
    expectThemeManifest(terminalPlatformDefaultTheme);
    expect(listMissingTerminalPlatformThemeTokens(terminalPlatformDefaultTheme)).toEqual([]);
  });
});

function expectThemeManifest(theme: ThemeManifest): void {
  expect(theme.tokens["--tp-color-bg"]).toBeTruthy();
}
