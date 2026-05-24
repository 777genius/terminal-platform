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
import { withTerminalDemoSmokeLock } from "./smoke-lock.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const viteCliPath = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
const rendererPort = process.env.TERMINAL_DEMO_LONG_HISTORY_RENDERER_PORT ?? "4279";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;
const cdpPort = process.env.TERMINAL_DEMO_LONG_HISTORY_CDP_PORT ?? "9231";
const runtimeSlug = process.env.TERMINAL_DEMO_LONG_HISTORY_RUNTIME_SLUG
  ?? `terminal-demo-long-history-${process.pid}-${Date.now().toString(16)}`;
const sessionStorePath = path.join(
  os.tmpdir(),
  `terminal-demo-long-history-store-${process.pid}-${Date.now()}.sqlite3`,
);
const browserBootstrapPath = path.join(appRoot, "dist", "renderer", "terminal-runtime-bootstrap.json");
const keepArtifacts = process.env.TERMINAL_DEMO_LONG_HISTORY_KEEP_ARTIFACTS === "1";
const historyLineCount = Number.parseInt(process.env.TERMINAL_DEMO_LONG_HISTORY_LINES ?? "1400", 10);
const historyIntervalMs = Number.parseInt(process.env.TERMINAL_DEMO_LONG_HISTORY_INTERVAL_MS ?? "1", 10);
const historyPayloadChars = Number.parseInt(process.env.TERMINAL_DEMO_LONG_HISTORY_PAYLOAD_CHARS ?? "900", 10);
const startMarker = "TPV2-LONG-HISTORY-START";
const endMarker = "TPV2-LONG-HISTORY-END";

let previewProcess = null;
let browserHostProcess = null;
let chromeProcess = null;
let chromeUserDataDir = null;

await withTerminalDemoSmokeLock("browser-persistence-long-history-smoke", main);

async function main() {
  try {
    runSync("npm", ["run", "build"], appRoot);

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
      env: process.env,
      stdio: "pipe",
      windowsHide: true,
    });
    pipeProcess(previewProcess, "[browser-long-history:preview]");
    await waitForHttpServer(rendererUrl, {
      child: previewProcess,
      label: "Renderer preview",
    });

    const chromeLaunch = await launchChromeWithCdp({
      appRoot,
      binaryMissingMessage: "Chrome binary not found. Set TERMINAL_DEMO_CHROME_BIN to run long-history smoke.",
      cdpPort,
      extraArgs: [
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--disable-software-rasterizer",
        "--no-first-run",
        "--no-default-browser-check",
        "--no-sandbox",
      ],
      headlessModeEnv: "TERMINAL_DEMO_LONG_HISTORY_HEADLESS_MODE",
      logPrefix: "browser-long-history:chrome",
      profilePrefix: "terminal-demo-long-history-profile",
    });
    chromeProcess = chromeLaunch.child;
    chromeUserDataDir = chromeLaunch.userDataDir;

    const browserUrl = await startBrowserHost(rendererUrl, {
      autoStartSession: "1",
      runtimeSlug,
      sessionStorePath,
    });
    const result = await runLongHistoryScenario(browserUrl);
    if (result.issues.length > 0) {
      throw new Error(`Long-history browser smoke reported runtime issues: ${JSON.stringify(result.issues)}`);
    }
    if (!result.seeded || !result.saved || !result.restored || !result.initialPageWasPaged || !result.loadedFullHistory) {
      throw new Error(`Long-history browser smoke did not prove full persistence: ${JSON.stringify(result)}`);
    }

    process.stdout.write(`Browser long-history persistence smoke passed - ${browserUrl}\n`);
    if (keepArtifacts) {
      process.stdout.write(`Browser long-history persistence store - ${sessionStorePath}\n`);
    }
  } finally {
    await shutdown();
  }
}

