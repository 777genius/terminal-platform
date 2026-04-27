import { describe, expect, it } from "vitest";

import type { BackendCapabilitiesInfo } from "@terminal-platform/runtime-types";
import { createInitialWorkspaceSnapshot, type WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import { resolveTerminalScreenControlState } from "./terminal-screen-controls.js";

describe("terminal screen controls", () => {
  it("enables copy and direct input for a focused pane with loaded write capability", () => {
    const controls = resolveTerminalScreenControlState(createWorkspaceSnapshot());

    expect(controls.activeSessionId).toBe("session-1");
    expect(controls.activePaneId).toBe("pane-1");
    expect(controls.canCopyVisibleOutput).toBe(true);
    expect(controls.canUseDirectInput).toBe(true);
    expect(controls.canUseDirectPaste).toBe(true);
    expect(controls.inputCapabilityStatus).toBe("known");
    expect(controls.pasteCapabilityStatus).toBe("known");
  });

  it("disables direct input when loaded backend capabilities reject pane input writes", () => {
    const controls = resolveTerminalScreenControlState(createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {
          native: createCapabilities({ pane_input_write: false }),
        },
      },
    }));

    expect(controls.canCopyVisibleOutput).toBe(true);
    expect(controls.canUseDirectInput).toBe(false);
    expect(controls.inputCapabilityStatus).toBe("known");
  });

  it("disables direct paste when loaded backend capabilities reject pane paste writes", () => {
    const controls = resolveTerminalScreenControlState(createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {
          native: createCapabilities({ pane_paste_write: false }),
        },
      },
    }));

    expect(controls.canUseDirectInput).toBe(true);
    expect(controls.canUseDirectPaste).toBe(false);
    expect(controls.pasteCapabilityStatus).toBe("known");
  });

  it("keeps direct input enabled while capabilities are pending and a focused pane exists", () => {
    const controls = resolveTerminalScreenControlState(createWorkspaceSnapshot({
      catalog: {
        ...createInitialWorkspaceSnapshot().catalog,
        backendCapabilities: {},
      },
    }));

    expect(controls.canUseDirectInput).toBe(true);
    expect(controls.canUseDirectPaste).toBe(true);
    expect(controls.inputCapabilityStatus).toBe("unknown");
    expect(controls.pasteCapabilityStatus).toBe("unknown");
  });

  it("disables screen actions when no screen is attached", () => {
    const controls = resolveTerminalScreenControlState(createWorkspaceSnapshot({
      attachedSession: null,
    }));

    expect(controls.screen).toBeNull();
    expect(controls.canCopyVisibleOutput).toBe(false);
    expect(controls.canUseDirectInput).toBe(false);
    expect(controls.canUseDirectPaste).toBe(false);
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
