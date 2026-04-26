#!/usr/bin/env node
import fs from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import WebSocket from "ws";

import { launchChromeWithCdp, resolveRuntimeEvaluationValue, stopProcess } from "./chrome-cdp-smoke.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const rendererDistRoot = path.join(appRoot, "dist", "renderer");
const rendererIndexPath = path.join(rendererDistRoot, "index.html");
const cdpPort = process.env.TERMINAL_DEMO_STATIC_SMOKE_CDP_PORT ?? "9236";
const staticRendererPort = process.env.TERMINAL_DEMO_STATIC_SMOKE_RENDERER_PORT ?? "0";
const screenshotPath = path.join("/tmp", `terminal-demo-static-renderer-${Date.now()}.png`);

let chromeProcess = null;
let chromeUserDataDir = null;
let staticRendererServer = null;

await main();

async function main() {
  try {
    await fs.access(rendererIndexPath);
    staticRendererServer = await startStaticRendererServer();
    const staticPreviewUrl = `${staticRendererServer.origin}/index.html?demoStaticWorkspace=1`;

    const chromeLaunch = await launchChromeWithCdp({
      appRoot,
      binaryMissingMessage:
        "Chrome binary not found. Set TERMINAL_DEMO_CHROME_BIN to run static renderer browser smoke.",
      cdpPort,
      extraArgs: [
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--disable-software-rasterizer",
        "--no-first-run",
        "--no-default-browser-check",
        "--no-sandbox",
      ],
      headlessModeEnv: "TERMINAL_DEMO_STATIC_SMOKE_HEADLESS_MODE",
      logPrefix: "static-browser:chrome",
      profilePrefix: "terminal-demo-static-browser-profile",
    });
    chromeProcess = chromeLaunch.child;
    chromeUserDataDir = chromeLaunch.userDataDir;

    const result = await runStaticPreviewScenario(staticPreviewUrl);
    if (result.issues.length > 0) {
      throw new Error(`Static preview browser reported runtime issues: ${JSON.stringify(result.issues)}`);
    }

    if (
      !result.hasReadyState
      || !result.hasWorkspace
      || !result.hasTerminalColumn
      || !result.hasCommandDock
      || !result.hasTerminalScreen
      || result.commandInputRows !== 1
      || result.commandInputPlaceholder !== "Type shell input for the focused pane"
      || result.commandInputStatus !== "Ready"
      || result.commandDockPlacement !== "terminal"
      || result.commandDockCanWrite !== "true"
      || result.commandDockInputCapability !== "known"
      || !result.runEnabledBeforeSubmit
      || result.runEnabledAfterSubmit
      || !result.pasteEnabled
      || !result.interruptEnabled
      || !result.enterEnabled
      || !result.hasSubmittedCommand
      || !result.hasAcceptedPreviewLine
      || result.hasCommandFailure
      || result.documentHorizontalOverflow > 1
      || result.workspaceHostHeaderDisplay !== "none"
      || result.workspaceHostTopOffset > 20
      || result.terminalColumnHeight < 560
      || result.screenViewportHeight < 340
      || result.workspacePanelShadow !== "none"
      || Math.abs(result.terminalComposerGapPx ?? 99) > 1
      || result.commandHistoryLatest !== "printf \"static-browser-ok\\n\""
    ) {
      throw new Error(`Static preview browser contract failed: ${JSON.stringify(result, null, 2)}`);
    }

    process.stdout.write(`Static renderer browser smoke passed - ${staticPreviewUrl}\n`);
    if (result.screenshotPath) {
      process.stdout.write(`Static renderer screenshot: ${result.screenshotPath}\n`);
    }
  } finally {
    await stopProcess(chromeProcess);
    await closeStaticRendererServer(staticRendererServer?.server);
    if (chromeUserDataDir) {
      await fs.rm(chromeUserDataDir, { recursive: true, force: true });
    }
  }
}

function startStaticRendererServer() {
  const server = createServer(async (request, response) => {
    try {
      const requestPath = resolveStaticRendererRequestPath(request.url ?? "/");
      if (!requestPath) {
        response.writeHead(403, { "content-type": "text/plain; charset=utf-8" });
        response.end("Forbidden");
        return;
      }

      const stat = await fs.stat(requestPath);
      if (!stat.isFile()) {
        response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
        response.end("Not found");
        return;
      }

      response.writeHead(200, {
        "cache-control": "no-store",
        "content-type": resolveContentType(requestPath),
      });
      response.end(await fs.readFile(requestPath));
    } catch (error) {
      if (error?.code === "ENOENT") {
        response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
        response.end("Not found");
        return;
      }

      response.writeHead(500, { "content-type": "text/plain; charset=utf-8" });
      response.end("Static renderer server failed");
    }
  });

  return new Promise((resolve, reject) => {
    const settleReject = (error) => {
      server.off("listening", settleResolve);
      reject(error);
    };
    const settleResolve = () => {
      server.off("error", settleReject);
      const address = server.address();
      if (!address || typeof address === "string") {
        reject(new Error("Static renderer server did not expose a TCP address"));
        return;
      }
      resolve({
        origin: `http://127.0.0.1:${address.port}`,
        server,
      });
    };

    server.once("error", settleReject);
    server.once("listening", settleResolve);
    server.listen(Number(staticRendererPort), "127.0.0.1");
  });
}

