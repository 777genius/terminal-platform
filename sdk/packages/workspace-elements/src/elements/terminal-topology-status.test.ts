import { describe, expect, it } from "vitest";

import type { TerminalTopologyControlState } from "./terminal-topology-controls.js";
import { resolveTerminalTopologyStatus } from "./terminal-topology-status.js";

describe("terminal topology status", () => {
  it("reports ready when the focused session accepts layout mutations", () => {
    expect(resolveTerminalTopologyStatus(createControls())).toMatchObject({
      label: "Topology ready",
      tone: "ready",
      capabilityLabel: "Capabilities known",
      canMutateLayout: true,
    });
  });

  it("keeps pending capabilities explicit", () => {
    expect(resolveTerminalTopologyStatus(createControls({
      capabilityStatus: "unknown",
    }))).toMatchObject({
      label: "Topology pending",
      tone: "pending",
      capabilityLabel: "Capabilities pending",
      canMutateLayout: true,
    });
  });

  it("reports read only when no layout mutation is available", () => {
    expect(resolveTerminalTopologyStatus(createControls({
      canCreateTab: false,
      canClosePane: false,
      canCloseTab: false,
      canRenameTab: false,
      canResizePane: false,
      canSplitPane: false,
    }))).toMatchObject({
      label: "Read only layout",
      tone: "limited",
      canMutateLayout: false,
    });
  });

  it("reports missing topology separately from unsupported mutations", () => {
    expect(resolveTerminalTopologyStatus(createControls({
      activeTab: null,
      activePaneId: null,
      activePaneSize: null,
      canCreateTab: false,
      canClosePane: false,
      canCloseTab: false,
      canFocusPane: false,
      canFocusTab: false,
      canRenameTab: false,
      canResizePane: false,
      canSplitPane: false,
      paneCount: 0,
      tabCount: 0,
    }))).toMatchObject({
      label: "No topology",
      tone: "idle",
      canMutateLayout: false,
    });
  });

  it("reports missing session before topology state", () => {
    expect(resolveTerminalTopologyStatus(createControls({
      activeSessionId: null,
      activeTab: null,
      activePaneId: null,
    }))).toMatchObject({
      label: "Pick a session",
      tone: "idle",
      capabilityLabel: "No backend",
    });
  });
});

function createControls(
  overrides: Partial<TerminalTopologyControlState> = {},
): TerminalTopologyControlState {
  return {
    activeSessionId: "session-1",
    activeTab: {
      tab_id: "tab-1",
      title: "shell",
      focused_pane: "pane-1",
      root: { kind: "leaf", pane_id: "pane-1" },
    },
    activePaneId: "pane-1",
    activePaneSize: {
      rows: 24,
      cols: 80,
    },
    capabilityStatus: "known",
    canCreateTab: true,
    canClosePane: false,
    canCloseTab: false,
    canFocusPane: true,
    canFocusTab: true,
    canRenameTab: true,
    canResizePane: true,
    canSplitPane: true,
    paneCount: 1,
    tabCount: 1,
    ...overrides,
  };
}