async function runLongHistoryScenario(browserUrl) {
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

    await waitForBrowserValue(send, "auto-started terminal session", `(() => {
      const state = window.terminalDemoDebug?.getState?.();
      const sessionId = state?.selection?.activeSessionId ?? state?.attachedSession?.session?.session_id ?? null;
      const paneId = state?.selection?.activePaneId ?? state?.attachedSession?.focused_screen?.pane_id ?? null;
      return {
        connectionReady: state?.connection?.state === 'ready',
        sessionId,
        paneId,
        hasScreen: Boolean(state?.attachedSession?.focused_screen),
      };
    })()`, (state) => Boolean(state?.connectionReady && state.sessionId && state.paneId && state.hasScreen), 35_000);

    const seed = await evaluate(send, `(async () => {
      const state = window.terminalDemoDebug?.getState?.();
      const sessionId = state?.selection?.activeSessionId ?? state?.attachedSession?.session?.session_id ?? null;
      const paneId = state?.selection?.activePaneId ?? state?.attachedSession?.focused_screen?.pane_id ?? null;
      const commands = window.terminalDemoDebug?.controller?.commands ?? null;
      if (!sessionId || !paneId || !commands?.dispatchMuxCommand) {
        return { ok: false, reason: 'active session or dispatch command missing', sessionId, paneId };
      }
      await commands.dispatchMuxCommand(sessionId, {
        kind: 'send_input',
        pane_id: paneId,
        data: ${JSON.stringify(buildLongHistoryCommand())} + '\\r',
        client_event_id: 'long-history-seed-' + Date.now(),
      });
      return { ok: true, sessionId, paneId };
    })()`);
    if (!seed.ok) {
      throw new Error(`Unable to seed long history output: ${JSON.stringify(seed)}`);
    }

    const afterSeed = await waitForBrowserValue(send, "long history end marker", `(() => {
      const state = window.terminalDemoDebug?.getState?.();
      const lines = state?.attachedSession?.focused_screen?.surface?.lines ?? [];
      const screenText = lines.map((line) => line.text ?? '').join('\\n');
      return {
        sessionId: state?.selection?.activeSessionId ?? state?.attachedSession?.session?.session_id ?? null,
        paneId: state?.selection?.activePaneId ?? state?.attachedSession?.focused_screen?.pane_id ?? null,
        containsEnd: screenText.includes(${JSON.stringify(endMarker)}),
        screenPreview: screenText.slice(-240),
      };
    })()`, (state) => Boolean(state?.containsEnd), Math.max(60_000, historyLineCount * historyIntervalMs + 30_000));

    await sleep(2_000);

    const saved = await evaluate(send, `(async () => {
      const waitForValue = async (predicate, timeoutMs = 35_000) => {
        const deadline = performance.now() + timeoutMs;
        let last = null;
        while (performance.now() < deadline) {
          last = await predicate();
          if (last) {
            return { ok: true, value: last };
          }
          await new Promise((resolve) => setTimeout(resolve, 250));
        }
        return { ok: false, last };
      };
      const stateBefore = window.terminalDemoDebug?.getState?.();
      const beforeSavedSessionCount = stateBefore?.catalog?.savedSessions?.length ?? 0;
      const workspaceHost = document.querySelector('tp-terminal-workspace') ?? null;
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const saveLayoutButton = commandRoot?.querySelector('[data-testid="tp-save-layout"]') ?? null;
      if (!workspaceHost || !saveLayoutButton) {
        return { ok: false, reason: workspaceHost ? 'save layout button missing' : 'workspace host missing' };
      }
      if (saveLayoutButton.disabled) {
        return { ok: false, reason: 'save layout button disabled', title: saveLayoutButton.getAttribute('title') };
      }

      let saveEventDetail = null;
      workspaceHost.addEventListener('tp-terminal-layout-saved', (event) => {
        saveEventDetail = event.detail ?? null;
      }, { once: true });
      saveLayoutButton.click();

      const savedWait = await waitForValue(async () => {
        const state = window.terminalDemoDebug?.getState?.();
        const savedSessions = state?.catalog?.savedSessions ?? [];
        const savedSessionId = saveEventDetail?.savedSessionId ?? savedSessions[0]?.session_id ?? null;
        const savedSession = savedSessionId
          ? savedSessions.find((session) => session.session_id === savedSessionId) ?? null
          : null;
        if (!savedSession || savedSessions.length <= beforeSavedSessionCount) {
          return null;
        }
        return {
          savedSessionId,
          savedSessionCount: savedSessions.length,
          restoreGuarantee: savedSession.restore_semantics_v2?.restore_guarantee_level ?? null,
          hasKnownGaps: savedSession.restore_semantics_v2?.has_known_gaps ?? null,
          compatibility: savedSession.compatibility?.status ?? null,
        };
      });
      if (!savedWait.ok) {
        return { ok: false, reason: 'saved session did not appear', beforeSavedSessionCount, saveEventDetail, savedWait };
      }
      return { ok: true, ...savedWait.value, beforeSavedSessionCount, saveEventDetail };
    })()`);
    if (!saved.ok) {
      throw new Error(`Unable to save long-history session: ${JSON.stringify(saved)}`);
    }

    await evaluate(send, `(async () => {
      await window.terminalDemoDebug?.controller?.commands?.restoreSavedSession?.(${JSON.stringify(saved.savedSessionId)});
      return true;
    })()`);

    const restoredInitial = await waitForBrowserValue(send, "restored paged v2 history", historySummaryExpression(), (state) => {
      return Boolean(
        state?.historySource === "v2_pane_history"
        && state.hasMoreSegments === true
        && state.nextEventSeq !== null
        && state.includesStart
        && state.domHasRestoreBoundary
      );
    }, 45_000);

    const loadResult = await evaluate(send, `(async () => {
      const waitForHistoryAdvance = async (previousNextEventSeq, timeoutMs = 20_000) => {
        const deadline = performance.now() + timeoutMs;
        while (performance.now() < deadline) {
          const summary = window.__terminalLongHistorySummary();
          if (!summary.hasMoreSegments || summary.nextEventSeq !== previousNextEventSeq) {
            return summary;
          }
          await new Promise((resolve) => setTimeout(resolve, 250));
        }
        return window.__terminalLongHistorySummary();
      };

      const commands = window.terminalDemoDebug?.controller?.commands ?? null;
      const loadedPages = [];
      let latest = window.__terminalLongHistorySummary();
      for (let index = 0; index < 20 && latest.hasMoreSegments; index += 1) {
        const previousNextEventSeq = latest.nextEventSeq;
        const loaded = await commands?.loadMorePaneHistory?.(latest.activePaneId);
        latest = await waitForHistoryAdvance(previousNextEventSeq);
        loadedPages.push({
          index,
          loaded: Boolean(loaded),
          previousNextEventSeq,
          nextEventSeq: latest.nextEventSeq,
          lineCount: latest.historyLineCount,
          segmentCount: latest.segmentCount,
          hasMoreSegments: latest.hasMoreSegments,
          includesEnd: latest.includesEnd,
          domIncludesEnd: latest.domIncludesEnd,
        });
        if (!loaded) {
          break;
        }
      }

      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      latest = window.__terminalLongHistorySummary();
      return {
        loadedPages,
        final: latest,
      };
    })()`);

    const final = loadResult.final ?? {};
    return {
      issues,
      seeded: afterSeed.containsEnd === true,
      saved: saved.ok === true,
      restored: restoredInitial.historySource === "v2_pane_history",
      initialPageWasPaged: restoredInitial.hasMoreSegments === true && restoredInitial.nextEventSeq !== null,
      loadedFullHistory:
        final.hasMoreSegments === false
        && final.includesStart === true
        && final.includesEnd === true
        && final.domIncludesStart === true
        && final.domIncludesEnd === true
        && final.domHasRestoreBoundary === true,
      seed,
      afterSeed,
      savedSession: saved,
      restoredInitial,
      loadedPages: loadResult.loadedPages,
      final,
    };
  } finally {
    await closeWebSocket(socket);
    await closePageTarget(target.id);
  }
}

