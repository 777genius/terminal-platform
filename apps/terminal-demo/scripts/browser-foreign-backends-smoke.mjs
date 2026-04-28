#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";
import { fileURLToPath } from "node:url";
import WebSocket from "ws";

import {
  launchChromeWithCdp,
  pipeProcess,
  removeChromeUserDataDir,
  resolveRuntimeEvaluationValue,
  stopProcess,
  waitForHttpServer,
} from "./chrome-cdp-smoke.mjs";
import { resolveSpawnCommand } from "./dev-launcher-utils.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "../..");
const viteCliPath = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
const rendererPort = process.env.TERMINAL_DEMO_FOREIGN_SMOKE_RENDERER_PORT ?? "4274";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;
const cdpPort = process.env.TERMINAL_DEMO_FOREIGN_SMOKE_CDP_PORT ?? "9227";
const runtimeSlug = process.env.TERMINAL_DEMO_FOREIGN_SMOKE_RUNTIME_SLUG
  ?? `terminal-demo-foreign-browser-smoke-${process.pid}-${Date.now().toString(16)}`;
const sessionStorePath = path.join(
  os.tmpdir(),
  `terminal-demo-foreign-browser-smoke-store-${process.pid}-${Date.now()}.sqlite3`,
);
const zellijMinimum = [0, 44, 0];
const foreignBackends = process.platform === "win32" ? ["zellij"] : ["tmux", "zellij"];

let previewProcess = null;
let browserHostProcess = null;
let chromeProcess = null;
let chromeUserDataDir = null;
let tmuxSessionName = null;
let zellijSessionName = null;
let tempZellijBinDir = null;
let smokeEnv = process.env;
const windowsZellijProcessIds = [];

await main();

async function main() {
  try {
    runSync("npm", ["run", "build"], appRoot, smokeEnv);
    smokeEnv = await resolveForeignBackendEnv(foreignBackends);

    if (foreignBackends.includes("tmux")) {
      tmuxSessionName = uniqueName("tp-ui-tmux");
      startTmuxSession(tmuxSessionName, smokeEnv);
    }
    if (foreignBackends.includes("zellij")) {
      zellijSessionName = uniqueName("tp-ui-zellij");
      await startZellijSession(zellijSessionName, smokeEnv);
    }

    previewProcess = spawn(process.execPath, [
      viteCliPath,
      "preview",
      "--host",
      "127.0.0.1",
      "--port",
      rendererPort,
      "--strictPort",
    ], {
      cwd: appRoot,
      env: smokeEnv,
      stdio: "pipe",
    });
    pipeProcess(previewProcess, "[foreign-browser-smoke:preview]");
    await waitForHttpServer(rendererUrl, {
      child: previewProcess,
      label: "Renderer preview",
    });

    const chromeLaunch = await launchChromeWithCdp({
      appRoot,
      binaryMissingMessage: "Chrome binary not found. Set TERMINAL_DEMO_CHROME_BIN to run foreign backend browser smoke.",
      cdpPort,
      extraArgs: [
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--disable-software-rasterizer",
        "--no-first-run",
        "--no-default-browser-check",
        "--no-sandbox",
      ],
      headlessModeEnv: "TERMINAL_DEMO_FOREIGN_SMOKE_HEADLESS_MODE",
      logPrefix: "foreign-browser-smoke:chrome",
      profilePrefix: "terminal-demo-foreign-browser-smoke-profile",
    });
    chromeProcess = chromeLaunch.child;
    chromeUserDataDir = chromeLaunch.userDataDir;

    const browserUrl = await startBrowserHost(rendererUrl, {
      autoStartSession: "0",
      sessionStorePath,
    });
    const result = await runForeignBackendScenario(browserUrl, {
      backendSessions: {
        ...(tmuxSessionName ? { tmux: tmuxSessionName } : {}),
        ...(zellijSessionName ? { zellij: zellijSessionName } : {}),
      },
    });

    if (result.issues.length > 0) {
      throw new Error(`Foreign backend browser smoke reported runtime issues: ${JSON.stringify(result.issues)}`);
    }

    for (const backend of foreignBackends) {
      const imported = result.imports[backend];
      if (
        !imported?.importClicked
        || !imported.imported
        || imported.attachedBackend !== backend
        || !imported.commandSent
        || !imported.screenText?.includes(imported.marker)
      ) {
        throw new Error(`Foreign backend ${backend} did not import through UI correctly: ${JSON.stringify(imported)}`);
      }
      if (
        backend === "zellij"
        && (
          !imported.muxActions?.newTabCreated
          || !imported.muxActions?.renamed
          || !imported.muxActions?.pasteMarkerSeen
          || !imported.muxActions?.closedTab
          || !imported.muxActions?.unsupportedSplitRejected
          || !imported.muxActions?.focusCapabilitiesMatchPlatform
        )
      ) {
        throw new Error(`Foreign backend zellij mux actions failed: ${JSON.stringify(imported.muxActions)}`);
      }
    }

    if (
      result.beforeImport.connectionState !== "ready"
      || !result.beforeImport.hasForeignSection
      || !result.beforeImport.hasRefresh
      || foreignBackends.some((backend) => (result.beforeImport.discoveredCounts?.[backend] ?? 0) < 1)
      || result.beforeImport.documentHorizontalOverflow > 1
    ) {
      throw new Error(`Foreign backend UI did not expose discovered sessions: ${JSON.stringify(result.beforeImport)}`);
    }
  } finally {
    await shutdown();
  }
}

