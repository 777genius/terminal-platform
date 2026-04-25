import { describe, expect, it } from "vitest";

import { terminalPlatformDefaultTheme, terminalPlatformLightTheme } from "@terminal-platform/design-tokens";

import {
  resolveTerminalToolbarFontScaleOption,
  resolveTerminalToolbarLineWrapOption,
  resolveTerminalToolbarThemeOption,
} from "./terminal-toolbar-preferences.js";

describe("terminal toolbar preferences", () => {
  it("normalizes theme labels and explains active state", () => {
    expect(resolveTerminalToolbarThemeOption(terminalPlatformDefaultTheme, "terminal-platform-default")).toEqual({
      isActive: true,
      label: "Default",
      title: "Default theme is active.",
    });

    expect(resolveTerminalToolbarThemeOption(terminalPlatformLightTheme, "terminal-platform-default")).toEqual({
      isActive: false,
      label: "Light",
      title: "Switch workspace theme to Light.",
    });
  });

  it("keeps custom theme names intact for community integrations", () => {
    expect(
      resolveTerminalToolbarThemeOption(
        {
          id: "community-night",
          displayName: "Community Night",
        },
        "terminal-platform-default",
      ),
    ).toEqual({
      isActive: false,
      label: "Community Night",
      title: "Switch workspace theme to Community Night.",
    });
  });

  it("builds stable font scale labels and titles", () => {
    expect(resolveTerminalToolbarFontScaleOption("compact", "default")).toEqual({
      isActive: false,
      label: "Compact",
      title: "Set terminal font size to compact.",
    });

    expect(resolveTerminalToolbarFontScaleOption("large", "large")).toEqual({
      isActive: true,
      label: "Large",
      title: "Large terminal font size is active.",
    });
  });

  it("describes the current and next line-wrap state", () => {
    expect(resolveTerminalToolbarLineWrapOption(true)).toEqual({
      isActive: true,
      label: "Wrap on",
      nextValue: false,
      title: "Disable terminal line wrapping.",
    });

    expect(resolveTerminalToolbarLineWrapOption(false)).toEqual({
      isActive: false,
      label: "Wrap off",
      nextValue: true,
      title: "Enable terminal line wrapping.",
    });
  });
});
