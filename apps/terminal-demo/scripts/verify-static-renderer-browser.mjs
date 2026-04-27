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
      || result.demoShellCanvas !== "terminal"
      || result.demoShellPaddingPx !== 0
      || !result.hasTerminalColumn
      || !result.hasCommandDock
      || !result.hasTerminalScreen
      || result.commandInputRows !== 1
      || result.commandInputPlaceholder !== "Type shell input for the focused pane"
      || result.commandInputAutocomplete !== "off"
      || result.commandInputAutocapitalize !== "off"
      || result.commandInputAutocorrect !== "off"
      || result.commandInputEnterKeyHint !== "send"
      || result.commandInputSpellcheck !== "false"
      || result.commandInputStatus !== "Ready"
      || result.commandInputDescribedBy !== "tp-command-input-status"
      || !result.commandInputDescribedByResolves
      || result.commandInputStatusLive !== "polite"
      || result.commandInputStatusAtomic !== "true"
      || !/^\d+ cmd$/.test(result.commandHistoryBadgeText ?? "")
      || result.commandActionLabels.join("|") !== "\u25b6|\u2398|^C|\u21b5"
      || result.terminalComposerActionPlacements.join("|") !== "terminal|terminal|terminal|terminal"
      || result.terminalComposerActionTones.join("|") !== "primary|secondary|secondary|secondary"
      || result.terminalComposerActionLabelModes.join("|") !== "glyph|glyph|glyph|glyph"
      || result.terminalComposerActionDisabledFlags.join("|") !== "true|false|false|false"
      || !result.terminalComposerPrimaryToneStyle
      || !result.terminalComposerSecondaryToneStyle
      || result.terminalComposerPrimaryToneStyle.opacity >= 0.7
      || result.terminalComposerPrimaryToneStyle.cursor !== "not-allowed"
      || result.terminalComposerPrimaryToneStyle.borderColor === result.terminalComposerSecondaryToneStyle.borderColor
      || result.terminalComposerPrimaryToneStyle.backgroundColor
        === result.terminalComposerSecondaryToneStyle.backgroundColor
      || result.terminalScreenActionTones.join("|") !== "primary|secondary|secondary"
      || !result.terminalScreenPrimaryToneStyle
      || !result.terminalScreenSecondaryToneStyle
      || result.terminalScreenPrimaryToneStyle.borderColor === result.terminalScreenSecondaryToneStyle.borderColor
      || result.terminalScreenPrimaryToneStyle.backgroundColor === result.terminalScreenSecondaryToneStyle.backgroundColor
      || result.commandDockPlacement !== "terminal"
      || result.commandDockAccessoryMode !== "bar"
      || result.commandAccessoryBarMode !== "bar"
      || result.commandAccessoryBarHasHistory !== "true"
      || result.commandAccessoryBarQuickCommandCount !== "5"
      || result.commandAccessoryBarRecentCommandCount !== "2"
      || !result.hasCommandAccessoryBar
      || result.terminalCommandAccessoryBarHeight > 72
      || result.quickCommandIds.join("|") !== "pwd|list-files|git-status|node-version|hello"
      || result.quickCommandTones.join("|") !== "secondary|secondary|secondary|primary|secondary"
      || result.quickCommandAriaLabels.join("|") !== "Show the current working directory|List files with metadata|Inspect the current git worktree|Insert node version command|Print a Terminal Platform greeting"
      || result.quickCommandWhiteSpaces.some((value) => value !== "nowrap")
      || Math.max(0, ...result.quickCommandHeights) > 38
      || result.quickCommandRowOverflowPx > 1
      || result.historyChipWhiteSpaces.some((value) => value !== "nowrap")
      || Math.max(0, ...result.historyChipHeights) > 38
      || result.historyChipCount > 2
      || result.historyChipIds.join("|") !== "history-4|history-3"
      || result.historyChipHistoryIndexes.join("|") !== "3|2"
      || !result.historyChipAriaLabels[0]?.includes("static-browser-ok")
      || result.commandDockCanWrite !== "true"
      || result.commandDockInputCapability !== "known"
      || result.screenChromeMode !== "compact"
      || !result.hasCompactScreenChrome
      || result.terminalScreenChromeHeight > 58
      || Math.abs(result.terminalScreenChromeViewportGapPx ?? 99) > 1
      || result.terminalScreenCompactSizeLabel !== "96x24"
      || result.terminalScreenActionIds.join("|") !== "follow-output|scroll-latest|copy-visible"
      || result.terminalScreenActionLabelModes.join("|") !== "glyph|glyph|glyph"
      || result.terminalScreenActionPlacements.join("|") !== "terminal|terminal|terminal"
      || result.terminalScreenActionLabels.join("|") !== "\u23f8|\u2193|\u2398"
      || result.terminalScreenActionAriaLabels.join("|") !== "Pause automatic terminal output follow|Scroll to latest terminal output|Copy visible terminal output"
      || result.terminalScreenActionTitles.join("|") !== "Pause automatic terminal output follow|Scroll to latest terminal output|Copy visible terminal output"
      || result.terminalScreenActionPressedFlags.join("|") !== "true||"
      || result.terminalSearchActionIds.join("|") !== "previous-match|next-match|clear-search"
      || result.terminalSearchActionLabelModes.join("|") !== "glyph|glyph|glyph"
      || result.terminalSearchActionPlacements.join("|") !== "terminal|terminal|terminal"
      || result.terminalSearchActionLabels.join("|") !== "\u2191|\u2193|\u00d7"
      || result.terminalSearchActionAriaLabels.join("|") !== "Select previous search match|Select next search match|Clear search query"
      || !result.terminalSearchActionsInsideChrome
      || result.terminalSearchInputType !== "search"
      || result.terminalSearchInputAutocomplete !== "off"
      || result.terminalSearchInputAutocapitalize !== "off"
      || result.terminalSearchInputAutocorrect !== "off"
      || result.terminalSearchInputEnterKeyHint !== "search"
      || result.terminalSearchInputInputMode !== "search"
      || result.terminalSearchInputSpellcheck !== "false"
      || result.terminalSearchInputDescribedBy !== "tp-screen-search-count"
      || !result.terminalSearchInputDescribedByResolves
      || result.terminalSearchCountLive !== "polite"
      || result.terminalSearchCountAtomic !== "true"
      || result.terminalSearchActiveHighlightText !== "static-browser-ok"
      || result.terminalSearchHighlightTexts.some((text) => text !== "static-browser-ok")
      || result.workspaceLayoutPreset !== "terminal"
      || result.workspaceNavigationMode !== "collapsed"
      || result.workspaceInspectorMode !== "collapsed"
      || result.workspaceChromeTone !== "terminal"
      || result.workspaceSecondaryChrome !== "terminal"
      || result.workspaceSecondaryDensity !== "compact"
      || result.inspectorDrawerSecondaryChrome !== "terminal"
      || result.inspectorDrawerSecondaryDensity !== "compact"
      || result.inspectorDrawerSummaryHeight < 24
      || result.inspectorDrawerSummaryHeight > 34
      || result.inspectorDrawerSummaryLabel !== "Tools"
      || result.inspectorDrawerClosedSummaryAction !== "Open"
      || result.navigationDrawerSecondaryChrome !== "terminal"
      || result.navigationDrawerSecondaryDensity !== "compact"
      || result.navigationDrawerSummaryHeight < 24
      || result.navigationDrawerSummaryHeight > 34
      || result.navigationDrawerSummaryLabel !== "Sessions"
      || result.navigationDrawerClosedSummaryAction !== "Open"
      || result.terminalSessionActionIds.join("|") !== "save-layout|refresh-terminal|clear-command-history"
      || result.terminalSessionActionTones.join("|") !== "secondary|secondary|danger"
      || result.terminalSessionActionLabelModes.join("|") !== "glyph|glyph|glyph"
      || result.terminalSessionActionPlacements.join("|") !== "terminal|terminal|terminal"
      || !result.terminalSessionSecondaryToneStyle
      || !result.terminalSessionDangerToneStyle
      || result.terminalSessionDangerToneStyle.color === result.terminalSessionSecondaryToneStyle.color
      || result.terminalSessionActionLabels.join("|") !== "\u21e9|\u21bb|\u232b"
      || result.terminalSessionActionAriaLabels[0] !== "Save the focused session layout"
      || result.terminalSessionActionAriaLabels[1] !== "Refresh the active terminal session"
      || !/^Clear \d+ command history entr(?:y|ies)$/.test(result.terminalSessionActionAriaLabels[2] ?? "")
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
      || result.workspaceOuterPanelBorderTopWidthPx !== 0
      || result.workspaceOuterPanelRadiusPx !== 0
      || result.workspaceOuterPanelShadow !== "none"
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
      const demoShell = document.querySelector('[data-testid="terminal-demo-shell"]') ?? null;
      const demoMain = document.querySelector('.shell__main') ?? null;
      const workspaceOuterPanel = document.querySelector('.panel--workspace') ?? null;
      const workspaceHostSlot = document.querySelector('[data-testid="terminal-workspace-host"]') ?? null;
      const workspaceHostHeader = document.querySelector('.panel__header--workspace') ?? null;
      const workspaceRoot = workspace?.shadowRoot ?? null;
      const layoutRoot = workspaceRoot?.querySelector('[data-testid="tp-workspace-layout"]') ?? null;
      const operationsDeck = workspaceRoot?.querySelector('[data-testid="tp-workspace-operations-deck"]') ?? null;
      const terminalColumn = workspaceRoot?.querySelector('[data-testid="tp-workspace-terminal-column"]') ?? null;
      const inspectorDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-inspector-drawer"]') ?? null;
      const navigationDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-navigation-drawer"]') ?? null;
      const commandDockElement = workspaceRoot?.querySelector('tp-terminal-command-dock') ?? null;
      const screenElement = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const workspaceFrame = workspaceRoot?.querySelector('[part="workspace"]') ?? null;
      const commandRoot = commandDockElement?.shadowRoot ?? null;
      const screenRoot = screenElement?.shadowRoot ?? null;
      const commandDockPanel = commandRoot?.querySelector('[data-testid="tp-command-dock"]') ?? null;
      const commandAccessoryBar = commandRoot?.querySelector('[data-testid="tp-command-accessory-bar"]') ?? null;
      const commandInputStatus = commandRoot?.querySelector('[data-testid="tp-command-input-status"]') ?? null;
      const commandHistoryBadge = commandRoot?.querySelector('[data-testid="tp-command-history-count"]') ?? null;
      const composer = commandRoot?.querySelector('tp-terminal-command-composer') ?? null;
      const quickCommands = [...(commandRoot?.querySelectorAll('[data-testid="tp-quick-command"]') ?? [])];
      const quickCommandRow = commandRoot?.querySelector('[part="quick-commands"]') ?? null;
      const input = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const run = commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null;
      const paste = commandRoot?.querySelector('[data-testid="tp-paste-clipboard"]') ?? null;
      const interrupt = commandRoot?.querySelector('[data-testid="tp-send-interrupt"]') ?? null;
      const enter = commandRoot?.querySelector('[data-testid="tp-send-enter"]') ?? null;
      const commandActionButtons = [run, paste, interrupt, enter];
      const sessionActionButtons = [
        ...(commandRoot?.querySelectorAll('[data-testid="tp-session-actions"] button') ?? []),
      ];
      const screenPanel = screenRoot?.querySelector('[data-testid="tp-terminal-screen"]') ?? null;
      const screenChrome = screenRoot?.querySelector('[data-testid="tp-screen-chrome"]') ?? null;
      const viewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      const screenActionButtons = [
        screenRoot?.querySelector('[data-testid="tp-screen-follow"]') ?? null,
        screenRoot?.querySelector('[data-testid="tp-screen-scroll-latest"]') ?? null,
        screenRoot?.querySelector('[data-testid="tp-screen-copy"]') ?? null,
      ];
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

      const searchInput = screenRoot?.querySelector('[data-testid="tp-screen-search"]') ?? null;
      if (searchInput) {
        const descriptor = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value');
        descriptor?.set?.call(searchInput, 'static-browser-ok');
        searchInput.dispatchEvent(new Event('input', { bubbles: true }));
        await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      }
      const searchActionButtons = [...(screenRoot?.querySelectorAll('[part="search-actions"] button') ?? [])];
      const terminalSearchActionIds = searchActionButtons.map((button) =>
        button.getAttribute('data-screen-search-action') ?? '',
      );
      const terminalSearchActionLabelModes = searchActionButtons.map((button) =>
        button.getAttribute('data-screen-search-action-label-mode') ?? '',
      );
      const terminalSearchActionPlacements = searchActionButtons.map((button) =>
        button.getAttribute('data-screen-search-action-placement') ?? '',
      );
      const terminalSearchActionLabels = searchActionButtons.map((button) =>
        button.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
      );
      const terminalSearchActionAriaLabels = searchActionButtons.map((button) =>
        button.getAttribute('aria-label') ?? '',
      );
      const terminalSearchHighlights = [...(screenRoot?.querySelectorAll('[part~="search-match"]') ?? [])];
      const terminalSearchActiveHighlight =
        screenRoot?.querySelector('[part~="active-search-match"]') ?? null;
      const terminalSearchActionsInsideChrome =
        searchActionButtons.length > 0
        && searchActionButtons.every((button) => Boolean(screenChrome?.contains(button)));

      const state = window.terminalDemoDebug?.getState?.();
      const terminalText = state?.attachedSession?.focused_screen?.surface?.lines
        ? state.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n')
        : '';
      const terminalColumnRect = terminalColumn?.getBoundingClientRect();
      const screenChromeRect = screenChrome?.getBoundingClientRect();
      const viewportRect = viewport?.getBoundingClientRect();
      const composerRect = composer?.getBoundingClientRect();
      const commandAccessoryBarRect = commandAccessoryBar?.getBoundingClientRect();
      const inspectorDrawerSummary = inspectorDrawer?.querySelector('summary') ?? null;
      const navigationDrawerSummary = navigationDrawer?.querySelector('summary') ?? null;
      const readSecondarySummary = (summary) => {
        const label = summary?.querySelector('.secondary-toggle__label')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null;
        const openAction = summary?.querySelector('.secondary-toggle__action-open') ?? null;
        const closeAction = summary?.querySelector('.secondary-toggle__action-close') ?? null;
        const openVisible = openAction ? getComputedStyle(openAction).display !== 'none' : false;
        const closeVisible = closeAction ? getComputedStyle(closeAction).display !== 'none' : false;
        return {
          action: openVisible
            ? openAction.textContent?.replace(/\\s+/g, ' ').trim() ?? null
            : closeVisible
              ? closeAction.textContent?.replace(/\\s+/g, ' ').trim() ?? null
              : null,
          label,
        };
      };
      const inspectorDrawerSummaryClosed = readSecondarySummary(inspectorDrawerSummary);
      const navigationDrawerSummaryClosed = readSecondarySummary(navigationDrawerSummary);
      const historyEntries = [...(commandRoot?.querySelectorAll('[data-testid="tp-command-history-entry"]') ?? [])];
      const demoShellStyle = demoShell ? getComputedStyle(demoShell) : null;
      const workspaceOuterPanelStyle = workspaceOuterPanel ? getComputedStyle(workspaceOuterPanel) : null;
      const readActionToneStyle = (button) => {
        if (!button) {
          return null;
        }
        const style = getComputedStyle(button);
        return {
          backgroundColor: style.backgroundColor,
          borderColor: style.borderTopColor,
          color: style.color,
          cursor: style.cursor,
          opacity: Number.parseFloat(style.opacity),
        };
      };

      return {
        hasReadyState: state?.connection?.state === 'ready',
        hasWorkspace: Boolean(workspace),
        demoShellCanvas: demoShell?.getAttribute('data-shell-canvas') ?? null,
        demoShellPaddingPx: Number.parseFloat(demoShellStyle?.paddingTop ?? '0'),
        hasTerminalColumn: Boolean(terminalColumn),
        hasCommandDock: Boolean(commandDockPanel),
        hasTerminalScreen: Boolean(screenRoot),
        commandInputRows: input?.rows ?? null,
        commandInputPlaceholder: input?.placeholder ?? null,
        commandInputAutocomplete: input?.getAttribute('autocomplete') ?? null,
        commandInputAutocapitalize: input?.getAttribute('autocapitalize') ?? null,
        commandInputAutocorrect: input?.getAttribute('autocorrect') ?? null,
        commandInputEnterKeyHint: input?.getAttribute('enterkeyhint') ?? null,
        commandInputSpellcheck: input?.getAttribute('spellcheck') ?? null,
        commandInputStatus: commandInputStatus?.textContent?.trim() ?? null,
        commandInputDescribedBy: input?.getAttribute('aria-describedby') ?? null,
        commandInputDescribedByResolves: Boolean(
          input?.getAttribute('aria-describedby')
          && commandRoot?.getElementById(input.getAttribute('aria-describedby') ?? ''),
        ),
        commandInputStatusLive: commandInputStatus?.getAttribute('aria-live') ?? null,
        commandInputStatusAtomic: commandInputStatus?.getAttribute('aria-atomic') ?? null,
        commandHistoryBadgeText: commandHistoryBadge?.textContent?.replace(/\s+/g, ' ').trim() ?? null,
        commandActionLabels: commandActionButtons.map((button) =>
          button?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        ),
        terminalComposerActionPlacements: commandActionButtons.map((button) =>
          button?.getAttribute('data-action-placement') ?? '',
        ),
        terminalComposerActionTones: commandActionButtons.map((button) =>
          button?.getAttribute('data-action-tone') ?? '',
        ),
        terminalComposerActionLabelModes: commandActionButtons.map((button) =>
          button?.getAttribute('data-action-label-mode') ?? '',
        ),
        terminalComposerActionDisabledFlags: commandActionButtons.map((button) =>
          button?.getAttribute('data-action-disabled') ?? '',
        ),
        terminalComposerPrimaryToneStyle: readActionToneStyle(run),
        terminalComposerSecondaryToneStyle: readActionToneStyle(paste),
        commandDockPlacement: commandDockPanel?.getAttribute('data-placement') ?? null,
        commandDockAccessoryMode: commandDockPanel?.getAttribute('data-accessory-mode') ?? null,
        commandAccessoryBarMode: commandAccessoryBar?.getAttribute('data-accessory-mode') ?? null,
        commandAccessoryBarHasHistory: commandAccessoryBar?.getAttribute('data-has-command-history') ?? null,
        commandAccessoryBarQuickCommandCount: commandAccessoryBar?.getAttribute('data-quick-command-count') ?? null,
        commandAccessoryBarRecentCommandCount: commandAccessoryBar?.getAttribute('data-recent-command-count') ?? null,
        hasCommandAccessoryBar: Boolean(commandAccessoryBar),
        terminalCommandAccessoryBarHeight: Math.round(commandAccessoryBarRect?.height ?? 0),
        quickCommandIds: quickCommands.map((button) => button.getAttribute('data-quick-command') ?? ''),
        quickCommandTones: quickCommands.map((button) => button.getAttribute('data-quick-command-tone') ?? ''),
        quickCommandAriaLabels: quickCommands.map((button) => button.getAttribute('aria-label') ?? ''),
        quickCommandHeights: quickCommands.map((button) => Math.round(button.getBoundingClientRect().height)),
        quickCommandWhiteSpaces: quickCommands.map((button) => getComputedStyle(button).whiteSpace),
        quickCommandRowOverflowPx: quickCommandRow
          ? Math.max(0, quickCommandRow.scrollWidth - quickCommandRow.clientWidth)
          : null,
        historyChipHeights: historyEntries.map((button) => Math.round(button.getBoundingClientRect().height)),
        historyChipWhiteSpaces: historyEntries.map((button) => getComputedStyle(button).whiteSpace),
        historyChipCount: historyEntries.length,
        historyChipIds: historyEntries.map((button) => button.getAttribute('data-command-history-entry') ?? ''),
        historyChipHistoryIndexes: historyEntries.map((button) => button.getAttribute('data-history-index') ?? ''),
        historyChipAriaLabels: historyEntries.map((button) => button.getAttribute('aria-label') ?? ''),
        commandDockCanWrite: commandDockPanel?.getAttribute('data-command-input') ?? null,
        commandDockInputCapability: commandDockPanel?.getAttribute('data-input-capability') ?? null,
        screenChromeMode: screenPanel?.getAttribute('data-chrome-mode') ?? null,
        hasCompactScreenChrome: Boolean(screenChrome && screenChrome.getAttribute('data-chrome-mode') === 'compact'),
        terminalScreenChromeHeight: Math.round(screenChromeRect?.height ?? 0),
        terminalScreenChromeViewportGapPx: screenChromeRect && viewportRect
          ? Math.round(viewportRect.top - screenChromeRect.bottom)
          : null,
        terminalScreenCompactSizeLabel: screenRoot?.querySelector('[data-meta-id="size"]')?.textContent?.trim() ?? null,
        terminalScreenActionIds: screenActionButtons.map((button) => button?.getAttribute('data-screen-action') ?? ''),
        terminalScreenActionTones: screenActionButtons.map((button) =>
          button?.getAttribute('data-screen-action-tone') ?? '',
        ),
        terminalScreenActionLabelModes: screenActionButtons.map((button) =>
          button?.getAttribute('data-screen-action-label-mode') ?? '',
        ),
        terminalScreenActionPlacements: screenActionButtons.map((button) =>
          button?.getAttribute('data-screen-action-placement') ?? '',
        ),
        terminalScreenPrimaryToneStyle: readActionToneStyle(screenActionButtons[0]),
        terminalScreenSecondaryToneStyle: readActionToneStyle(screenActionButtons[1]),
        terminalScreenActionLabels: screenActionButtons.map((button) =>
          button?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        ),
        terminalScreenActionAriaLabels: screenActionButtons.map((button) => button?.getAttribute('aria-label') ?? null),
        terminalScreenActionTitles: screenActionButtons.map((button) => button?.getAttribute('title') ?? null),
        terminalScreenActionPressedFlags: screenActionButtons.map((button) => button?.getAttribute('aria-pressed') ?? ''),
        terminalSearchActionIds,
        terminalSearchActionLabelModes,
        terminalSearchActionPlacements,
        terminalSearchActionLabels,
        terminalSearchActionAriaLabels,
        terminalSearchActionsInsideChrome,
        terminalSearchInputType: searchInput?.getAttribute('type') ?? null,
        terminalSearchInputAutocomplete: searchInput?.getAttribute('autocomplete') ?? null,
        terminalSearchInputAutocapitalize: searchInput?.getAttribute('autocapitalize') ?? null,
        terminalSearchInputAutocorrect: searchInput?.getAttribute('autocorrect') ?? null,
        terminalSearchInputEnterKeyHint: searchInput?.getAttribute('enterkeyhint') ?? null,
        terminalSearchInputInputMode: searchInput?.getAttribute('inputmode') ?? null,
        terminalSearchInputSpellcheck: searchInput?.getAttribute('spellcheck') ?? null,
        terminalSearchInputDescribedBy: searchInput?.getAttribute('aria-describedby') ?? null,
        terminalSearchInputDescribedByResolves: Boolean(
          searchInput?.getAttribute('aria-describedby')
          && screenRoot?.getElementById(searchInput.getAttribute('aria-describedby') ?? ''),
        ),
        terminalSearchCountLive:
          screenRoot?.querySelector('[part="search-count"]')?.getAttribute('aria-live') ?? null,
        terminalSearchCountAtomic:
          screenRoot?.querySelector('[part="search-count"]')?.getAttribute('aria-atomic') ?? null,
        terminalSearchHighlightTexts: terminalSearchHighlights.map((highlight) => highlight.textContent ?? ''),
        terminalSearchActiveHighlightText: terminalSearchActiveHighlight?.textContent ?? null,
        workspaceLayoutPreset: layoutRoot?.getAttribute('data-layout-preset') ?? null,
        workspaceNavigationMode: layoutRoot?.getAttribute('data-navigation-mode') ?? null,
        workspaceInspectorMode: operationsDeck?.getAttribute('data-inspector-mode') ?? null,
        workspaceChromeTone: workspaceFrame?.getAttribute('data-chrome-tone') ?? null,
        workspaceSecondaryChrome: layoutRoot?.getAttribute('data-secondary-chrome') ?? null,
        workspaceSecondaryDensity: layoutRoot?.getAttribute('data-secondary-density') ?? null,
        inspectorDrawerSecondaryChrome: inspectorDrawer?.getAttribute('data-secondary-chrome') ?? null,
        inspectorDrawerSecondaryDensity: inspectorDrawer?.getAttribute('data-secondary-density') ?? null,
        inspectorDrawerSummaryHeight: Math.round(inspectorDrawerSummary?.getBoundingClientRect().height ?? 0),
        inspectorDrawerSummaryLabel: inspectorDrawerSummaryClosed.label,
        inspectorDrawerClosedSummaryAction: inspectorDrawerSummaryClosed.action,
        navigationDrawerSecondaryChrome: navigationDrawer?.getAttribute('data-secondary-chrome') ?? null,
        navigationDrawerSecondaryDensity: navigationDrawer?.getAttribute('data-secondary-density') ?? null,
        navigationDrawerSummaryHeight: Math.round(navigationDrawerSummary?.getBoundingClientRect().height ?? 0),
        navigationDrawerSummaryLabel: navigationDrawerSummaryClosed.label,
        navigationDrawerClosedSummaryAction: navigationDrawerSummaryClosed.action,
        terminalSessionActionIds: sessionActionButtons.map((button) => button.getAttribute('data-session-action')),
        terminalSessionActionTones: sessionActionButtons.map((button) =>
          button.getAttribute('data-session-action-tone') ?? '',
        ),
        terminalSessionActionLabelModes: sessionActionButtons.map((button) =>
          button.getAttribute('data-session-action-label-mode') ?? '',
        ),
        terminalSessionActionPlacements: sessionActionButtons.map((button) =>
          button.getAttribute('data-session-action-placement') ?? '',
        ),
        terminalSessionSecondaryToneStyle: readActionToneStyle(sessionActionButtons[0]),
        terminalSessionDangerToneStyle: readActionToneStyle(
          sessionActionButtons.find((button) => button.getAttribute('data-session-action-tone') === 'danger') ?? null,
        ),
        terminalSessionActionLabels: sessionActionButtons.map((button) =>
          button.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        ),
        terminalSessionActionAriaLabels: sessionActionButtons.map((button) => button.getAttribute('aria-label')),
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
        workspaceOuterPanelBorderTopWidthPx: Number.parseFloat(workspaceOuterPanelStyle?.borderTopWidth ?? '0'),
        workspaceOuterPanelRadiusPx: Number.parseFloat(workspaceOuterPanelStyle?.borderTopLeftRadius ?? '0'),
        workspaceOuterPanelShadow: workspaceOuterPanelStyle?.boxShadow ?? null,
        workspacePanelShadow: workspaceFrame
          ? getComputedStyle(workspaceFrame).getPropertyValue('--tp-shadow-panel').trim()
          : null,
        terminalComposerGapPx: viewportRect && composerRect ? Math.round(composerRect.top - viewportRect.bottom) : null,
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
