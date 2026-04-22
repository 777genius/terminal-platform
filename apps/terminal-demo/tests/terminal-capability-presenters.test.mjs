import test from "node:test";
import assert from "node:assert/strict";

import { initialTerminalRuntimeWorkspaceState } from "../dist/features/terminal-workspace-kernel/contracts/index.js";
import { createTerminalWorkspaceCatalogModel } from "../dist/features/terminal-workspace-catalog/renderer/presenters/createTerminalWorkspaceCatalogModel.js";
import { createTerminalSavedSessionsModel } from "../dist/features/terminal-saved-sessions/renderer/presenters/createTerminalSavedSessionsModel.js";
import { createTerminalActiveSessionModel } from "../dist/features/terminal-active-session/renderer/presenters/createTerminalActiveSessionModel.js";
import { createTerminalInputComposerModel } from "../dist/features/terminal-input-composer/renderer/presenters/createTerminalInputComposerModel.js";

test("capability presenters project runtime state into focused read models", () => {
  const runtime = {
    transport: {
      controlPlaneUrl: "ws://127.0.0.1:4100/terminal-gateway/control?token=abc",
      sessionStreamUrl: "ws://127.0.0.1:4100/terminal-gateway/stream?token=abc",
      runtimeSlug: "terminal-demo",
    },
    state: {
      ...structuredClone(initialTerminalRuntimeWorkspaceState),
      status: "ready",
      sessionStatus: "ready",
      sessionStreamHealth: {
        phase: "reconnecting",
        reconnectAttempts: 2,
        lastError: "Terminal session stream connection closed",
      },
      handshake: {
        handshake: {
          protocol_version: { major: 1, minor: 0 },
          binary_version: "1.0.0",
          daemon_phase: "ready",
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
          available_backends: ["tmux"],
          session_scope: "terminal-demo",
        },
        assessment: {
          can_use: true,
          protocol: {
            can_connect: true,
            status: "compatible",
          },
          status: "ready",
        },
        degradedSemantics: [],
      },
      sessions: [
        {
          session_id: "session-1",
          origin: {
            backend: "tmux",
            authority: "imported_foreign",
            foreignReferenceLabel: "tmux",
          },
          title: "Foreign Session",
          degradedSemantics: [
            {
              code: "foreign_session_semantics",
              scope: "session",
              severity: "warning",
              summary: "Session uses tmux semantics",
              detail: "Canonical IDs are stable, but behavior may differ from native.",
            },
          ],
        },
      ],
      discoveredSessions: {
        tmux: [
          {
            importHandle: "import-1",
            backend: "tmux",
            title: null,
            sourceLabel: "tmux",
            degradedSemantics: [],
          },
        ],
      },
      capabilities: {
        tmux: {
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
          degradedSemantics: [
            {
              code: "foreign_backend_projection",
              scope: "backend",
              severity: "warning",
              summary: "Foreign backend - tmux",
              detail: "Projection is conservative.",
            },
          ],
        },
      },
      activeSessionId: "session-1",
      activeSessionState: {
        session: {
          session_id: "session-1",
          origin: {
            backend: "tmux",
            authority: "imported_foreign",
            foreignReferenceLabel: "tmux",
          },
          title: "Foreign Session",
          degradedSemantics: [],
        },
        topology: {
          session_id: "session-1",
          backend_kind: "tmux",
          focused_tab: "tab-1",
          tabs: [
            {
              tab_id: "tab-1",
              title: null,
              focused_pane: "pane-1",
              root: {
                kind: "leaf",
                pane_id: "pane-1",
              },
            },
          ],
        },
        focusedScreen: {
          pane_id: "pane-1",
          sequence: "7",
          rows: 24,
          cols: 80,
          source: "tmux_capture_pane",
          surface: {
            title: null,
            cursor: null,
            lines: [
              { text: "demo" },
            ],
          },
        },
      },
      savedSessions: Array.from({ length: 12 }, (_, index) => ({
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
      })),
    },
    commands: {},
  };

  const catalogModel = createTerminalWorkspaceCatalogModel({
    runtime,
    form: {
      title: "Workspace",
      program: "",
      args: "",
      cwd: "",
    },
  });
  const savedModel = createTerminalSavedSessionsModel({
    runtime,
    showAll: false,
  });
  const activeModel = createTerminalActiveSessionModel(runtime);
  const inputModel = createTerminalInputComposerModel({
    runtime,
    draft: "pwd",
  });

  assert.equal(catalogModel.statusBadge.label, "status ready");
  assert.equal(catalogModel.sessionStreamBadge.label, "stream reconnecting");
  assert.equal(catalogModel.gatewayInfo[0].value, "terminal-demo");
  assert.equal(catalogModel.discoveredGroups[0].sessions[0].title, "Untitled foreign session");
  assert.equal(savedModel.savedSessionItems.length, 10);
  assert.equal(savedModel.hiddenSavedSessionsCount, 2);
  assert.equal(activeModel.sessionStreamBanner?.title, "Live session stream reconnecting");
  assert.equal(activeModel.toolbar.canSave, false);
  assert.equal(activeModel.toolbar.canSplit, true);
  assert.equal(activeModel.screen?.sequenceBadge.label, "seq 7");
  assert.equal(activeModel.capabilities?.degradedReasons.length, 2);
  assert.equal(inputModel.draft, "pwd");
  assert.equal(inputModel.canWrite, true);
});