async function resolveForeignBackendEnv(backends) {
  if (backends.includes("tmux")) {
    assertCommand("tmux", ["-V"], "tmux is required for foreign backend browser smoke.");
  }
  let env = { ...process.env };
  let version = resolveZellijVersion(env);

  if (!isVersionAtLeast(version, zellijMinimum)) {
    if (process.env.TERMINAL_DEMO_FOREIGN_AUTO_INSTALL_ZELLIJ === "0") {
      throw new Error(`Zellij ${formatVersion(zellijMinimum)}+ is required; found ${version.raw}.`);
    }

    tempZellijBinDir = path.join(os.tmpdir(), `terminal-demo-zellij-${process.pid}-${Date.now()}`);
    const python = resolvePython();
    const installEnv = { ...env };
    if (process.env.SSL_CERT_FILE) {
      installEnv.SSL_CERT_FILE = process.env.SSL_CERT_FILE;
    }
    runSync(python, [
      path.join(repoRoot, ".github", "scripts", "install_zellij.py"),
      "--out",
      tempZellijBinDir,
    ], repoRoot, installEnv);
    env = {
      ...env,
      PATH: `${tempZellijBinDir}${path.delimiter}${env.PATH ?? ""}`,
    };
    version = resolveZellijVersion(env);
  }

  if (!isVersionAtLeast(version, zellijMinimum)) {
    throw new Error(`Zellij ${formatVersion(zellijMinimum)}+ is required; found ${version.raw}.`);
  }

  const tools = [
    backends.includes("tmux") ? `tmux ${runCapture("tmux", ["-V"], appRoot, env).trim()}` : "tmux skipped",
    version.raw,
  ];
  process.stdout.write(`Foreign backend smoke tools - ${tools.join(", ")}\n`);
  return env;
}

function startTmuxSession(sessionName, env) {
  runCapture("tmux", ["kill-session", "-t", sessionName], appRoot, env, { allowFailure: true });
  runCapture("tmux", [
    "new-session",
    "-d",
    "-s",
    sessionName,
    "sh",
    "-lc",
    "printf 'hello from tmux ui smoke\\n'; exec cat",
  ], appRoot, env);
  runCapture("tmux", [
    "new-window",
    "-d",
    "-t",
    sessionName,
    "-n",
    "logs",
    "sh",
    "-lc",
    "printf 'tmux logs ready\\n'; exec cat",
  ], appRoot, env);
}

async function startZellijSession(sessionName, env) {
  runCapture("zellij", ["kill-session", sessionName], appRoot, env, { allowFailure: true });

  if (process.platform === "win32") {
    startWindowsZellijPty(sessionName, env);
    await waitFor(async () => {
      const sessions = runCapture("zellij", ["list-sessions", "--short", "--no-formatting"], appRoot, env, {
        allowFailure: true,
      });
      return sessions.split("\n").map((line) => line.trim()).includes(sessionName);
    }, `zellij session ${sessionName} to appear`);
    return;
  }

  runCapture("zellij", ["attach", "--create-background", sessionName], appRoot, env, {
    allowFailure: true,
    timeout: 15_000,
  });

  await waitFor(async () => {
    const sessions = runCapture("zellij", ["list-sessions", "--short", "--no-formatting"], appRoot, env, {
      allowFailure: true,
    });
    return sessions.split("\n").map((line) => line.trim()).includes(sessionName);
  }, `zellij session ${sessionName} to appear`);
}