function historySummaryExpression() {
  return `(() => {
    window.__terminalLongHistorySummary = () => {
      const state = window.terminalDemoDebug?.getState?.();
      const activeSessionId = state?.selection?.activeSessionId ?? state?.attachedSession?.session?.session_id ?? null;
      const activePaneId = state?.selection?.activePaneId ?? state?.attachedSession?.focused_screen?.pane_id ?? null;
      const history = activePaneId ? state?.historicalPanes?.[activePaneId] ?? null : null;
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const screenRoot = workspaceRoot?.querySelector('tp-terminal-screen')?.shadowRoot ?? null;
      const historyLineTexts = [...(screenRoot?.querySelectorAll('[data-line-source="history"] .text') ?? [])]
        .map((line) => line.textContent ?? '');
      const boundaryTexts = [...(screenRoot?.querySelectorAll('[data-line-source="boundary"] .text') ?? [])]
        .map((line) => line.textContent ?? '');
      const lines = history?.lines ?? [];
      return {
        activeSessionId,
        activePaneId,
        historySource: history?.source ?? null,
        replayStrategy: history?.replayStrategy ?? null,
        restoreGuaranteeLevel: history?.restoreGuaranteeLevel ?? null,
        historyLineCount: lines.length,
        segmentCount: history?.segmentCount ?? 0,
        hasMoreSegments: history?.hasMoreSegments ?? false,
        nextEventSeq: history?.nextEventSeq == null ? null : String(history.nextEventSeq),
        loadedPayloadBytes: history?.loadedPayloadBytes == null ? null : String(history.loadedPayloadBytes),
        includesStart: lines.some((line) => line.includes(${JSON.stringify(startMarker)})),
        includesEnd: lines.some((line) => line.includes(${JSON.stringify(endMarker)})),
        domIncludesStart: historyLineTexts.some((line) => line.includes(${JSON.stringify(startMarker)})),
        domIncludesEnd: historyLineTexts.some((line) => line.includes(${JSON.stringify(endMarker)})),
        domHasRestoreBoundary: boundaryTexts.some((line) => /restored history above/i.test(line)),
        domHistoryLineCount: historyLineTexts.length,
        diagnostics: (state?.diagnostics ?? []).map((diagnostic) => ({
          code: diagnostic.code ?? null,
          message: diagnostic.message ?? null,
          severity: diagnostic.severity ?? null,
        })),
      };
    };
    return window.__terminalLongHistorySummary();
  })()`;
}

