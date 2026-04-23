#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const rendererPort = process.env.TERMINAL_DEMO_SMOKE_RENDERER_PORT ?? "4273";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;
const cdpPort = process.env.TERMINAL_DEMO_SMOKE_CDP_PORT ?? "9226";
const chromeBinary = resolveChromeBinary();
const screenshotPath = path.join("/tmp", `terminal-demo-browser-smoke-${Date.now()}.png`);
const chromeUserDataDir = path.join("/tmp", `terminal-demo-browser-smoke-profile-${process.pid}`);

let previewProcess = null;
let browserHostProcess = null;
let chromeProcess = null;

await main();

async function main() {
  try {
    runSync("npm", ["run", "build"], appRoot);

    previewProcess = spawn("npx", ["vite", "preview", "--host", "127.0.0.1", "--port", rendererPort, "--strictPort"], {
      cwd: appRoot,
      env: process.env,
      stdio: "inherit",
    });
    await waitForServer(rendererUrl);

    const browserUrl = await startBrowserHost(rendererUrl);

    chromeProcess = spawn(chromeBinary, [
      "--headless=new",
      `--remote-debugging-port=${cdpPort}`,
      `--user-data-dir=${chromeUserDataDir}`,
      "about:blank",
    ], {
      cwd: appRoot,
      env: process.env,
      stdio: "pipe",
    });
    pipeProcess(chromeProcess, "[browser-smoke:chrome]");
    await waitForServer(`http://127.0.0.1:${cdpPort}/json/version`);

    const result = await runSmokeScenario(browserUrl);

    if (result.issues.length > 0) {
      throw new Error(`Browser reported runtime issues: ${JSON.stringify(result.issues)}`);
    }

    if (
      !result.afterCreate.hasReady
      || result.afterCreate.hasError
      || result.afterCreate.healthPhase !== "ready"
      || !result.afterCreate.hasStatusBar
      || !result.afterCreate.hasCommandDock
      || result.afterCreate.savedItemsRendered > 8
      || (result.afterCreate.savedSessionCount > 8 && !result.afterCreate.hasSavedPagination)
      || !result.afterCreate.hasActiveTitle
      || !result.afterCreate.inputEnabled
    ) {
      throw new Error(`Session creation did not settle correctly: ${JSON.stringify(result.afterCreate)}`);
    }

    if (result.afterCreate.savedSessionCount > result.afterCreate.savedItemsRendered) {
      const expectedPaginatedItems = Math.min(
        result.afterCreate.savedSessionCount,
        result.afterCreate.savedItemsRendered + 8,
      );
      if (
        !result.afterSavedPagination.clicked
        || result.afterSavedPagination.savedItemsRendered !== expectedPaginatedItems
        || !result.afterSavedPagination.hasCollapse
      ) {
        throw new Error(`Saved-session pagination did not expand correctly: ${JSON.stringify({
          expectedPaginatedItems,
          afterSavedPagination: result.afterSavedPagination,
        })}`);
      }
    }

    if (
      !result.afterCommand.connectionReady
      || (!result.afterCommand.sequenceAdvanced && !result.afterCommand.containsCommandOutput)
    ) {
      throw new Error(`Command lane did not advance the focused screen: ${JSON.stringify(result.afterCommand)}`);
    }

    if (
      !result.afterHistoryReplay.recalledDraft?.includes("browser-smoke-ok")
      || !result.afterHistoryReplay.replayClicked
      || !result.afterHistoryReplay.connectionReady
      || (!result.afterHistoryReplay.sequenceAdvanced && !result.afterHistoryReplay.containsCommandOutput)
    ) {
      throw new Error(`Command history replay did not settle correctly: ${JSON.stringify(result.afterHistoryReplay)}`);
    }

    process.stdout.write(`Browser smoke passed - ${browserUrl}\n`);
    process.stdout.write(`Browser smoke screenshot - ${screenshotPath}\n`);
  } finally {
    await shutdown();
  }
}

