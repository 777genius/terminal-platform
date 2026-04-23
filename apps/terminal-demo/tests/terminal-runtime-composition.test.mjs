import test from "node:test";
import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import {
  TerminalDemoWorkspaceScreen,
  createStaticWorkspaceKernel,
} from "../dist/renderer/app/TerminalDemoWorkspaceApp.js";

test("renderer app mounts the sdk react workspace shell", () => {
  const snapshot = {
    connection: {
      state: "ready",
      handshake: {
        protocol_version: { major: 0, minor: 2 },
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
          session_health: true,
        },
        available_backends: ["native"],
        session_scope: "terminal-demo",
      },
      lastError: null,
    },
    catalog: {
      sessions: [
        {
          session_id: "session-1",
          origin: {
            backend: "native",
            authority: "local_daemon",
            foreign_reference_label: null,
          },
          route: {
            backend: "native",
            authority: "local_daemon",
            foreign_reference: null,
          },
          title: "SDK Workspace",
          degraded_semantics: [],
        },
      ],
      savedSessions: [],
      discoveredSessions: {},
      backendCapabilities: {},
    },
    selection: {
      activeSessionId: "session-1",
      activePaneId: "pane-1",
    },
    attachedSession: {
      session: {
        session_id: "session-1",
        origin: {
          backend: "native",
          authority: "local_daemon",
          foreign_reference_label: null,
        },
        route: {
          backend: "native",
          authority: "local_daemon",
          foreign_reference: null,
        },
        title: "SDK Workspace",
        degraded_semantics: [],
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
            title: "Shell",
            focused_pane: "pane-1",
            root: {
              kind: "leaf",
              pane_id: "pane-1",
            },
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
          title: "Shell",
          cursor: null,
          lines: [{ text: "demo" }],
        },
      },
    },
    diagnostics: [],
    drafts: {},
    theme: {
      themeId: "terminal-platform-default",
    },
  };
  const kernel = createStaticWorkspaceKernel(snapshot);

  const markup = renderToStaticMarkup(createElement(TerminalDemoWorkspaceScreen, {
    config: {
      controlPlaneUrl: "ws://127.0.0.1:4100/terminal-gateway/control?token=abc",
      sessionStreamUrl: "ws://127.0.0.1:4100/terminal-gateway/stream?token=abc",
      runtimeSlug: "terminal-demo",
    },
    kernel,
  }));

  assert.match(markup, /Terminal Platform/);
  assert.match(markup, /NativeMux workspace/);
  assert.match(markup, /Session launcher/);
  assert.match(markup, /data-testid="terminal-workspace-host"/);
  assert.match(markup, /tp-terminal-workspace/);
  assert.match(markup, /SDK Workspace/);
});
