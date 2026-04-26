import { describe, expect, it } from "vitest";

import {
  TERMINAL_WORKSPACE_INSPECTOR_MODES,
  resolveTerminalWorkspaceInspectorState,
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
});
