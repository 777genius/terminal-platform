import { describe, expect, it } from "vitest";

import type { BackendCapabilitiesInfo } from "@terminal-platform/runtime-types";
import { createInitialWorkspaceSnapshot, type WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import { resolveTerminalCommandDockControlState } from "./terminal-command-dock-controls.js";

describe("terminal command dock controls", () => {
  it("resolves command lane controls from focused workspace state", () => {
    const snapshot = createWorkspaceSnapshot({
      drafts: {
        "pane-1": "git status",
      },
      commandHistory: {
        entries: ["pwd", "ls -la", "git status", "npm test", "cargo test", "git diff"],
        limit: 50,
      },
    });

    const controls = resolveTerminalCommandDockControlState(snapshot, { pending: false });

    expect(controls.activeSessionId).toBe("session-1");
    expect(controls.activePaneId).toBe("pane-1");
    expect(controls.draft).toBe("git status");
    expect(controls.canSend).toBe(true);
    expect(controls.canUsePane).toBe(true);
    expect(controls.canWriteInput).toBe(true);
    expect(controls.inputCapabilityStatus).toBe("known");
    expect(controls.canPasteClipboard).toBe(true);
    expect(controls.pasteCapabilityStatus).toBe("known");
    expect(controls.canSaveLayout).toBe(true);
    expect(controls.saveCapabilityStatus).toBe("known");
    expect(controls.recentCommands).toEqual([
      "git diff",
      "cargo test",
      "npm test",
      "git status",
      "ls -la",
    ]);
  });

  it("disables command input when loaded backend capabilities reject pane input writes", () => {
    const snapshot = createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {
          native: createCapabilities({ pane_input_write: false }),
        },
      },
      drafts: {
        "pane-1": "git status",
      },
    });

    const controls = resolveTerminalCommandDockControlState(snapshot, { pending: false });

    expect(controls.canUsePane).toBe(true);
    expect(controls.canWriteInput).toBe(false);
    expect(controls.canSend).toBe(false);
    expect(controls.inputCapabilityStatus).toBe("known");
  });

  it("disables paste when loaded backend capabilities reject pane paste writes", () => {
    const snapshot = createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {
          native: createCapabilities({ pane_paste_write: false }),
        },
      },
    });

    const controls = resolveTerminalCommandDockControlState(snapshot, { pending: false });

    expect(controls.canUsePane).toBe(true);
    expect(controls.canPasteClipboard).toBe(false);
    expect(controls.pasteCapabilityStatus).toBe("known");
  });

  it("disables save layout when loaded backend capabilities reject explicit session saves", () => {
    const snapshot = createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {
          native: createCapabilities({ explicit_session_save: false }),
        },
      },
    });

    const controls = resolveTerminalCommandDockControlState(snapshot, { pending: false });

    expect(controls.activeSessionId).toBe("session-1");
    expect(controls.canSaveLayout).toBe(false);
    expect(controls.saveCapabilityStatus).toBe("known");
  });

  it("keeps command input enabled while capabilities are pending and a focused pane exists", () => {
    const snapshot = createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {},
      },
      drafts: {
        "pane-1": "git status",
      },
    });

    const controls = resolveTerminalCommandDockControlState(snapshot, { pending: false });

    expect(controls.canUsePane).toBe(true);
    expect(controls.canWriteInput).toBe(true);
    expect(controls.canSend).toBe(true);
    expect(controls.inputCapabilityStatus).toBe("unknown");
  });

  it("keeps paste enabled while capabilities are pending and a focused pane exists", () => {
    const snapshot = createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {},
      },
    });

    const controls = resolveTerminalCommandDockControlState(snapshot, { pending: false });

    expect(controls.canUsePane).toBe(true);
    expect(controls.canPasteClipboard).toBe(true);
    expect(controls.pasteCapabilityStatus).toBe("unknown");
  });

  it("keeps save layout disabled while capabilities are pending", () => {
    const snapshot = createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {},
      },
    });

    const controls = resolveTerminalCommandDockControlState(snapshot, { pending: false });

    expect(controls.activeSessionId).toBe("session-1");
    expect(controls.canSaveLayout).toBe(false);
    expect(controls.saveCapabilityStatus).toBe("unknown");
  });

  it("disables all pane actions while pending", () => {
    const controls = resolveTerminalCommandDockControlState(createWorkspaceSnapshot(), { pending: true });

    expect(controls.canSend).toBe(false);
    expect(controls.canUsePane).toBe(false);
    expect(controls.canWriteInput).toBe(false);
    expect(controls.canPasteClipboard).toBe(false);
    expect(controls.canSaveLayout).toBe(false);
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
      focused_screen: {
        pane_id: "pane-1",
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
