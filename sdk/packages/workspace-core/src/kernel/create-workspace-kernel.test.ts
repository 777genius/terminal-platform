import { describe, expect, it } from "vitest";

import type { WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";

import { createWorkspaceKernel } from "./create-workspace-kernel.js";
import { DEFAULT_WORKSPACE_THEME_ID } from "../read-models/workspace-snapshot.js";

describe("createWorkspaceKernel theme commands", () => {
  it("applies registered terminal platform themes", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      now: () => 1000,
    });

    kernel.commands.setTheme(" terminal-platform-light ");

    expect(kernel.selectors.themeId()).toBe("terminal-platform-light");
    expect(kernel.diagnostics.list()).toEqual([]);

    await kernel.dispose();
  });

  it("rejects unknown themes without corrupting the snapshot", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      now: () => 2000,
    });

    kernel.commands.setTheme("missing-theme");

    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "theme_unsupported",
        message: "Theme \"missing-theme\" is not registered for this workspace",
        recoverable: true,
        severity: "warn",
        timestampMs: 2000,
      },
    ]);

    await kernel.dispose();
  });

  it("allows hosts to register custom theme ids", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      availableThemeIds: ["acme-terminal"],
    });

    kernel.commands.setTheme("acme-terminal");

    expect(kernel.selectors.themeId()).toBe("acme-terminal");

    await kernel.dispose();
  });

  it("starts with a registered initial theme", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialThemeId: " terminal-platform-light ",
    });

    expect(kernel.selectors.themeId()).toBe("terminal-platform-light");
    expect(kernel.diagnostics.list()).toEqual([]);

    await kernel.dispose();
  });

  it("falls back when an initial theme is not registered", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialThemeId: "stale-theme",
      now: () => 3000,
    });

    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "theme_unsupported",
        message: "Initial theme \"stale-theme\" is not registered for this workspace",
        recoverable: true,
        severity: "warn",
        timestampMs: 3000,
      },
    ]);

    await kernel.dispose();
  });

  it("keeps the default theme available when hosts register custom themes", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      availableThemeIds: ["acme-terminal"],
      initialThemeId: DEFAULT_WORKSPACE_THEME_ID,
    });

    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);

    kernel.commands.setTheme("acme-terminal");
    expect(kernel.selectors.themeId()).toBe("acme-terminal");

    kernel.commands.setTheme(DEFAULT_WORKSPACE_THEME_ID);
    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);

    await kernel.dispose();
  });
});

describe("createWorkspaceKernel terminal display preferences", () => {
  it("starts with explicit terminal display preferences", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialTerminalFontScale: " large ",
      initialTerminalLineWrap: false,
    });

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "large",
      lineWrap: false,
    });
    expect(kernel.diagnostics.list()).toEqual([]);

    await kernel.dispose();
  });

  it("updates terminal display preferences without touching theme state", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
    });

    kernel.commands.setTerminalFontScale("compact");
    kernel.commands.setTerminalLineWrap(false);

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "compact",
      lineWrap: false,
    });
    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);

    await kernel.dispose();
  });

  it("rejects unknown terminal font scales without corrupting preferences", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      now: () => 4000,
    });

    kernel.commands.setTerminalFontScale("poster");

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "default",
      lineWrap: true,
    });
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "terminal_display_preference_unsupported",
        message: 'Terminal font scale "poster" is not supported',
        recoverable: true,
        severity: "warn",
        timestampMs: 4000,
      },
    ]);

    await kernel.dispose();
  });

  it("falls back when an initial terminal font scale is unsupported", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialTerminalFontScale: "poster",
      now: () => 5000,
    });

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "default",
      lineWrap: true,
    });
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "terminal_display_preference_unsupported",
        message: 'Initial terminal font scale "poster" is not supported',
        recoverable: true,
        severity: "warn",
        timestampMs: 5000,
      },
    ]);

    await kernel.dispose();
  });
});

function createUnusedTransport(): WorkspaceTransportClient {
  return {
    close: async () => {},
  } as unknown as WorkspaceTransportClient;
}
