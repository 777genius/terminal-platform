import { describe, expect, it } from "vitest";

import type { BackendCapabilitiesInfo } from "@terminal-platform/runtime-types";
import { createInitialWorkspaceSnapshot, type WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  compactTerminalId,
  countPaneTreeLeaves,
  resolveTerminalTopologyControlState,
} from "./terminal-topology-controls.js";

describe("terminal topology controls", () => {
  it("resolves focused tab and pane actions while capabilities are still pending", () => {
    const snapshot = createWorkspaceSnapshot();

    const controls = resolveTerminalTopologyControlState(snapshot);

    expect(controls.capabilityStatus).toBe("unknown");
    expect(controls.activeSessionId).toBe("session-1");
    expect(controls.activePaneId).toBe("pane-2");
    expect(controls.tabCount).toBe(1);
    expect(controls.paneCount).toBe(2);
    expect(controls.canCreateTab).toBe(true);
    expect(controls.canClosePane).toBe(true);
    expect(controls.canCloseTab).toBe(false);
    expect(controls.canSplitPane).toBe(true);
    expect(controls.canFocusPane).toBe(true);
    expect(controls.canFocusTab).toBe(true);
    expect(controls.canRenameTab).toBe(true);
  });

  it("uses loaded backend capabilities to disable unsupported topology actions", () => {
    const snapshot = createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {
          native: createCapabilities({
            pane_close: false,
            pane_split: false,
            tab_close: false,
            tab_create: false,
            tab_rename: false,
          }),
        },
      },
    });

    const controls = resolveTerminalTopologyControlState(snapshot);

    expect(controls.capabilityStatus).toBe("known");
    expect(controls.canCreateTab).toBe(false);
    expect(controls.canClosePane).toBe(false);
    expect(controls.canCloseTab).toBe(false);
    expect(controls.canSplitPane).toBe(false);
    expect(controls.canFocusPane).toBe(true);
    expect(controls.canFocusTab).toBe(true);
    expect(controls.canRenameTab).toBe(false);
  });

  it("prevents destructive close controls from removing the last pane or tab", () => {
    const snapshot = createWorkspaceSnapshot({
      attachedSession: {
        ...createWorkspaceSnapshot().attachedSession!,
        topology: {
          session_id: "session-1",
          backend_kind: "native",
          focused_tab: "tab-1",
          tabs: [
            {
              tab_id: "tab-1",
              title: "shell",
              focused_pane: "pane-1",
              root: { kind: "leaf", pane_id: "pane-1" },
            },
          ],
        },
      },
    });

    const controls = resolveTerminalTopologyControlState(snapshot);

    expect(controls.paneCount).toBe(1);
    expect(controls.tabCount).toBe(1);
    expect(controls.canClosePane).toBe(false);
    expect(controls.canCloseTab).toBe(false);
    expect(controls.canRenameTab).toBe(true);
  });

  it("keeps compact ids stable and counts nested pane trees", () => {
    expect(compactTerminalId("short-pane")).toBe("short-pane");
    expect(compactTerminalId("memory-session-1-pane-123456")).toBe("memory-s...123456");
    expect(countPaneTreeLeaves({
      kind: "split",
      direction: "horizontal",
      first: { kind: "leaf", pane_id: "pane-1" },
      second: {
        kind: "split",
        direction: "vertical",
        first: { kind: "leaf", pane_id: "pane-2" },
        second: { kind: "leaf", pane_id: "pane-3" },
      },
    })).toBe(3);
  });
});

function createWorkspaceSnapshot(overrides: Partial<WorkspaceSnapshot> = {}): WorkspaceSnapshot {
  const base = createInitialWorkspaceSnapshot();
  return {
    ...base,
    connection: {
      state: "ready",
      handshake: null,
      lastError: null,
    },
    selection: {
      activeSessionId: "session-1",
      activePaneId: null,
    },
    attachedSession: {
      session: {
        session_id: "session-1",
        route: {
          backend: "native",
          authority: "local_daemon",
          external: null,
        },
        title: "SDK shell",
      },
      health: {
        session_id: "session-1",
        phase: "ready",
        can_attach: true,
        invalidated: false,
        reason: null,
        detail: null,
      },
      topology: {
        session_id: "session-1",
        backend_kind: "native",
        focused_tab: "tab-1",
        tabs: [
          {
            tab_id: "tab-1",
            title: "shell",
            focused_pane: "pane-2",
            root: {
              kind: "split",
              direction: "horizontal",
              first: { kind: "leaf", pane_id: "pane-1" },
              second: { kind: "leaf", pane_id: "pane-2" },
            },
          },
        ],
      },
      focused_screen: {
        pane_id: "pane-2",
        sequence: 1n,
        rows: 24,
        cols: 80,
        source: "native_emulator",
        surface: {
          title: "shell",
          cursor: null,
          lines: [{ text: "ready" }],
        },
      },
    },
    ...overrides,
  };
}

function createCapabilities(
  overrides: Partial<BackendCapabilitiesInfo["capabilities"]>,
): BackendCapabilitiesInfo {
  return {
    backend: "native",
    capabilities: {
      tiled_panes: true,
      floating_panes: false,
      split_resize: true,
      tab_create: true,
      tab_close: true,
      tab_focus: true,
      tab_rename: true,
      session_scoped_tab_refs: true,
      session_scoped_pane_refs: true,
      pane_split: true,
      pane_close: true,
      pane_focus: true,
      pane_input_write: true,
      pane_paste_write: true,
      raw_output_stream: false,
      rendered_viewport_stream: true,
      rendered_viewport_snapshot: true,
      rendered_scrollback_snapshot: false,
      layout_dump: true,
      layout_override: true,
      read_only_client_mode: false,
      explicit_session_save: true,
      explicit_session_restore: true,
      plugin_panes: false,
      advisory_metadata_subscriptions: true,
      independent_resize_authority: true,
      ...overrides,
    },
  };
}