function startWindowsZellijPty(sessionName, env) {
  const script = [
    "$ErrorActionPreference = 'Stop'",
    "$zellij = (Get-Command zellij.exe -ErrorAction Stop).Source",
    `$process = Start-Process -FilePath $zellij -ArgumentList @('attach','--create',${quotePowerShell(sessionName)}) -WindowStyle Hidden -PassThru`,
    "Write-Output $process.Id",
  ].join("; ");
  const output = runCapture(windowsPowerShellPath(), ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script], appRoot, env);
  const processId = Number(output.trim().match(/\d+/u)?.[0] ?? 0);
  if (!Number.isInteger(processId) || processId <= 0) {
    throw new Error(`Failed to capture Windows Zellij process id: ${output.trim()}`);
  }
  windowsZellijProcessIds.push(processId);
}

async function runForeignBackendScenario(browserUrl, expected) {
  const backendSessions = Object.entries(expected.backendSessions);
  const target = await fetch(`http://127.0.0.1:${cdpPort}/json/new?${encodeURIComponent(browserUrl)}`, {
    method: "PUT",
  }).then((response) => response.json());
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await onceSocketOpen(socket);

  let id = 0;
  const pending = new Map();
  const issues = [];

  socket.on("message", (data) => {
    const message = JSON.parse(data.toString());
    if (message.id && pending.has(message.id)) {
      const request = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) {
        request.reject(new Error(message.error.message));
      } else {
        request.resolve(message.result);
      }
      return;
    }

    if (message.method === "Log.entryAdded") {
      const entry = message.params.entry;
      if (entry.level === "error") {
        issues.push({ type: "log", source: entry.source, text: entry.text });
      }
      return;
    }

    if (message.method === "Runtime.exceptionThrown") {
      issues.push({
        type: "exception",
        text: message.params.exceptionDetails?.text ?? "Runtime exception",
      });
    }
  });

  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const requestId = ++id;
    pending.set(requestId, { resolve, reject });
    socket.send(JSON.stringify({ id: requestId, method, params }));
  });

  try {
    await send("Page.enable");
    await send("Page.bringToFront").catch(() => undefined);
    await send("Runtime.enable");
    await send("Log.enable");
    await send("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 1100,
      deviceScaleFactor: 1,
      mobile: false,
    });

    await waitForBrowser(send, `state ready with discovered ${backendSessions.map(([, title]) => title).join(", ")}`, `(() => {
      const state = window.terminalDemoDebug?.getState?.();
      const discovered = state?.catalog?.discoveredSessions ?? {};
      const expected = ${JSON.stringify(expected.backendSessions)};
      return state?.connection?.state === 'ready'
        && Object.entries(expected).every(([backend, title]) =>
          (discovered[backend] ?? []).some((session) => session.title === title)
        );
    })()`);

    const beforeImport = await evaluate(send, `(() => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const navigationDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-navigation-drawer"]') ?? null;
      if (navigationDrawer && !navigationDrawer.hasAttribute('open')) {
        navigationDrawer.querySelector('summary')?.click();
      }
      const sessionListRoot = workspaceRoot?.querySelector('tp-terminal-session-list')?.shadowRoot ?? null;
      const state = window.terminalDemoDebug?.getState?.();
      const buttons = [...(sessionListRoot?.querySelectorAll('[data-testid="tp-discovered-session-import"]') ?? [])];
      return {
        connectionState: state?.connection?.state ?? null,
        hasForeignSection: Boolean(sessionListRoot?.querySelector('[data-testid="tp-foreign-backends"]')),
        hasRefresh: Boolean(sessionListRoot?.querySelector('[data-testid="tp-foreign-refresh"]')),
        tmuxDiscovered: buttons.filter((button) => button.getAttribute('data-backend') === 'tmux').length,
        zellijDiscovered: buttons.filter((button) => button.getAttribute('data-backend') === 'zellij').length,
        discoveredCounts: buttons.reduce((counts, button) => {
          const backend = button.getAttribute('data-backend');
          counts[backend] = (counts[backend] ?? 0) + 1;
          return counts;
        }, {}),
        documentHorizontalOverflow: Math.max(0, document.documentElement.scrollWidth - document.documentElement.clientWidth),
      };
    })()`);

    const imports = {};
    for (const [backend, title] of backendSessions) {
      imports[backend] = await importBackendViaUi(send, backend, title, `${backend}-ui-smoke-marker`);
    }

    return {
      beforeImport,
      imports,
      issues,
    };
  } finally {
    await closeWebSocket(socket);
    await closePageTarget(target.id);
  }
}

