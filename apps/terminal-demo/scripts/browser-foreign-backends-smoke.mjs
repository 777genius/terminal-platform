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
  resolveRuntimeEvaluationValue,
  stopProcess,
  waitForHttpServer,
} from "./chrome-cdp-smoke.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "../..");
const viteCliPath = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
const rendererPort = process.env.TERMINAL_DEMO_FOREIGN_SMOKE_RENDERER_PORT ?? "4274";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;
const cdpPort = process.env.TERMINAL_DEMO_FOREIGN_SMOKE_CDP_PORT ?? "9227";
const sessionStorePath = path.join(
  os.tmpdir(),
  `terminal-demo-foreign-browser-smoke-store-${process.pid}-${Date.now()}.sqlite3`,
);
const zellijMinimum = [0, 44, 0];

let previewProcess = null;
let browserHostProcess = null;
let chromeProcess = null;
let chromeUserDataDir = null;
let tmuxSessionName = null;
let zellijSessionName = null;
let tempZellijBinDir = null;
let smokeEnv = process.env;

await main();

async function main() {
  try {
    if (process.platform === "win32") {
      throw new Error("tmux UI smoke is Unix-only; Windows acceptance covers Native + Zellij.");
    }

    runSync("npm", ["run", "build"], appRoot, smokeEnv);
    smokeEnv = await resolveForeignBackendEnv();

    tmuxSessionName = uniqueName("tp-ui-tmux");
    zellijSessionName = uniqueName("tp-ui-zellij");
    startTmuxSession(tmuxSessionName, smokeEnv);
    await startZellijSession(zellijSessionName, smokeEnv);

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
      tmuxSessionName,
      zellijSessionName,
    });

    if (result.issues.length > 0) {
      throw new Error(`Foreign backend browser smoke reported runtime issues: ${JSON.stringify(result.issues)}`);
    }

    for (const backend of ["tmux", "zellij"]) {
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
    }

    if (
      result.beforeImport.connectionState !== "ready"
      || !result.beforeImport.hasForeignSection
      || !result.beforeImport.hasRefresh
      || result.beforeImport.tmuxDiscovered < 1
      || result.beforeImport.zellijDiscovered < 1
      || result.beforeImport.documentHorizontalOverflow > 1
    ) {
      throw new Error(`Foreign backend UI did not expose discovered sessions: ${JSON.stringify(result.beforeImport)}`);
    }
  } finally {
    await shutdown();
  }
}

async function resolveForeignBackendEnv() {
  assertCommand("tmux", ["-V"], "tmux is required for foreign backend browser smoke.");
  let env = { ...process.env };
  let version = resolveZellijVersion(env);

  if (!isVersionAtLeast(version, zellijMinimum)) {
    if (process.env.TERMINAL_DEMO_FOREIGN_AUTO_INSTALL_ZELLIJ === "0") {
      throw new Error(`Zellij ${formatVersion(zellijMinimum)}+ is required; found ${version.raw}.`);
    }

    tempZellijBinDir = path.join(os.tmpdir(), `terminal-demo-zellij-${process.pid}-${Date.now()}`);
    const python = resolvePython();
    runSync(python, [
      path.join(repoRoot, ".github", "scripts", "install_zellij.py"),
      "--out",
      tempZellijBinDir,
    ], repoRoot, {
      ...env,
      SSL_CERT_FILE: process.env.SSL_CERT_FILE ?? "/etc/ssl/cert.pem",
    });
    env = {
      ...env,
      PATH: `${tempZellijBinDir}${path.delimiter}${env.PATH ?? ""}`,
      SSL_CERT_FILE: env.SSL_CERT_FILE ?? "/etc/ssl/cert.pem",
    };
    version = resolveZellijVersion(env);
  }

  if (!isVersionAtLeast(version, zellijMinimum)) {
    throw new Error(`Zellij ${formatVersion(zellijMinimum)}+ is required; found ${version.raw}.`);
  }

  process.stdout.write(`Foreign backend smoke tools - tmux ${runCapture("tmux", ["-V"], appRoot, env).trim()}, ${version.raw}\n`);
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

async function runForeignBackendScenario(browserUrl, expected) {
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

    await waitForBrowser(send, `state ready with discovered ${expected.tmuxSessionName} and ${expected.zellijSessionName}`, `(() => {
      const state = window.terminalDemoDebug?.getState?.();
      const discovered = state?.catalog?.discoveredSessions ?? {};
      const hasTmux = (discovered.tmux ?? []).some((session) => session.title === ${JSON.stringify(expected.tmuxSessionName)});
      const hasZellij = (discovered.zellij ?? []).some((session) => session.title === ${JSON.stringify(expected.zellijSessionName)});
      return state?.connection?.state === 'ready' && hasTmux && hasZellij;
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
        documentHorizontalOverflow: Math.max(0, document.documentElement.scrollWidth - document.documentElement.clientWidth),
      };
    })()`);

    const tmuxImport = await importBackendViaUi(send, "tmux", expected.tmuxSessionName, "tmux-ui-smoke-marker");
    const zellijImport = await importBackendViaUi(send, "zellij", expected.zellijSessionName, "zellij-ui-smoke-marker");

    return {
      beforeImport,
      imports: {
        tmux: tmuxImport,
        zellij: zellijImport,
      },
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
    descriptor?.set?.call(textarea, ${JSON.stringify(`printf "${marker}\\n"`)} );
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

  return {
    ...afterCommand,
    commandSent,
    importClicked,
    marker,
  };
}

async function startBrowserHost(rendererUrlValue, options) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error("Timed out waiting for TERMINAL_DEMO_BROWSER_URL"));
    }, 20_000);

    browserHostProcess = spawn("node", ["./dist/host/browser/index.js"], {
      cwd: appRoot,
      env: {
        ...smokeEnv,
        TERMINAL_DEMO_AUTO_START_SESSION: options.autoStartSession,
        TERMINAL_DEMO_RENDERER_URL: rendererUrlValue,
        TERMINAL_DEMO_BROWSER_BOOTSTRAP_SCOPE: "dist-only",
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
  if (chromeUserDataDir) {
    await fs.rm(chromeUserDataDir, { recursive: true, force: true });
  }
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
  const result = spawnSync(command, args, {
    cwd,
    env,
    stdio: "inherit",
  });

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
  if (result.status !== 0 && !options.allowFailure) {
    throw new Error(`${command} ${args.join(" ")} failed: ${result.stderr.trim()}`);
  }
  return result.stdout ?? "";
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
  const raw = runCapture("zellij", ["--version"], appRoot, env).trim();
  const parsed = raw.match(/(\d+)\.(\d+)\.(\d+)/u)?.slice(1).map(Number) ?? [0, 0, 0];
  return {
    raw,
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

function uniqueName(prefix) {
  return `${prefix}-${process.pid}-${Date.now().toString(16)}`;
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
