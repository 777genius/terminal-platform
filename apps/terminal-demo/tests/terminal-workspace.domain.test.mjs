import test from "node:test";
import assert from "node:assert/strict";

import {
  buildBackendDegradedSemantics,
  buildCreateNativeSessionPayload,
  buildHandshakeDegradedSemantics,
  findUnsupportedActionDegradedReason,
  focusedPaneId,
  getHiddenSavedSessionsCount,
  getVisibleSavedSessions,
  parseLaunchArgs,
} from "../dist/features/terminal-workspace/core/domain/index.js";

test("parseLaunchArgs keeps quoted groups intact", () => {
  assert.deepEqual(parseLaunchArgs('-l "-c echo demo" --flag'), ["-l", "-c echo demo", "--flag"]);
});

test("buildCreateNativeSessionPayload trims optional launch fields", () => {
  assert.deepEqual(
    buildCreateNativeSessionPayload({
      title: "  Workspace  ",
      program: " /bin/zsh ",
      args: ' -l "-c pwd" ',
      cwd: " /tmp/demo ",
    }),
    {
      title: "Workspace",
      launch: {
        program: "/bin/zsh",
        args: ["-l", "-c pwd"],
        cwd: "/tmp/demo",
      },
    },
  );
});

test("focusedPaneId prefers focused tab pane before screen fallback", () => {
  const state = {
    session: {
      session_id: "session-1",
      origin: {
        backend: "native",
        authority: "local_daemon",
        foreignReferenceLabel: null,
      },
      title: "Workspace",
      degradedSemantics: [],
    },
    topology: {
      session_id: "session-1",
      backend_kind: "native",
      focused_tab: "tab-2",
      tabs: [
        {
          tab_id: "tab-1",
          title: "One",
          focused_pane: "pane-a",
          root: {
            kind: "leaf",
            pane_id: "pane-a",
          },
        },
        {
          tab_id: "tab-2",
          title: "Two",
          focused_pane: "pane-b",
          root: {
            kind: "leaf",
            pane_id: "pane-b",
          },
        },
      ],
    },
    focusedScreen: {
      pane_id: "pane-screen",
      sequence: "9",
      rows: 24,
      cols: 80,
      source: "native_emulator",
      surface: {
        title: null,
        cursor: null,
        lines: [],
      },
    },
  };

  assert.equal(focusedPaneId(state), "pane-b");
});

test("saved sessions collapse policy keeps recent window and reports hidden count", () => {
  const sessions = Array.from({ length: 12 }, (_, index) => ({
    session_id: `saved-${index + 1}`,
    origin: {
      backend: "native",
      authority: "local_daemon",
      foreignReferenceLabel: null,
    },
    title: `Saved ${index + 1}`,
    saved_at_ms: 1_700_000_000_000 + index,
    manifest: {
      format_version: 1,
      binary_version: "1.0.0",
      protocol_major: 1,
      protocol_minor: 0,
    },
    compatibility: {
      can_restore: true,
      status: "compatible",
    },
    has_launch: true,
    tab_count: 1,
    pane_count: 1,
    restore_semantics: {
      restores_topology: true,
      restores_focus_state: true,
      restores_tab_titles: true,
      uses_saved_launch_spec: true,
      replays_saved_screen_buffers: false,
      preserves_process_state: false,
    },
    degradedSemantics: [],
  }));

  const visible = getVisibleSavedSessions(sessions, false);

  assert.equal(visible.length, 10);
  assert.equal(getHiddenSavedSessionsCount(sessions, visible), 2);
});

test("buildHandshakeDegradedSemantics surfaces daemon degradation explicitly", () => {
  const reasons = buildHandshakeDegradedSemantics({
    handshake: {
      protocol_version: { major: 1, minor: 0 },
      binary_version: "1.0.0",
      daemon_phase: "degraded",
      capabilities: {
        request_reply: true,
        topology_subscriptions: true,
        pane_subscriptions: true,
        backend_discovery: true,
        backend_capability_queries: true,
        saved_sessions: true,
        session_restore: true,
        degraded_error_reasons: true,
      },
      available_backends: ["native"],
      session_scope: "terminal-demo",
    },
    assessment: {
      can_use: true,
      protocol: {
        can_connect: true,
        status: "compatible",
      },
      status: "degraded",
    },
  });

  assert.equal(reasons[0].code, "daemon_degraded");
});

test("buildBackendDegradedSemantics exposes foreign backend limitations", () => {
  const reasons = buildBackendDegradedSemantics({
    backend: "tmux",
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
      layout_dump: false,
      layout_override: false,
      read_only_client_mode: false,
      explicit_session_save: false,
      explicit_session_restore: true,
      plugin_panes: false,
      advisory_metadata_subscriptions: false,
      independent_resize_authority: false,
    },
  });

  assert.deepEqual(reasons.map((reason) => reason.code), [
    "foreign_backend_projection",
    "raw_output_unavailable",
    "scrollback_snapshot_unavailable",
  ]);
});

test("findUnsupportedActionDegradedReason turns unsupported action into explicit degraded semantics", () => {
  const reason = findUnsupportedActionDegradedReason({
    action: "save_session",
    backend: "tmux",
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
      layout_dump: false,
      layout_override: false,
      read_only_client_mode: false,
      explicit_session_save: false,
      explicit_session_restore: true,
      plugin_panes: false,
      advisory_metadata_subscriptions: false,
      independent_resize_authority: false,
    },
  });

  assert.equal(reason.code, "action_save_session_unsupported");
});