async function importBackendViaUi(send, backend, title, marker) {
  const importClicked = await evaluate(send, `(() => {
    const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
    const navigationDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-navigation-drawer"]') ?? null;
    if (navigationDrawer && !navigationDrawer.hasAttribute('open')) {
      navigationDrawer.querySelector('summary')?.click();
    }
    const sessionListRoot = workspaceRoot?.querySelector('tp-terminal-session-list')?.shadowRoot ?? null;
    const button = [...(sessionListRoot?.querySelectorAll('[data-testid="tp-discovered-session-import"]') ?? [])]
      .find((candidate) =>
        candidate.getAttribute('data-backend') === ${JSON.stringify(backend)}
        && candidate.closest('[data-testid="tp-discovered-session"]')?.getAttribute('data-session-title') === ${JSON.stringify(title)}
      );
    if (!button) {
      return false;
    }
    button.click();
    return true;
  })()`);

  await waitForBrowser(send, `${backend} imported and attached`, `(() => {
    const state = window.terminalDemoDebug?.getState?.();
    return state?.catalog?.sessions?.some((session) =>
      session.route.backend === ${JSON.stringify(backend)}
      && session.title === ${JSON.stringify(title)}
    ) && state?.attachedSession?.session?.route?.backend === ${JSON.stringify(backend)};
  })()`);

  const commandSent = await evaluate(send, `(async () => {
    const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
    const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
    const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
    const button = commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null;
    if (!textarea || !button) {
      return false;
    }
    const descriptor = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value');
    descriptor?.set?.call(textarea, ${JSON.stringify(`echo ${marker}`)} );
    textarea.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    if (button.disabled) {
      return false;
    }
    button.click();
    return true;
  })()`);

  await waitForBrowser(send, `${backend} screen marker`, `(() => {
    const state = window.terminalDemoDebug?.getState?.();
    const screenText = state?.attachedSession?.focused_screen?.surface?.lines
      ?.map((line) => line.text)
      .join('\\n') ?? '';
    return screenText.includes(${JSON.stringify(marker)});
  })()`);

  const afterCommand = await evaluate(send, `(() => {
    const state = window.terminalDemoDebug?.getState?.();
    const screenText = state?.attachedSession?.focused_screen?.surface?.lines
      ?.map((line) => line.text)
      .join('\\n') ?? '';
    return {
      imported: state?.catalog?.sessions?.some((session) =>
        session.route.backend === ${JSON.stringify(backend)}
        && session.title === ${JSON.stringify(title)}
      ) ?? false,
      attachedBackend: state?.attachedSession?.session?.route?.backend ?? null,
      attachedTitle: state?.attachedSession?.session?.title ?? null,
      screenSource: state?.attachedSession?.focused_screen?.source ?? null,
      screenText,
    };
  })()`);
  const muxActions = backend === "zellij"
    ? await exerciseZellijMuxActions(send, title)
    : null;

  return {
    ...afterCommand,
    commandSent,
    importClicked,
    marker,
    muxActions,
  };
}

