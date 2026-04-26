import test from "node:test";
import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import {
  TerminalDemoWorkspaceScreen,
  createDemoPreviewBackendCapabilities,
  createDemoPreviewWorkspaceSnapshot,
  createStaticWorkspaceKernel,
} from "../dist/renderer-node-test/renderer/app/TerminalDemoWorkspaceApp.js";
import { resolveTerminalDemoShellChromeState } from "../dist/renderer-node-test/renderer/app/terminal-demo-shell-chrome.js";
import { buildTerminalRuntimeBrowserUrl } from "../dist/renderer-node-test/features/terminal-runtime-host/contracts/index.js";
import { resolveDemoDefaultShellProgram } from "../dist/renderer-node-test/features/terminal-runtime-host/main/composition/shell-policy.js";

test("renderer app mounts the sdk react workspace shell", () => {
  const config = {
    controlPlaneUrl: "ws://127.0.0.1:4100/terminal-gateway/control?token=abc",
    sessionStreamUrl: "ws://127.0.0.1:4100/terminal-gateway/stream?token=abc",
    runtimeSlug: "terminal-demo",
  };
  const snapshot = createDemoPreviewWorkspaceSnapshot(config);
  const kernel = createStaticWorkspaceKernel(snapshot);

  const markup = renderToStaticMarkup(createElement(TerminalDemoWorkspaceScreen, {
    config,
    kernel,
  }));

  assert.match(markup, /data-shell-mode="terminal"/);
  assert.doesNotMatch(markup, /data-testid="terminal-demo-workspace-hero"/);
  assert.match(markup, /Shell controls/);
  assert.match(markup, /data-testid="terminal-workspace-host"/);
  assert.match(markup, /tp-terminal-workspace/);
  assert.match(markup, /Terminal Platform preview/);
  assert.equal(snapshot.connection.handshake.session_scope, "terminal-demo");
  assert.equal(snapshot.drafts["preview-pane-main"], "git status");
  assert.equal(snapshot.catalog.backendCapabilities.native.capabilities.pane_input_write, true);
  assert.equal(snapshot.catalog.backendCapabilities.native.capabilities.pane_paste_write, true);
  assert.equal(snapshot.catalog.backendCapabilities.native.capabilities.explicit_session_save, true);
});

test("demo shell chrome hides overview content once a terminal is active", () => {
  const terminalSnapshot = createDemoPreviewWorkspaceSnapshot({ runtimeSlug: "terminal-demo" });
  const terminalChrome = resolveTerminalDemoShellChromeState(terminalSnapshot);

  assert.deepEqual(terminalChrome, {
    hasActiveSession: true,
    mode: "terminal",
    density: "focus",
    showWorkspaceHero: false,
    launcherTitle: "Shell controls",
    advancedToolsLabel: "Tools",
  });

  const launcherChrome = resolveTerminalDemoShellChromeState({
    ...terminalSnapshot,
    attachedSession: null,
    catalog: {
      ...terminalSnapshot.catalog,
      sessions: [],
    },
    selection: {
      activeSessionId: null,
      activePaneId: null,
    },
  });

  assert.equal(launcherChrome.mode, "launcher");
  assert.equal(launcherChrome.density, "browse");
  assert.equal(launcherChrome.showWorkspaceHero, true);
  assert.equal(launcherChrome.launcherTitle, "Session launcher");
});

test("static preview kernel exposes ready native backend capabilities", async () => {
  const snapshot = createDemoPreviewWorkspaceSnapshot({ runtimeSlug: "terminal-demo" });
  const kernel = createStaticWorkspaceKernel(snapshot);
  const nativeCapabilities = await kernel.commands.getBackendCapabilities("native");

  assert.deepEqual(nativeCapabilities, createDemoPreviewBackendCapabilities("native"));
  assert.equal(nativeCapabilities.capabilities.pane_input_write, true);
  await assert.rejects(
    () => kernel.commands.getBackendCapabilities("tmux"),
    /No static backend capabilities for tmux/,
  );
});

test("static preview kernel models command input without a native host", async () => {
  const kernel = createStaticWorkspaceKernel(createDemoPreviewWorkspaceSnapshot({ runtimeSlug: "terminal-demo" }));
  let notifications = 0;
  const unsubscribe = kernel.subscribe(() => {
    notifications += 1;
  });

  await kernel.commands.dispatchMuxCommand("preview-session-native", {
    kind: "send_input",
    pane_id: "preview-pane-main",
    data: "printf \"preview-ok\\n\"\n",
  });
  kernel.commands.recordCommandHistory("printf \"preview-ok\\n\"");
  kernel.commands.clearDraft("preview-pane-main");

  const updatedSnapshot = kernel.getSnapshot();
  const screen = updatedSnapshot.attachedSession?.focused_screen;
  assert.equal(screen?.sequence, 2n);
  assert.match(screen?.surface.lines.at(-2)?.text ?? "", /preview-ok/);
  assert.equal(screen?.surface.lines.at(-1)?.text, "preview runtime accepted input without native host");
  assert.equal(updatedSnapshot.drafts["preview-pane-main"], undefined);
  assert.equal(updatedSnapshot.commandHistory.entries.at(-1), "printf \"preview-ok\\n\"");
  assert.ok(notifications >= 3);

  unsubscribe();
});

test("static preview kernel models save layout as local demo state", async () => {
  const kernel = createStaticWorkspaceKernel(createDemoPreviewWorkspaceSnapshot({ runtimeSlug: "terminal-demo" }));
  await kernel.commands.dispatchMuxCommand("preview-session-native", { kind: "save_session" });

  const savedSession = kernel.getSnapshot().catalog.savedSessions[0];
  assert.equal(savedSession?.session_id, "preview-saved-session");
  assert.equal(savedSession?.route.external, null);
  assert.equal(savedSession?.compatibility.can_restore, true);
  assert.equal(savedSession?.restore_semantics.preserves_process_state, false);
});

test("browser bootstrap URL preserves host shell policy", () => {
  const url = new URL(buildTerminalRuntimeBrowserUrl("http://127.0.0.1:5173/", {
    controlPlaneUrl: "ws://127.0.0.1:4100/terminal-gateway/control?token=abc",
    demoDefaultShellProgram: "/bin/zsh",
    sessionStreamUrl: "ws://127.0.0.1:4100/terminal-gateway/stream?token=abc",
    runtimeSlug: "terminal-demo",
  }));

  assert.equal(url.searchParams.get("runtimeSlug"), "terminal-demo");
  assert.equal(url.searchParams.get("demoAutoStartSession"), null);
  assert.equal(url.searchParams.get("demoDefaultShellProgram"), "/bin/zsh");
});

test("demo default shell resolver prefers host policy and stable platform fallbacks", () => {
  assert.equal(
    resolveDemoDefaultShellProgram({
      env: { TERMINAL_DEMO_DEFAULT_SHELL: "/opt/homebrew/bin/fish", SHELL: "/bin/zsh" },
      platform: "darwin",
    }),
    "/opt/homebrew/bin/fish",
  );
  assert.equal(
    resolveDemoDefaultShellProgram({
      env: { SHELL: "/bin/zsh" },
      platform: "darwin",
    }),
    "/bin/zsh",
  );
  assert.equal(
    resolveDemoDefaultShellProgram({
      env: {},
      platform: "darwin",
    }),
    "zsh",
  );
  assert.equal(
    resolveDemoDefaultShellProgram({
      env: { ComSpec: "C:\\Windows\\System32\\cmd.exe" },
      platform: "win32",
    }),
    "C:\\Windows\\System32\\cmd.exe",
  );
});