async function runSmokeScenario(browserUrl) {
  const target = await fetch(`http://127.0.0.1:${cdpPort}/json/new?${encodeURIComponent(browserUrl)}`, {
    method: "PUT",
  }).then((response) => response.json());
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await onceSocketOpen(socket);

  let id = 0;
  const pending = new Map();
  const issues = [];

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data.toString());
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
    await send("Runtime.enable");
    await send("Log.enable");
    await send("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 1100,
      deviceScaleFactor: 1,
      mobile: false,
    });

    await sleep(3000);

    const before = await evaluate(send, `(() => ({
      bodyText: document.body.innerText,
      hasWorkspaceShell: Boolean(document.querySelector('[data-testid="terminal-demo-shell"]')),
      buttons: [...document.querySelectorAll('button')].map((button) => button.textContent?.trim()).filter(Boolean),
    }))()`);

    const createButtonResult = await evaluate(send, `(() => {
      const button = document.querySelector('[data-testid="start-default-shell"]');
      if (!button) {
        return { clicked: false };
      }
      button.click();
      return { clicked: true };
    })()`);
    if (!createButtonResult.clicked) {
      throw new Error("Start default shell button was not found");
    }

    await sleep(2500);

    const afterCreate = await evaluate(send, `(() => {
      const debug = window.terminalDemoDebug?.getState?.();
      const workspaceHost = document.querySelector('tp-terminal-workspace');
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const statusRoot = workspaceRoot?.querySelector('tp-terminal-status-bar')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const savedRoot = workspaceRoot?.querySelector('tp-terminal-saved-sessions')?.shadowRoot ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const screenRoot = screenHost?.shadowRoot ?? null;
      const terminalScreenText = debug?.attachedSession?.focused_screen?.surface?.lines
        ? debug.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : (screenRoot?.querySelector('[part=\"screen-lines\"]')?.textContent?.trim() ?? null);
      const activeTitle = document.querySelector('.workspace-summary__title')?.textContent?.trim() ?? null;
      const input = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      return {
        hasReady: debug?.connection?.state === 'ready',
        hasError: debug?.connection?.state === 'error',
        activeSessionId: debug?.selection?.activeSessionId ?? null,
        savedSessionCount: debug?.catalog?.savedSessions?.length ?? 0,
        savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        hasSavedPagination: Boolean(savedRoot?.querySelector('[part="show-more"]')),
        healthPhase: debug?.attachedSession?.health?.phase ?? null,
        focusedSequence: debug?.attachedSession?.focused_screen?.sequence != null
          ? String(debug.attachedSession.focused_screen.sequence)
          : null,
        hasScreen: Boolean(terminalScreenText),
        hasStatusBar: Boolean(statusRoot?.querySelector('[part="status-bar"]')),
        hasCommandDock: Boolean(commandRoot?.querySelector('[part="command-dock"]')),
        hasActiveTitle: Boolean(activeTitle && activeTitle !== 'Pick a session to inspect'),
        inputEnabled: Boolean(input && !input.disabled),
      };
    })()`);

    const initialSequence = afterCreate.focusedSequence;
    const afterSavedPagination = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const savedRoot = workspaceRoot?.querySelector('tp-terminal-saved-sessions')?.shadowRoot ?? null;
      const showMoreButton = savedRoot?.querySelector('[part="show-more"]') ?? null;
      if (!showMoreButton) {
        return {
          clicked: false,
          reason: 'show-more missing',
          savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
          hasCollapse: Boolean(savedRoot?.querySelector('[part="collapse"]')),
          summaryText: savedRoot?.querySelector('[part="list-summary"]')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        };
      }
      showMoreButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      return {
        clicked: true,
        savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        hasCollapse: Boolean(savedRoot?.querySelector('[part="collapse"]')),
        summaryText: savedRoot?.querySelector('[part="list-summary"]')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
      };
    })()`);

    const sendCommandResult = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      if (!textarea) {
        return { ok: false, reason: 'textarea missing' };
      }
      const descriptor = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value');
      descriptor?.set?.call(textarea, 'printf \"browser-smoke-ok\\\\n\"');
      textarea.dispatchEvent(new Event('input', { bubbles: true }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const button = commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null;
      if (!button) {
        return { ok: false, reason: 'send button missing' };
      }
      if (button.disabled) {
        return { ok: false, reason: 'send button disabled after input' };
      }
      button.click();
      return { ok: true };
    })()`);
    if (!sendCommandResult.ok) {
      throw new Error(`Unable to send command through command dock: ${JSON.stringify(sendCommandResult)}`);
    }

    await sleep(2000);

    const afterCommand = await evaluate(send, `(() => {
      const debug = window.terminalDemoDebug?.getState?.();
      const workspaceHost = document.querySelector('tp-terminal-workspace');
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const screenRoot = screenHost?.shadowRoot ?? null;
      const terminalScreenText = debug?.attachedSession?.focused_screen?.surface?.lines
        ? debug.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : (screenRoot?.querySelector('[part=\"screen-lines\"]')?.textContent?.trim() ?? '');
      return {
        connectionReady: debug?.connection?.state === 'ready',
        focusedSequence: debug?.attachedSession?.focused_screen?.sequence != null
          ? String(debug.attachedSession.focused_screen.sequence)
          : null,
        terminalScreenText,
        containsCommandOutput: /browser-smoke-ok/i.test(terminalScreenText),
      };
    })()`);
    const replayInitialSequence = afterCommand.focusedSequence;

    const historyReplayResult = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      if (!textarea) {
        return { ok: false, reason: 'textarea missing', recalledDraft: null };
      }
      textarea.focus();
      textarea.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'ArrowUp',
        bubbles: true,
        cancelable: true,
      }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const recalledDraft = textarea.value;
      const button = commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null;
      if (!button) {
        return { ok: false, reason: 'send button missing', recalledDraft };
      }
      if (button.disabled) {
        return { ok: false, reason: 'send button disabled after history recall', recalledDraft };
      }
      button.click();
      return { ok: true, recalledDraft };
    })()`);
    if (!historyReplayResult.ok) {
      throw new Error(`Unable to replay command through command history: ${JSON.stringify(historyReplayResult)}`);
    }

    await sleep(2000);

    const afterHistoryReplay = await evaluate(send, `(() => {
      const debug = window.terminalDemoDebug?.getState?.();
      const terminalScreenText = debug?.attachedSession?.focused_screen?.surface?.lines
        ? debug.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : '';
      return {
        replayClicked: true,
        recalledDraft: ${JSON.stringify(historyReplayResult.recalledDraft)},
        connectionReady: debug?.connection?.state === 'ready',
        focusedSequence: debug?.attachedSession?.focused_screen?.sequence != null
          ? String(debug.attachedSession.focused_screen.sequence)
          : null,
        terminalScreenText,
        containsCommandOutput: /browser-smoke-ok/i.test(terminalScreenText),
      };
    })()`);

    const screenshot = await send("Page.captureScreenshot", {
      format: "png",
      captureBeyondViewport: true,
    });
    await fs.writeFile(screenshotPath, Buffer.from(screenshot.data, "base64"));

    return {
      before,
      afterCreate,
      afterSavedPagination,
      afterCommand: {
        ...afterCommand,
        sequenceAdvanced: initialSequence !== null
          ? afterCommand.focusedSequence !== initialSequence
          : false,
      },
      afterHistoryReplay: {
        ...afterHistoryReplay,
        sequenceAdvanced: replayInitialSequence !== null
          ? afterHistoryReplay.focusedSequence !== replayInitialSequence
          : false,
      },
      issues,
    };
  } finally {
    socket.close();
  }
}

async function startBrowserHost(rendererUrlValue) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error("Timed out waiting for TERMINAL_DEMO_BROWSER_URL"));
    }, 20_000);

    browserHostProcess = spawn("node", ["./dist/host/browser/index.js"], {
      cwd: appRoot,
      env: {
        ...process.env,
        TERMINAL_DEMO_RENDERER_URL: rendererUrlValue,
        TERMINAL_DEMO_BROWSER_BOOTSTRAP_SCOPE: "dist-only",
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

async function shutdown() {
  await stopProcess(browserHostProcess);
  await stopProcess(previewProcess);
  await stopProcess(chromeProcess);
  await fs.rm(chromeUserDataDir, { recursive: true, force: true });
}

async function stopProcess(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) {
    return;
  }

  const exited = new Promise((resolve) => {
    child.once("exit", () => resolve());
  });
  child.kill("SIGTERM");
  await Promise.race([exited, sleep(5_000)]);
  if (child.exitCode === null && child.signalCode === null) {
    child.kill("SIGKILL");
  }
}

function evaluate(send, expression) {
  return send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  }).then((result) => result.result.value);
}

function pipeProcess(child, prefix) {
  child.stdout?.on("data", (chunk) => {
    process.stdout.write(`${prefix} ${chunk}`);
  });
  child.stderr?.on("data", (chunk) => {
    process.stderr.write(`${prefix} ${chunk}`);
  });
}

function runSync(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

async function waitForServer(url) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < 20_000) {
    try {
      const response = await fetch(url, { method: "GET" });
      if (response.ok) {
        return;
      }
    } catch {
      // Server is still starting.
    }

    await sleep(200);
  }

  throw new Error(`Timed out waiting for ${url}`);
}

function onceSocketOpen(socket) {
  return new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
}

function resolveChromeBinary() {
  const candidates = [
    process.env.TERMINAL_DEMO_CHROME_BIN,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    resolveBinaryFromShell("google-chrome"),
    resolveBinaryFromShell("chromium"),
    resolveBinaryFromShell("chromium-browser"),
  ].filter(Boolean);

  const binary = candidates[0];
  if (!binary) {
    throw new Error("Chrome binary not found. Set TERMINAL_DEMO_CHROME_BIN to run browser smoke.");
  }

  return binary;
}

function resolveBinaryFromShell(name) {
  const result = spawnSync("bash", ["-lc", `command -v ${name}`], {
    cwd: appRoot,
    env: process.env,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    return null;
  }

  const value = result.stdout.trim();
  return value.length > 0 ? value : null;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