async function exerciseZellijMuxActions(send, title) {
  const newTabTitle = uniqueName("zellij-mux-tab");
  const renamedTabTitle = uniqueName("zellij-mux-renamed");
  const pasteMarker = uniqueName("zellij-paste-marker");
  const expectedTabFocus = process.platform !== "win32";
  const expectedPaneFocus = process.platform !== "win32";

  const before = await evaluate(send, `(() => {
    const state = window.terminalDemoDebug?.getState?.();
    const session = state?.attachedSession?.session ?? null;
    const topology = state?.attachedSession?.topology ?? null;
    const capabilities = state?.catalog?.backendCapabilities?.zellij?.capabilities ?? null;
    const focusedTab = topology?.tabs?.find((tab) => tab.tab_id === topology.focused_tab)
      ?? topology?.tabs?.[0]
      ?? null;
    return {
      sessionId: session?.session_id ?? null,
      attachedBackend: session?.route?.backend ?? null,
      attachedTitle: session?.title ?? null,
      tabCount: topology?.tabs?.length ?? 0,
      focusedTabId: focusedTab?.tab_id ?? null,
      focusedPaneId: focusedTab?.focused_pane ?? null,
      capabilities,
    };
  })()`);

  if (!before.sessionId || before.attachedBackend !== "zellij" || before.attachedTitle !== title) {
    return {
      ok: false,
      reason: "zellij session was not attached before mux action exercise",
      before,
    };
  }

  const focusCapabilitiesMatchPlatform = Boolean(
    before.capabilities
    && before.capabilities.tab_focus === expectedTabFocus
    && before.capabilities.pane_focus === expectedPaneFocus,
  );

  const dispatchResult = await evaluate(send, `(async () => {
    const commands = window.terminalDemoDebug?.controller?.commands ?? null;
    if (!commands?.dispatchMuxCommand || !commands?.attachSession) {
      return { ok: false, reason: 'workspace commands missing' };
    }
    await commands.dispatchMuxCommand(${JSON.stringify(before.sessionId)}, {
      kind: 'new_tab',
      title: ${JSON.stringify(newTabTitle)},
    });
    await commands.attachSession(${JSON.stringify(before.sessionId)});
    return { ok: true };
  })()`);
  if (!dispatchResult.ok) {
    return {
      ok: false,
      reason: dispatchResult.reason ?? "new_tab dispatch failed",
      before,
      focusCapabilitiesMatchPlatform,
    };
  }

  await waitForBrowser(send, "zellij new tab to appear", `(() => {
    const tabs = window.terminalDemoDebug?.getState?.()?.attachedSession?.topology?.tabs ?? [];
    return tabs.some((tab) => tab.title === ${JSON.stringify(newTabTitle)});
  })()`);

  const afterNewTab = await evaluate(send, `(() => {
    const state = window.terminalDemoDebug?.getState?.();
    const topology = state?.attachedSession?.topology ?? null;
    const newTab = topology?.tabs?.find((tab) => tab.title === ${JSON.stringify(newTabTitle)}) ?? null;
    return {
      tabCount: topology?.tabs?.length ?? 0,
      focusedTabId: topology?.focused_tab ?? null,
      newTabId: newTab?.tab_id ?? null,
      newPaneId: newTab?.focused_pane ?? null,
    };
  })()`);
  const newTabCreated = Boolean(
    afterNewTab.newTabId
    && afterNewTab.newPaneId
    && afterNewTab.tabCount > before.tabCount,
  );
  if (!newTabCreated) {
    return {
      ok: false,
      reason: "new_tab did not create an importable zellij tab",
      before,
      afterNewTab,
      focusCapabilitiesMatchPlatform,
    };
  }

  await evaluate(send, `(async () => {
    const commands = window.terminalDemoDebug?.controller?.commands;
    await commands.dispatchMuxCommand(${JSON.stringify(before.sessionId)}, {
      kind: 'send_paste',
      pane_id: ${JSON.stringify(afterNewTab.newPaneId)},
      data: ${JSON.stringify(`echo ${pasteMarker}`)},
    });
    await commands.dispatchMuxCommand(${JSON.stringify(before.sessionId)}, {
      kind: 'send_input',
      pane_id: ${JSON.stringify(afterNewTab.newPaneId)},
      data: '\\r',
    });
    await commands.attachSession(${JSON.stringify(before.sessionId)});
    return true;
  })()`);

  const zellijPasteScreen = await waitForZellijTabScreenMarker(newTabTitle, pasteMarker);
  const afterPaste = await evaluate(send, `(() => {
    const screenText = window.terminalDemoDebug?.getState?.()?.attachedSession?.focused_screen?.surface?.lines
      ?.map((line) => line.text)
      .join('\\n') ?? '';
    return {
      focusedScreenPasteMarkerSeen: screenText.includes(${JSON.stringify(pasteMarker)}),
    };
  })()`);

  await evaluate(send, `(async () => {
    const commands = window.terminalDemoDebug?.controller?.commands;
    await commands.dispatchMuxCommand(${JSON.stringify(before.sessionId)}, {
      kind: 'rename_tab',
      tab_id: ${JSON.stringify(afterNewTab.newTabId)},
      title: ${JSON.stringify(renamedTabTitle)},
    });
    await commands.attachSession(${JSON.stringify(before.sessionId)});
    return true;
  })()`);

  await waitForBrowser(send, "zellij renamed tab", `(() => {
    const tabs = window.terminalDemoDebug?.getState?.()?.attachedSession?.topology?.tabs ?? [];
    return tabs.some((tab) =>
      tab.tab_id === ${JSON.stringify(afterNewTab.newTabId)}
      && tab.title === ${JSON.stringify(renamedTabTitle)}
    );
  })()`);

  const unsupportedSplit = await evaluate(send, `(async () => {
    const commands = window.terminalDemoDebug?.controller?.commands;
    try {
      await commands.dispatchMuxCommand(${JSON.stringify(before.sessionId)}, {
        kind: 'split_pane',
        pane_id: ${JSON.stringify(afterNewTab.newPaneId)},
        direction: 'horizontal',
      });
      return { rejected: false, message: null };
    } catch (error) {
      return { rejected: true, message: error instanceof Error ? error.message : String(error) };
    }
  })()`);

  await evaluate(send, `(async () => {
    const commands = window.terminalDemoDebug?.controller?.commands;
    await commands.dispatchMuxCommand(${JSON.stringify(before.sessionId)}, {
      kind: 'close_tab',
      tab_id: ${JSON.stringify(afterNewTab.newTabId)},
    });
    await commands.attachSession(${JSON.stringify(before.sessionId)});
    return true;
  })()`);

  await waitForBrowser(send, "zellij tab closed", `(() => {
    const tabs = window.terminalDemoDebug?.getState?.()?.attachedSession?.topology?.tabs ?? [];
    return !tabs.some((tab) => tab.tab_id === ${JSON.stringify(afterNewTab.newTabId)});
  })()`);

  const afterClose = await evaluate(send, `(() => {
    const state = window.terminalDemoDebug?.getState?.();
    const topology = state?.attachedSession?.topology ?? null;
    const splitButton = document
      .querySelector('tp-terminal-workspace')
      ?.shadowRoot
      ?.querySelector('tp-terminal-pane-tree')
      ?.shadowRoot
      ?.querySelector('[data-testid="tp-split-right"]') ?? null;
    return {
      tabCount: topology?.tabs?.length ?? 0,
      renamedStillPresent: topology?.tabs?.some((tab) => tab.title === ${JSON.stringify(renamedTabTitle)}) ?? false,
      splitButtonDisabled: splitButton?.disabled ?? null,
    };
  })()`);

  const splitMessage = String(unsupportedSplit.message ?? "");
  const unsupportedSplitRejected = Boolean(
    unsupportedSplit.rejected
    && (
      splitMessage.includes("do not support this command")
      || splitMessage.includes("UnsupportedByBackend")
      || splitMessage.includes("unsupported")
    ),
  );

  return {
    ok: true,
    before,
    afterNewTab,
    afterPaste,
    afterClose,
    zellijPasteScreen,
    focusCapabilitiesMatchPlatform,
    newTabCreated,
    renamed: !afterClose.renamedStillPresent,
    pasteMarker,
    pasteMarkerSeen: zellijPasteScreen.markerSeen,
    closedTab: afterClose.tabCount === before.tabCount,
    unsupportedSplitRejected,
    unsupportedSplitMessage: unsupportedSplit.message,
    splitButtonDisabled: afterClose.splitButtonDisabled,
  };
}