function resolveStaticRendererRequestPath(rawUrl) {
  const url = new URL(rawUrl, "http://127.0.0.1");
  const pathname = url.pathname === "/" ? "/index.html" : url.pathname;
  const decodedPathname = decodeURIComponent(pathname);
  const requestedPath = path.resolve(rendererDistRoot, `.${decodedPathname}`);
  if (requestedPath !== rendererDistRoot && !requestedPath.startsWith(`${rendererDistRoot}${path.sep}`)) {
    return null;
  }
  return requestedPath;
}

function resolveContentType(filePath) {
  switch (path.extname(filePath)) {
    case ".css":
      return "text/css; charset=utf-8";
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
      return "text/javascript; charset=utf-8";
    case ".json":
      return "application/json; charset=utf-8";
    case ".svg":
      return "image/svg+xml";
    default:
      return "application/octet-stream";
  }
}

function closeStaticRendererServer(server) {
  if (!server?.listening) {
    return Promise.resolve();
  }

  return new Promise((resolve) => {
    server.close(() => resolve());
  });
}

async function runStaticPreviewScenario(staticPreviewUrl) {
  const target = await fetch(`http://127.0.0.1:${cdpPort}/json/new?${encodeURIComponent(staticPreviewUrl)}`, {
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
    await send("Runtime.enable");
    await send("Log.enable");
    await send("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 1000,
      deviceScaleFactor: 1,
      mobile: false,
    });

    const result = await evaluate(send, `(async () => {
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const waitFor = async (predicate) => {
        for (let attempt = 0; attempt < 40; attempt += 1) {
          const value = predicate();
          if (value) {
            return value;
          }
          await wait(100);
        }
        return predicate();
      };

      await waitFor(() => {
        const state = window.terminalDemoDebug?.getState?.();
        return state?.connection?.state === 'ready' && state?.attachedSession?.focused_screen;
      });

      const workspace = document.querySelector('tp-terminal-workspace');
      const demoMain = document.querySelector('.shell__main') ?? null;
      const workspaceHostSlot = document.querySelector('[data-testid="terminal-workspace-host"]') ?? null;
      const workspaceHostHeader = document.querySelector('.panel__header--workspace') ?? null;
      const workspaceRoot = workspace?.shadowRoot ?? null;
      const terminalColumn = workspaceRoot?.querySelector('[data-testid="tp-workspace-terminal-column"]') ?? null;
      const commandDockElement = workspaceRoot?.querySelector('tp-terminal-command-dock') ?? null;
      const screenElement = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const workspaceFrame = workspaceRoot?.querySelector('[part="workspace"]') ?? null;
      const commandRoot = commandDockElement?.shadowRoot ?? null;
      const screenRoot = screenElement?.shadowRoot ?? null;
      const commandDockPanel = commandRoot?.querySelector('[data-testid="tp-command-dock"]') ?? null;
      const commandInputStatus = commandRoot?.querySelector('[data-testid="tp-command-input-status"]') ?? null;
      const composer = commandRoot?.querySelector('tp-terminal-command-composer') ?? null;
      const input = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const run = commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null;
      const paste = commandRoot?.querySelector('[data-testid="tp-paste-clipboard"]') ?? null;
      const interrupt = commandRoot?.querySelector('[data-testid="tp-send-interrupt"]') ?? null;
      const enter = commandRoot?.querySelector('[data-testid="tp-send-enter"]') ?? null;
      const viewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      const runEnabledBeforeSubmit = Boolean(run && !run.disabled);

      if (input && run) {
        const descriptor = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value');
        descriptor?.set?.call(input, 'printf "static-browser-ok\\\\n"');
        input.dispatchEvent(new InputEvent('input', {
          bubbles: true,
          composed: true,
          data: 'printf "static-browser-ok\\\\n"',
          inputType: 'insertText',
        }));
        await waitFor(() => window.terminalDemoDebug?.getState?.()?.drafts?.['preview-pane-main']?.includes('static-browser-ok'));
        run.click();
        await waitFor(() => {
          const state = window.terminalDemoDebug?.getState?.();
          const lines = state?.attachedSession?.focused_screen?.surface?.lines ?? [];
          const status = commandRoot?.querySelector('[data-testid="tp-command-input-status"]')?.textContent?.trim();
          const dockPanel = commandRoot?.querySelector('[data-testid="tp-command-dock"]') ?? null;
          return lines.some((line) => /static-browser-ok/.test(line.text))
            && lines.some((line) => /preview runtime accepted input without native host/.test(line.text))
            && state?.commandHistory?.entries?.at?.(-1) === 'printf "static-browser-ok\\\\n"'
            && status === 'Ready'
            && dockPanel?.getAttribute('data-command-input') === 'true';
        });
      }

      const state = window.terminalDemoDebug?.getState?.();
      const terminalText = state?.attachedSession?.focused_screen?.surface?.lines
        ? state.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n')
        : '';
      const terminalColumnRect = terminalColumn?.getBoundingClientRect();
      const viewportRect = viewport?.getBoundingClientRect();
      const composerRect = composer?.getBoundingClientRect();

      return {
        hasReadyState: state?.connection?.state === 'ready',
        hasWorkspace: Boolean(workspace),
        hasTerminalColumn: Boolean(terminalColumn),
        hasCommandDock: Boolean(commandDockPanel),
        hasTerminalScreen: Boolean(screenRoot),
        commandInputRows: input?.rows ?? null,
        commandInputPlaceholder: input?.placeholder ?? null,
        commandInputStatus: commandInputStatus?.textContent?.trim() ?? null,
        commandDockPlacement: commandDockPanel?.getAttribute('data-placement') ?? null,
        commandDockCanWrite: commandDockPanel?.getAttribute('data-command-input') ?? null,
        commandDockInputCapability: commandDockPanel?.getAttribute('data-input-capability') ?? null,
        runEnabledBeforeSubmit,
        runEnabledAfterSubmit: Boolean(run && !run.disabled),
        pasteEnabled: Boolean(paste && !paste.disabled),
        interruptEnabled: Boolean(interrupt && !interrupt.disabled),
        enterEnabled: Boolean(enter && !enter.disabled),
        hasSubmittedCommand: /static-browser-ok/.test(terminalText),
        hasAcceptedPreviewLine: /preview runtime accepted input without native host/.test(terminalText),
        hasCommandFailure: Boolean(commandRoot?.textContent?.includes('Command failed')),
        commandHistoryLatest: state?.commandHistory?.entries?.at?.(-1) ?? null,
        documentHorizontalOverflow: Math.max(0, document.documentElement.scrollWidth - document.documentElement.clientWidth),
        workspaceHostTopOffset: demoMain && workspaceHostSlot
          ? Math.round(workspaceHostSlot.getBoundingClientRect().top - demoMain.getBoundingClientRect().top)
          : null,
        workspaceHostHeaderDisplay: workspaceHostHeader ? getComputedStyle(workspaceHostHeader).display : null,
        terminalColumnHeight: Math.round(terminalColumnRect?.height ?? 0),
        screenViewportHeight: Math.round(viewportRect?.height ?? 0),
        workspacePanelShadow: workspaceFrame
          ? getComputedStyle(workspaceFrame).getPropertyValue('--tp-shadow-panel').trim()
          : null,
        terminalComposerGapPx: viewportRect && composerRect ? composerRect.top - viewportRect.bottom : null,
      };
    })()`);

    let screenshotSaved = false;
    try {
      const screenshot = await send("Page.captureScreenshot", { format: "png", fromSurface: true });
      await fs.writeFile(screenshotPath, Buffer.from(screenshot.data, "base64"));
      screenshotSaved = true;
    } catch {
      screenshotSaved = false;
    }

    return {
      ...result,
      issues,
      screenshotPath: screenshotSaved ? screenshotPath : null,
    };
  } finally {
    await closeWebSocket(socket);
    await closePageTarget(target.id);
  }
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

function onceSocketOpen(socket) {
  return new Promise((resolve, reject) => {
    socket.once("open", resolve);
    socket.once("error", reject);
  });
}

function closeWebSocket(socket) {
  if (!socket || socket.readyState === WebSocket.CLOSED) {
    return Promise.resolve();
  }

  return new Promise((resolve) => {
    let settled = false;
    const settle = () => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timeout);
      socket.off("close", settle);
      socket.off("error", settle);
      resolve();
    };
    const timeout = setTimeout(() => {
      if (socket.readyState !== WebSocket.CLOSED) {
        socket.terminate();
      }
      settle();
    }, 1_000);

    socket.once("close", settle);
    socket.once("error", settle);

    try {
      socket.close();
    } catch {
      settle();
    }
  });
}

async function closePageTarget(targetId) {
  await fetch(`http://127.0.0.1:${cdpPort}/json/close/${targetId}`).catch(() => undefined);
}
