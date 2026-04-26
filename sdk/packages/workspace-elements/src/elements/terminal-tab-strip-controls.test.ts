import { describe, expect, it } from "vitest";

import type { BackendCapabilitiesInfo } from "@terminal-platform/runtime-types";
import { createInitialWorkspaceSnapshot, type WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import { resolveTerminalTabStripControlState } from "./terminal-tab-strip-controls.js";

describe("terminal tab strip controls", () => {
  it("resolves stable tab presentation state for custom terminal UI surfaces", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot(), { pending: false });

    expect(controls.activeSessionId).toBe("session-1");
    expect(controls.capabilityStatus).toBe("known");
    expect(controls.canCloseTab).toBe(true);
    expect(controls.canCreateTab).toBe(true);
    expect(controls.canFocusTab).toBe(true);
    expect(controls.tabCount).toBe(3);
    expect(controls.tabs).toEqual([
      {
        active: false,
        canClose: true,
        canFocus: true,
        closeTabIndex: -1,
        closeArmed: false,
        closeLabel: "Close tab",
        closeTitle: "Close tab shell",
        index: 0,
        itemKey: "tab-1:0",
        label: "shell",
        metaLabel: "tab-1",
        tabId: "tab-1",
        tabIndex: -1,
        title: "tab-1",
      },
      {
        active: true,
        canClose: true,
        canFocus: true,
        closeTabIndex: 0,
        closeArmed: false,
        closeLabel: "Close tab",
        closeTitle: "Close tab deploy",
        index: 1,
        itemKey: "tab-2:1",
        label: "deploy",
        metaLabel: "tab-2",
        tabId: "tab-2",
        tabIndex: 0,
        title: "tab-2",
      },
      {
        active: false,
        canClose: true,
        canFocus: true,
        closeTabIndex: -1,
        closeArmed: false,
        closeLabel: "Close tab",
        closeTitle: "Close tab terminal...tifier",
        index: 2,
        itemKey: "terminal-tab-with-a-very-long-identifier:2",
        label: "terminal...tifier",
        metaLabel: "terminal...tifier",
        tabId: "terminal-tab-with-a-very-long-identifier",
        tabIndex: -1,
        title: "terminal-tab-with-a-very-long-identifier",
      },
    ]);
  });

  it("disables tab actions while a topology command is pending", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot(), { pending: true });

    expect(controls.canCreateTab).toBe(false);
    expect(controls.canCloseTab).toBe(false);
    expect(controls.canFocusTab).toBe(false);
    expect(controls.tabs.every((tab) => !tab.canFocus)).toBe(true);
    expect(controls.tabs.every((tab) => !tab.canClose)).toBe(true);
  });

  it("exposes armed close state without mutating topology state", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot(), {
      armedCloseTabKey: "tab-2:1",
      pending: false,
    });

    expect(controls.tabs.map((tab) => ({
      closeArmed: tab.closeArmed,
      closeLabel: tab.closeLabel,
      closeTitle: tab.closeTitle,
      itemKey: tab.itemKey,
      tabId: tab.tabId,
    }))).toEqual([
      {
        closeArmed: false,
        closeLabel: "Close tab",
        closeTitle: "Close tab shell",
        itemKey: "tab-1:0",
        tabId: "tab-1",
      },
      {
        closeArmed: true,
        closeLabel: "Confirm close tab",
        closeTitle: "Confirm closing tab deploy",
        itemKey: "tab-2:1",
        tabId: "tab-2",
      },
      {
        closeArmed: false,
        closeLabel: "Close tab",
        closeTitle: "Close tab terminal...tifier",
        itemKey: "terminal-tab-with-a-very-long-identifier:2",
        tabId: "terminal-tab-with-a-very-long-identifier",
      },
    ]);
  });

  it("does not expose tab actions before an attached topology exists", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot({
      attachedSession: null,
    }), { pending: false });

    expect(controls.tabCount).toBe(0);
    expect(controls.tabs).toEqual([]);
    expect(controls.canCloseTab).toBe(false);
    expect(controls.canCreateTab).toBe(false);
    expect(controls.canFocusTab).toBe(false);
  });

  it("disables close affordances when only one tab remains", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot({
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
    }), { pending: false });

    expect(controls.canCloseTab).toBe(false);
    expect(controls.tabs[0]?.canClose).toBe(false);
  });

  it("falls back to compact terminal ids when a backend reports blank titles", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot({
      attachedSession: {
        ...createWorkspaceSnapshot().attachedSession!,
        topology: {
          session_id: "session-1",
          backend_kind: "native",
          focused_tab: "terminal-tab-with-a-very-long-identifier",
          tabs: [
            {
              tab_id: "terminal-tab-with-a-very-long-identifier",
              title: "   ",
              focused_pane: "pane-1",
              root: { kind: "leaf", pane_id: "pane-1" },
            },
          ],
        },
      },
    }), { pending: false });

    expect(controls.tabs[0]?.label).toBe("terminal...tifier");
  });

  it("marks only the first matching tab active when a degraded backend duplicates tab ids", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot({
      attachedSession: {
        ...createWorkspaceSnapshot().attachedSession!,
        topology: {
          session_id: "session-1",
          backend_kind: "native",
          focused_tab: "tab-dup",
          tabs: [
            {
              tab_id: "tab-dup",
              title: "first",
              focused_pane: "pane-1",
              root: { kind: "leaf", pane_id: "pane-1" },
            },
            {
              tab_id: "tab-dup",
              title: "second",
              focused_pane: "pane-2",
              root: { kind: "leaf", pane_id: "pane-2" },
            },
          ],
        },
      },
    }), { pending: false });

    expect(controls.tabs.map((tab) => tab.active)).toEqual([true, false]);
  });

  it("arms only one close affordance when a degraded backend duplicates tab ids", () => {
    const controls = resolveTerminalTabStripControlState(createWorkspaceSnapshot({
      attachedSession: {
        ...createWorkspaceSnapshot().attachedSession!,
        topology: {
          session_id: "session-1",
          backend_kind: "native",
          focused_tab: "tab-dup",
          tabs: [
            {
              tab_id: "tab-dup",
              title: "first",
              focused_pane: "pane-1",
              root: { kind: "leaf", pane_id: "pane-1" },
            },
            {
              tab_id: "tab-dup",
              title: "second",
              focused_pane: "pane-2",
              root: { kind: "leaf", pane_id: "pane-2" },
            },
          ],
        },
      },
    }), {
      armedCloseTabKey: "tab-dup:1",
      pending: false,
    });

    expect(controls.tabs.map((tab) => tab.closeArmed)).toEqual([false, true]);
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
    catalog: {
      ...base.catalog,
      backendCapabilities: {
        native: createCapabilities({}),
      },
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
        focused_tab: "tab-2",
        tabs: [
          {
            tab_id: "tab-1",
            title: "shell",
            focused_pane: "pane-1",
            root: { kind: "leaf", pane_id: "pane-1" },
          },
          {
            tab_id: "tab-2",
            title: "deploy",
            focused_pane: "pane-2",
            root: { kind: "leaf", pane_id: "pane-2" },
          },
          {
            tab_id: "terminal-tab-with-a-very-long-identifier",
            title: null,
            focused_pane: "pane-3",
            root: { kind: "leaf", pane_id: "pane-3" },
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
          title: "deploy",
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