async function waitForZellijTabScreenMarker(tabTitle, marker) {
  let latest = null;
  await waitFor(() => {
    latest = readZellijTabScreen(tabTitle);
    return latest?.screenText.includes(marker) ?? false;
  }, `zellij tab ${tabTitle} screen marker`);

  return {
    ...latest,
    markerSeen: latest?.screenText.includes(marker) ?? false,
    screenTextPreview: compactText(latest?.screenText ?? ""),
  };
}

function readZellijTabScreen(tabTitle) {
  if (!zellijSessionName) {
    return null;
  }

  try {
    const tabs = JSON.parse(runCapture(
      "zellij",
      ["--session", zellijSessionName, "action", "list-tabs", "--json"],
      appRoot,
      smokeEnv,
    ));
    const tab = tabs.find((candidate) => candidate?.name === tabTitle);
    if (!tab) {
      return null;
    }

    const panes = JSON.parse(runCapture(
      "zellij",
      ["--session", zellijSessionName, "action", "list-panes", "--json"],
      appRoot,
      smokeEnv,
    ));
    const pane = panes.find((candidate) =>
      candidate?.tab_id === tab.tab_id
      && !candidate?.is_plugin
      && !candidate?.is_floating
    );
    if (!pane) {
      return null;
    }

    const paneRef = `terminal_${pane.id}`;
    const screenText = runCapture(
      "zellij",
      ["--session", zellijSessionName, "action", "dump-screen", "--pane-id", paneRef],
      appRoot,
      smokeEnv,
    );
    return {
      tabId: tab.tab_id,
      paneId: pane.id,
      paneRef,
      screenText,
    };
  } catch {
    return null;
  }
}

