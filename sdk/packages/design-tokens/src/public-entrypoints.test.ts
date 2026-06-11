import { describe, expect, it } from "vitest";

import {
  TERMINAL_PLATFORM_THEME_ATTRIBUTE as cssThemeAttribute,
  terminalPlatformThemeCssText,
} from "./css.js";
import {
  TERMINAL_PLATFORM_THEME_ATTRIBUTE as themesThemeAttribute,
  terminalPlatformDefaultTheme,
  terminalPlatformThemeManifests,
  type ThemeManifest,
} from "./themes.js";

describe("design tokens public subpath entrypoints", () => {
  it("exposes css assets without requiring hosts to import theme manifests", () => {
    expect(cssThemeAttribute).toBe("data-tp-theme");
    expect(terminalPlatformThemeCssText).toContain("--tp-terminal-color-bg");
  });

  it("exposes theme manifests without requiring hosts to import css text", () => {
    expect(themesThemeAttribute).toBe("data-tp-theme");
    expect(terminalPlatformThemeManifests.map((theme) => theme.id)).toEqual([
      "terminal-platform-default",
      "terminal-platform-light",
    ]);
    expectThemeManifest(terminalPlatformDefaultTheme);
  });
});

function expectThemeManifest(theme: ThemeManifest): void {
  expect(theme.tokens["--tp-color-bg"]).toBeTruthy();
}
