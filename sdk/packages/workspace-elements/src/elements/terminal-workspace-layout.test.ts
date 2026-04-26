import { describe, expect, it } from "vitest";

import {
  TERMINAL_WORKSPACE_INSPECTOR_MODES,
  TERMINAL_WORKSPACE_NAVIGATION_MODES,
  resolveTerminalWorkspaceInspectorState,
  resolveTerminalWorkspaceNavigationState,
} from "./terminal-workspace-layout.js";

describe("terminal workspace layout", () => {
  it("keeps the inline inspector as the default public layout", () => {
    expect(resolveTerminalWorkspaceInspectorState(undefined)).toEqual({
      mode: TERMINAL_WORKSPACE_INSPECTOR_MODES.inline,
      renderCollapsedInspector: false,
      renderInlineInspector: true,
      renderInspector: true,
      summaryLabel: "Layout and tools",
    });
  });

  it("supports a collapsed inspector for terminal-first workspaces", () => {
    expect(resolveTerminalWorkspaceInspectorState("collapsed")).toEqual({
      mode: TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed,
      renderCollapsedInspector: true,
      renderInlineInspector: false,
      renderInspector: true,
      summaryLabel: "Layout and tools",
    });
  });

  it("supports hiding the inspector when a host owns topology controls elsewhere", () => {
    expect(resolveTerminalWorkspaceInspectorState("hidden")).toEqual({
      mode: TERMINAL_WORKSPACE_INSPECTOR_MODES.hidden,
      renderCollapsedInspector: false,
      renderInlineInspector: false,
      renderInspector: false,
      summaryLabel: "Layout and tools",
    });
  });

  it("falls back to inline mode for unknown host input", () => {
    expect(resolveTerminalWorkspaceInspectorState("unknown")).toMatchObject({
      mode: TERMINAL_WORKSPACE_INSPECTOR_MODES.inline,
      renderInlineInspector: true,
      renderInspector: true,
    });
  });

  it("keeps inline navigation as the default public layout", () => {
    expect(resolveTerminalWorkspaceNavigationState(undefined)).toEqual({
      mode: TERMINAL_WORKSPACE_NAVIGATION_MODES.inline,
      renderCollapsedNavigation: false,
      renderInlineNavigation: true,
      renderNavigation: true,
      summaryLabel: "Sessions and saved layouts",
    });
  });

  it("supports collapsed navigation for terminal-first workspaces", () => {
    expect(resolveTerminalWorkspaceNavigationState("collapsed")).toEqual({
      mode: TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed,
      renderCollapsedNavigation: true,
      renderInlineNavigation: false,
      renderNavigation: true,
      summaryLabel: "Sessions and saved layouts",
    });
  });

  it("supports hiding navigation when a host owns session selection elsewhere", () => {
    expect(resolveTerminalWorkspaceNavigationState("hidden")).toEqual({
      mode: TERMINAL_WORKSPACE_NAVIGATION_MODES.hidden,
      renderCollapsedNavigation: false,
      renderInlineNavigation: false,
      renderNavigation: false,
      summaryLabel: "Sessions and saved layouts",
    });
  });

  it("falls back to inline navigation for unknown host input", () => {
    expect(resolveTerminalWorkspaceNavigationState("unknown")).toMatchObject({
      mode: TERMINAL_WORKSPACE_NAVIGATION_MODES.inline,
      renderInlineNavigation: true,
      renderNavigation: true,
    });
  });
});