async function startBrowserHost(rendererUrlValue, options) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error("Timed out waiting for TERMINAL_DEMO_BROWSER_URL"));
    }, 20_000);

    browserHostProcess = spawn(process.execPath, ["./dist/host/browser/index.js"], {
      cwd: appRoot,
      env: {
        ...smokeEnv,
        TERMINAL_DEMO_AUTO_START_SESSION: options.autoStartSession,
        TERMINAL_DEMO_RENDERER_URL: rendererUrlValue,
        TERMINAL_DEMO_BROWSER_BOOTSTRAP_SCOPE: "dist-only",
        TERMINAL_DEMO_RUNTIME_SLUG: runtimeSlug,
        TERMINAL_DEMO_SESSION_STORE_PATH: options.sessionStorePath,
      },
      stdio: ["ignore", "pipe", "pipe"],
    });

    const onLine = (line) => {
      process.stdout.write(`${line}\n`);
      const match = line.match(/^TERMINAL_DEMO_BROWSER_URL=(.+)$/u);
      if (match) {
        clearTimeout(timeout);
        cleanup();
        resolve(match[1]);
      }
    };

    const cleanup = () => {
      stdout.close();
      stderr.close();
      browserHostProcess.off("exit", onExit);
    };

    const onExit = (code) => {
      clearTimeout(timeout);
      cleanup();
      reject(new Error(`Browser host exited before exposing browser URL - exit code ${code ?? 0}`));
    };

    const stdout = readline.createInterface({ input: browserHostProcess.stdout });
    const stderr = readline.createInterface({ input: browserHostProcess.stderr });
    stdout.on("line", onLine);
    stderr.on("line", onLine);
    browserHostProcess.on("exit", onExit);
  });
}

function evaluate(send, expression) {
  let timeoutId;
  const evaluation = send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  }).then(resolveRuntimeEvaluationValue);
  const timeout = new Promise((_, reject) => {
    timeoutId = setTimeout(() => {
      reject(new Error("Timed out waiting for browser evaluation"));
    }, 60_000);
  });

  return Promise.race([evaluation, timeout]).finally(() => {
    clearTimeout(timeoutId);
  });
}

async function waitForBrowser(send, label, expression, timeoutMs = 20_000) {
  await waitFor(async () => evaluate(send, expression), label, timeoutMs);
}