function buildLongHistoryCommand() {
  const lineCount = Math.max(300, Number.isFinite(historyLineCount) ? historyLineCount : 1400);
  const intervalMs = Math.max(1, Number.isFinite(historyIntervalMs) ? historyIntervalMs : 1);
  const payloadChars = Math.max(64, Number.isFinite(historyPayloadChars) ? historyPayloadChars : 900);
  return [
    "node -e \"",
    `let i=0;console.log('${startMarker}');`,
    "const t=setInterval(()=>{",
    "i++;",
    `console.log('TPV2-LONG-HISTORY-LINE-'+String(i).padStart(4,'0')+' '+'.'.repeat(${payloadChars}));`,
    `if(i>=${lineCount}){console.log('${endMarker}');clearInterval(t)}`,
    `},${intervalMs})`,
    "\"",
  ].join("");
}

async function startBrowserHost(rendererUrlValue, options) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error("Timed out waiting for TERMINAL_DEMO_BROWSER_URL"));
    }, 20_000);

    browserHostProcess = spawn(process.execPath, ["./dist/host/browser/index.js"], {
      cwd: appRoot,
      env: {
        ...process.env,
        TERMINAL_DEMO_AUTO_START_SESSION: options.autoStartSession,
        TERMINAL_DEMO_RENDERER_URL: rendererUrlValue,
        TERMINAL_DEMO_BROWSER_BOOTSTRAP_SCOPE: "dist-only",
        TERMINAL_DEMO_RUNTIME_SLUG: options.runtimeSlug,
        TERMINAL_DEMO_SESSION_STORE_PATH: options.sessionStorePath,
      },
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
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
  await removeBrowserBootstrapConfig();
  await stopProcess(previewProcess);
  await stopProcess(chromeProcess);
  await removeChromeUserDataDir(chromeUserDataDir);
  if (keepArtifacts) {
    process.stderr.write(`[browser-long-history] keeping session store: ${sessionStorePath}\n`);
    return;
  }
  await removeSessionStore(sessionStorePath);
}

async function removeBrowserBootstrapConfig() {
  await fs.rm(browserBootstrapPath, {
    force: true,
    maxRetries: process.platform === "win32" ? 8 : 0,
    retryDelay: process.platform === "win32" ? 250 : 0,
  });
}

async function removeSessionStore(storePath) {
  await Promise.all([
    removeFileWithWindowsRetries(storePath, { recursive: true }),
    removeFileWithWindowsRetries(`${storePath}-shm`, { recursive: true }),
    removeFileWithWindowsRetries(`${storePath}-wal`, { recursive: true }),
  ]);
}

async function removeFileWithWindowsRetries(filePath, options = {}) {
  try {
    await fs.rm(filePath, {
      force: true,
      recursive: Boolean(options.recursive),
      maxRetries: process.platform === "win32" ? 8 : 0,
      retryDelay: process.platform === "win32" ? 250 : 0,
    });
  } catch (error) {
    if (process.platform === "win32" && ["EBUSY", "ENOTEMPTY", "EPERM"].includes(error?.code)) {
      process.stderr.write(`[browser-long-history] skipped locked temporary cleanup ${filePath}: ${error.message}\n`);
      return;
    }

    throw error;
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
      reject(new Error(`Timed out waiting for browser evaluation: ${formatEvaluationTimeoutSnippet(expression)}`));
    }, 75_000);
  });

  return Promise.race([evaluation, timeout]).finally(() => {
    clearTimeout(timeoutId);
  });
}

function formatEvaluationTimeoutSnippet(expression) {
  return String(expression)
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(0, 4)
    .join(" ")
    .slice(0, 240);
}

async function waitForBrowserValue(send, label, expression, predicate, timeoutMs = 20_000) {
  let latest = null;
  try {
    await waitFor(async () => {
      latest = await evaluate(send, expression);
      return predicate(latest);
    }, label, timeoutMs);
  } catch (error) {
    throw new Error(`${error.message}; latest=${JSON.stringify(latest)}`);
  }
  return latest;
}

async function waitFor(probe, label, timeoutMs = 20_000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (await probe()) {
      return;
    }
    await sleep(250);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function closePageTarget(targetId) {
  await fetch(`http://127.0.0.1:${cdpPort}/json/close/${targetId}`).catch(() => undefined);
}

function runSync(command, args, cwd) {
  const resolved = resolveSpawnCommand(command, args);
  const result = spawnSync(resolved.command, resolved.args, {
    cwd,
    env: process.env,
    shell: resolved.shell,
    stdio: "inherit",
    windowsHide: true,
  });

  if (result.error) {
    throw new Error(`${command} ${args.join(" ")} failed: ${result.error.message}`);
  }

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
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
    const timeout = setTimeout(settle, 1_000);
    socket.once("close", settle);
    socket.once("error", settle);
    socket.close();
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
