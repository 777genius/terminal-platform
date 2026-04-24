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
  });
});