async function waitFor(probe, label, timeoutMs = 20_000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (await probe()) {
      return;
    }
    await sleep(200);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function shutdown() {
  await stopProcess(browserHostProcess);
  await stopProcess(previewProcess);
  await stopProcess(chromeProcess);
  if (tmuxSessionName) {
    runCapture("tmux", ["kill-session", "-t", tmuxSessionName], appRoot, smokeEnv, { allowFailure: true });
  }
  if (zellijSessionName) {
    runCapture("zellij", ["kill-session", zellijSessionName], appRoot, smokeEnv, { allowFailure: true });
  }
  for (const processId of windowsZellijProcessIds) {
    runCapture(
      windowsPowerShellPath(),
      ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", `Stop-Process -Id ${processId} -Force -ErrorAction SilentlyContinue`],
      appRoot,
      smokeEnv,
      { allowFailure: true },
    );
  }
  await removeChromeUserDataDir(chromeUserDataDir);
  await removeSessionStore(sessionStorePath);
  if (tempZellijBinDir) {
    await fs.rm(tempZellijBinDir, { recursive: true, force: true });
  }
}

async function removeSessionStore(storePath) {
  await Promise.all([
    fs.rm(storePath, { force: true }),
    fs.rm(`${storePath}-shm`, { force: true }),
    fs.rm(`${storePath}-wal`, { force: true }),
  ]);
}

function runSync(command, args, cwd, env) {
  const resolved = resolveSpawnCommand(command, args, env);
  const result = spawnSync(resolved.command, resolved.args, {
    cwd,
    env,
    shell: resolved.shell,
    stdio: "inherit",
  });

  if (result.error) {
    throw new Error(`${command} ${args.join(" ")} failed: ${result.error.message}`);
  }

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

function runCapture(command, args, cwd, env, options = {}) {
  const result = spawnSync(command, args, {
    cwd,
    env,
    encoding: "utf8",
    timeout: options.timeout ?? 10_000,
  });
  if ((result.error || result.status !== 0 || result.signal) && !options.allowFailure) {
    const output = [result.error?.message, result.stderr, result.stdout]
      .filter(Boolean)
      .map((value) => String(value).trim())
      .filter(Boolean)
      .join("\n");
    const reason = output || `exit code ${result.status ?? "unknown"}${result.signal ? `, signal ${result.signal}` : ""}`;
    throw new Error(`${command} ${args.join(" ")} failed: ${reason}`);
  }
  return result.stdout ?? "";
}

function windowsPowerShellPath() {
  if (process.platform !== "win32") {
    return "powershell.exe";
  }
  const windowsRoot = process.env.SystemRoot || process.env.WINDIR || "C:\\Windows";
  return path.join(windowsRoot, "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
}

function assertCommand(command, args, message) {
  const result = spawnSync(command, args, {
    cwd: appRoot,
    env: process.env,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(message);
  }
}

function resolveZellijVersion(env) {
  const raw = runCapture("zellij", ["--version"], appRoot, env, { allowFailure: true }).trim();
  const parsed = raw.match(/(\d+)\.(\d+)\.(\d+)/u)?.slice(1).map(Number) ?? [0, 0, 0];
  return {
    raw: raw || "zellij not found",
    parsed,
  };
}

function resolvePython() {
  for (const candidate of ["python3", "python"]) {
    const result = spawnSync(candidate, ["--version"], {
      cwd: repoRoot,
      env: process.env,
      encoding: "utf8",
    });
    if (result.status === 0) {
      return candidate;
    }
  }
  throw new Error("python3 or python is required to install the project-scoped Zellij test binary.");
}

function isVersionAtLeast(version, minimum) {
  for (let index = 0; index < minimum.length; index += 1) {
    if (version.parsed[index] > minimum[index]) {
      return true;
    }
    if (version.parsed[index] < minimum[index]) {
      return false;
    }
  }
  return true;
}

function formatVersion(parts) {
  return parts.join(".");
}

function compactText(value, limit = 240) {
  const normalized = value.replace(/\s+/gu, " ").trim();
  if (normalized.length <= limit) {
    return normalized;
  }

  return `${normalized.slice(0, limit)}...`;
}

function uniqueName(prefix) {
  return `${prefix}-${process.pid}-${Date.now().toString(16)}`;
}

function quotePowerShell(value) {
  return `'${String(value).replace(/'/gu, "''")}'`;
}

function onceSocketOpen(socket) {
  return new Promise((resolve, reject) => {
    socket.once("open", resolve);
    socket.once("error", reject);
  });
}

function closeWebSocket(socket) {
  return new Promise((resolve) => {
    socket.once("close", resolve);
    socket.close();
    setTimeout(resolve, 500);
  });
}

async function closePageTarget(targetId) {
  await fetch(`http://127.0.0.1:${cdpPort}/json/close/${targetId}`).catch(() => undefined);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
