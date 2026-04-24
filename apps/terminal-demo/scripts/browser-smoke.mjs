#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const viteCliPath = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
const rendererPort = process.env.TERMINAL_DEMO_SMOKE_RENDERER_PORT ?? "4273";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;
const cdpPort = process.env.TERMINAL_DEMO_SMOKE_CDP_PORT ?? "9226";
const chromeBinary = resolveChromeBinary();
const screenshotPath = path.join("/tmp", `terminal-demo-browser-smoke-${Date.now()}.png`);
const chromeUserDataDir = path.join("/tmp", `terminal-demo-browser-smoke-profile-${process.pid}`);
const sessionStorePath = path.join("/tmp", `terminal-demo-browser-smoke-store-${process.pid}-${Date.now()}.sqlite3`);
const themeStorageKey = "terminal-platform-demo.theme";
const fontScaleStorageKey = "terminal-platform-demo.terminal-font-scale";
const lineWrapStorageKey = "terminal-platform-demo.terminal-line-wrap";

let previewProcess = null;
let browserHostProcess = null;
let chromeProcess = null;

await main();

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
    });
    pipeProcess(previewProcess, "[browser-smoke:preview]");
    await waitForServer(rendererUrl, {
      child: previewProcess,
      label: "Renderer preview",
    });

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
    await waitForServer(`http://127.0.0.1:${cdpPort}/json/version`, {
      child: chromeProcess,
      label: "Chrome CDP",
    });

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
      || !result.afterCreate.screenPrecedesCommandDock
      || !result.afterCreate.hasScreenFollowControls
      || !result.afterCreate.hasScreenSearchControls
      || !result.afterCreate.hasScreenCopyControl
      || !result.afterCreate.hasSaveLayoutControl
      || !result.afterCreate.hasDisplayControls
      || result.afterCreate.savedSessionCount !== 0
      || !result.afterCreate.screenFollowPressed
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
      !result.afterThemeSwitch.clicked
      || result.afterThemeSwitch.themeId !== "terminal-platform-light"
      || result.afterThemeSwitch.workspaceTheme !== "terminal-platform-light"
      || result.afterThemeSwitch.screenTheme !== "terminal-platform-light"
      || result.afterThemeSwitch.activeThemeButton !== "terminal-platform-light"
      || result.afterThemeSwitch.screenBgToken !== "#f6f8fb"
      || result.afterThemeSwitch.storedTheme !== "terminal-platform-light"
    ) {
      throw new Error(`Theme switch did not apply to SDK elements: ${JSON.stringify(result.afterThemeSwitch)}`);
    }

    if (
      !result.afterDisplaySwitch.clicked
      || result.afterDisplaySwitch.fontScale !== "large"
      || result.afterDisplaySwitch.lineWrap !== false
      || result.afterDisplaySwitch.screenFontScale !== "large"
      || result.afterDisplaySwitch.screenLineWrap !== "false"
      || result.afterDisplaySwitch.activeFontScaleButton !== "large"
      || result.afterDisplaySwitch.wrapPressed !== "false"
      || result.afterDisplaySwitch.storedFontScale !== "large"
      || result.afterDisplaySwitch.storedLineWrap !== "false"
    ) {
      throw new Error(`Terminal display preferences did not apply to SDK elements: ${JSON.stringify(result.afterDisplaySwitch)}`);
    }

    if (
      !result.afterSaveLayout.clicked
      || result.afterSaveLayout.savedSessionCount < result.afterSaveLayout.beforeSavedSessionCount
      || result.afterSaveLayout.savedItemsRendered < 1
      || !result.afterSaveLayout.deletePrompted
      || result.afterSaveLayout.savedSessionCountAfterDeletePrompt !== result.afterSaveLayout.savedSessionCount
      || result.afterSaveLayout.saveEventDetail?.savedSessionCount !== result.afterSaveLayout.savedSessionCount
    ) {
      throw new Error(`Save layout workflow did not complete: ${JSON.stringify(result.afterSaveLayout)}`);
    }

    if (
      !result.afterScreenSearch.searched
      || result.afterScreenSearch.matchCount < 1
      || !result.afterScreenSearch.hasHighlights
      || !result.afterScreenSearch.hasActiveHighlight
      || !result.afterScreenSearch.nextClicked
    ) {
      throw new Error(`Terminal screen search did not highlight output: ${JSON.stringify(result.afterScreenSearch)}`);
    }

    if (
      !result.afterCommand.connectionReady
      || !result.afterCommand.screenFollowPressed
      || !result.afterCommand.screenViewportAtBottom
      || (!result.afterCommand.sequenceAdvanced && !result.afterCommand.containsCommandOutput)
    ) {
      throw new Error(`Command lane did not advance the focused screen: ${JSON.stringify(result.afterCommand)}`);
    }

    if (
      !result.afterScreenFollowToggle.paused
      || !result.afterScreenFollowToggle.resumed
      || !result.afterScreenFollowToggle.screenViewportAtBottom
    ) {
      throw new Error(`Screen follow controls did not toggle correctly: ${JSON.stringify(result.afterScreenFollowToggle)}`);
    }

    if (
      !result.afterHistoryReplay.recalledDraft?.includes("browser-smoke-ok")
      || !result.afterHistoryReplay.replayClicked
      || !result.afterHistoryReplay.connectionReady
      || (!result.afterHistoryReplay.sequenceAdvanced && !result.afterHistoryReplay.containsCommandOutput)
    ) {
      throw new Error(`Command history replay did not settle correctly: ${JSON.stringify(result.afterHistoryReplay)}`);
    }

    if (
      !result.afterDirectScreenInput.focused
      || result.afterDirectScreenInput.submittedEvents < 1
      || !result.afterDirectScreenInput.connectionReady
      || !result.afterDirectScreenInput.containsDirectOutput
    ) {
      throw new Error(`Direct terminal screen input did not settle correctly: ${JSON.stringify(result.afterDirectScreenInput)}`);
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
      const toolbarRoot = workspaceRoot?.querySelector('tp-terminal-toolbar')?.shadowRoot ?? null;
      const contentRoot = workspaceRoot?.querySelector('[part="content"]') ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const commandDockHost = workspaceRoot?.querySelector('tp-terminal-command-dock') ?? null;
      const screenRoot = screenHost?.shadowRoot ?? null;
      const screenFollow = screenRoot?.querySelector('[data-testid="tp-screen-follow"]') ?? null;
      const screenSearch = screenRoot?.querySelector('[data-testid="tp-screen-search"]') ?? null;
      const screenCopy = screenRoot?.querySelector('[data-testid="tp-screen-copy"]') ?? null;
      const screenViewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      const saveLayout = commandRoot?.querySelector('[data-testid="tp-save-layout"]') ?? null;
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
        hasScreenFollowControls: Boolean(screenFollow && screenRoot?.querySelector('[data-testid="tp-screen-scroll-latest"]')),
        hasScreenSearchControls: Boolean(screenSearch),
        hasScreenCopyControl: Boolean(screenCopy && !screenCopy.disabled),
        hasSaveLayoutControl: Boolean(saveLayout && !saveLayout.disabled),
        hasDisplayControls: Boolean(
          toolbarRoot?.querySelector('[data-testid="tp-font-scale-option"][data-font-scale="large"]')
          && toolbarRoot?.querySelector('[data-testid="tp-line-wrap-option"]')
        ),
        screenFollowPressed: screenFollow?.getAttribute('aria-pressed') === 'true',
        screenViewportAtBottom: screenViewport
          ? screenViewport.scrollHeight - screenViewport.scrollTop - screenViewport.clientHeight <= 2
          : false,
        screenPrecedesCommandDock: Boolean(
          contentRoot
          && screenHost
          && commandDockHost
          && [...contentRoot.children].indexOf(screenHost) > -1
          && [...contentRoot.children].indexOf(screenHost) < [...contentRoot.children].indexOf(commandDockHost)
        ),
        hasScreen: Boolean(terminalScreenText),
        hasStatusBar: Boolean(statusRoot?.querySelector('[part="status-bar"]')),
        hasCommandDock: Boolean(commandRoot?.querySelector('[part="command-dock"]')),
        hasActiveTitle: Boolean(activeTitle && activeTitle !== 'Pick a session to inspect'),
        inputEnabled: Boolean(input && !input.disabled),
      };
    })()`);

    const initialSequence = afterCreate.focusedSequence;
    let afterScreenSearch = {
      searched: false,
      reason: "deferred until command output is present",
      matchCount: 0,
      hasHighlights: false,
      hasActiveHighlight: false,
      nextClicked: false,
    };

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

    const afterThemeSwitch = await evaluate(send, `(async () => {
      const debug = window.terminalDemoDebug?.getState?.();
      const workspaceHost = document.querySelector('tp-terminal-workspace');
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const toolbarRoot = workspaceRoot?.querySelector('tp-terminal-toolbar')?.shadowRoot ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const commandDockHost = workspaceRoot?.querySelector('tp-terminal-command-dock') ?? null;
      const themeButton = [...(toolbarRoot?.querySelectorAll('[part="theme-option"]') ?? [])]
        .find((button) => button.getAttribute('data-theme-id') === 'terminal-platform-light') ?? null;
      if (!themeButton) {
        return {
          clicked: false,
          reason: 'light theme button missing',
          themeId: debug?.theme?.themeId ?? null,
        };
      }

      themeButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

      return {
        clicked: true,
        themeId: window.terminalDemoDebug?.getState?.()?.theme?.themeId ?? null,
        workspaceTheme: workspaceHost?.getAttribute('data-tp-theme') ?? null,
        screenTheme: screenHost?.getAttribute('data-tp-theme') ?? null,
        commandDockTheme: commandDockHost?.getAttribute('data-tp-theme') ?? null,
        activeThemeButton: toolbarRoot?.querySelector('[part="theme-option"][aria-pressed="true"]')
          ?.getAttribute('data-theme-id') ?? null,
        storedTheme: window.localStorage.getItem(${JSON.stringify(themeStorageKey)}),
        screenBgToken: screenHost
          ? getComputedStyle(screenHost).getPropertyValue('--tp-color-bg').trim()
          : null,
      };
    })()`);

    const afterDisplaySwitch = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const toolbarRoot = workspaceRoot?.querySelector('tp-terminal-toolbar')?.shadowRoot ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const largeButton = toolbarRoot?.querySelector('[data-testid="tp-font-scale-option"][data-font-scale="large"]') ?? null;
      const wrapButton = toolbarRoot?.querySelector('[data-testid="tp-line-wrap-option"]') ?? null;
      if (!largeButton || !wrapButton) {
        return {
          clicked: false,
          reason: largeButton ? 'wrap button missing' : 'large font button missing',
        };
      }

      largeButton.click();
      if (wrapButton.getAttribute('aria-pressed') === 'true') {
        wrapButton.click();
      }
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

      const state = window.terminalDemoDebug?.getState?.();
      return {
        clicked: true,
        fontScale: state?.terminalDisplay?.fontScale ?? null,
        lineWrap: state?.terminalDisplay?.lineWrap ?? null,
        screenFontScale: screenHost?.getAttribute('data-font-scale') ?? null,
        screenLineWrap: screenHost?.getAttribute('data-line-wrap') ?? null,
        activeFontScaleButton: toolbarRoot?.querySelector('[part="font-scale-option"][aria-pressed="true"]')
          ?.getAttribute('data-font-scale') ?? null,
        wrapPressed: wrapButton.getAttribute('aria-pressed'),
        storedFontScale: window.localStorage.getItem(${JSON.stringify(fontScaleStorageKey)}),
        storedLineWrap: window.localStorage.getItem(${JSON.stringify(lineWrapStorageKey)}),
      };
    })()`);

    const afterSaveLayout = await evaluate(send, `(async () => {
      const beforeSavedSessionCount = window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      const workspaceHost = document.querySelector('tp-terminal-workspace') ?? null;
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const savedRoot = workspaceRoot?.querySelector('tp-terminal-saved-sessions')?.shadowRoot ?? null;
      const sessionTools = commandRoot?.querySelector('[data-testid="tp-session-tools"]') ?? null;
      const saveLayoutButton = commandRoot?.querySelector('[data-testid="tp-save-layout"]') ?? null;
      if (!sessionTools || !saveLayoutButton) {
        return {
          clicked: false,
          reason: sessionTools ? 'save layout button missing' : 'session tools missing',
          beforeSavedSessionCount,
          savedSessionCount: beforeSavedSessionCount,
          savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        };
      }

      sessionTools.open = true;
      let saveEventDetail = null;
      workspaceHost?.addEventListener('tp-terminal-layout-saved', (event) => {
        saveEventDetail = event.detail ?? null;
      }, { once: true });
      if (saveLayoutButton.disabled) {
        return {
          clicked: false,
          reason: 'save layout disabled',
          beforeSavedSessionCount,
          savedSessionCount: beforeSavedSessionCount,
          savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        };
      }

      saveLayoutButton.click();
      await new Promise((resolve) => setTimeout(resolve, 850));
      const state = window.terminalDemoDebug?.getState?.();
      const deleteButton = savedRoot?.querySelector('[data-testid="tp-delete-saved-session"]') ?? null;
      const savedSessionCount = state?.catalog?.savedSessions?.length ?? 0;
      const deletePromptResult = {
        deletePrompted: false,
        deleteButtonText: deleteButton?.textContent?.trim() ?? null,
        savedSessionCountAfterDeletePrompt: savedSessionCount,
      };
      if (deleteButton && !deleteButton.disabled) {
        deleteButton.click();
        await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        deletePromptResult.deletePrompted = deleteButton.getAttribute('data-confirming') === 'true'
          && /confirm delete/i.test(deleteButton.textContent ?? '');
        deletePromptResult.deleteButtonText = deleteButton.textContent?.trim() ?? null;
        deletePromptResult.savedSessionCountAfterDeletePrompt =
          window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      }

      return {
        clicked: true,
        beforeSavedSessionCount,
        savedSessionCount,
        savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        firstSavedTitle: state?.catalog?.savedSessions?.[0]?.title ?? null,
        saveEventDetail,
        ...deletePromptResult,
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
      const screenFollow = screenRoot?.querySelector('[data-testid="tp-screen-follow"]') ?? null;
      const screenViewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      const terminalScreenText = debug?.attachedSession?.focused_screen?.surface?.lines
        ? debug.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : (screenRoot?.querySelector('[part=\"screen-lines\"]')?.textContent?.trim() ?? '');
      return {
        connectionReady: debug?.connection?.state === 'ready',
        focusedSequence: debug?.attachedSession?.focused_screen?.sequence != null
          ? String(debug.attachedSession.focused_screen.sequence)
          : null,
        screenFollowPressed: screenFollow?.getAttribute('aria-pressed') === 'true',
        screenViewportAtBottom: screenViewport
          ? screenViewport.scrollHeight - screenViewport.scrollTop - screenViewport.clientHeight <= 2
          : false,
        screenViewportMetrics: screenViewport
          ? {
              scrollHeight: screenViewport.scrollHeight,
              scrollTop: screenViewport.scrollTop,
              clientHeight: screenViewport.clientHeight,
            }
          : null,
        terminalScreenText,
        containsCommandOutput: /browser-smoke-ok/i.test(terminalScreenText),
      };
    })()`);
    const replayInitialSequence = afterCommand.focusedSequence;

    afterScreenSearch = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const screenRoot = workspaceRoot?.querySelector('tp-terminal-screen')?.shadowRoot ?? null;
      const searchInput = screenRoot?.querySelector('[data-testid="tp-screen-search"]') ?? null;
      const query = 'browser-smoke-ok';
      if (!searchInput) {
        return {
          searched: false,
          reason: 'search input missing',
          matchCount: 0,
          hasHighlights: false,
          hasActiveHighlight: false,
          nextClicked: false,
        };
      }

      const descriptor = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
      descriptor?.set?.call(searchInput, query);
      searchInput.dispatchEvent(new Event('input', { bubbles: true }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const nextButton = screenRoot?.querySelector('[data-testid="tp-screen-search-next"]') ?? null;
      const nextClicked = Boolean(nextButton && !nextButton.disabled);
      if (nextClicked) {
        nextButton.click();
        await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      }
      const countText = screenRoot?.querySelector('[part="search-count"]')?.textContent?.trim() ?? '';
      return {
        searched: true,
        query,
        countText,
        matchCount: Number.parseInt(countText, 10) || 0,
        hasHighlights: Boolean(screenRoot?.querySelector('[part="search-match"]')),
        hasActiveHighlight: Boolean(screenRoot?.querySelector('[part~="active-search-match"]')),
        nextClicked,
      };
    })()`);

    const afterScreenFollowToggle = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const screenRoot = workspaceRoot?.querySelector('tp-terminal-screen')?.shadowRoot ?? null;
      const followButton = screenRoot?.querySelector('[data-testid="tp-screen-follow"]') ?? null;
      const scrollLatestButton = screenRoot?.querySelector('[data-testid="tp-screen-scroll-latest"]') ?? null;
      const viewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      if (!followButton || !scrollLatestButton || !viewport) {
        return {
          paused: false,
          resumed: false,
          reason: 'screen controls missing',
          screenViewportAtBottom: false,
        };
      }

      followButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const paused = followButton.getAttribute('aria-pressed') === 'false' && followButton.textContent?.trim() === 'Paused';

      scrollLatestButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const resumed = followButton.getAttribute('aria-pressed') === 'true' && followButton.textContent?.trim() === 'Following';

      return {
        paused,
        resumed,
        screenViewportAtBottom: viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= 2,
      };
    })()`);

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

    const directScreenInputResult = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const screenRoot = screenHost?.shadowRoot ?? null;
      const viewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      if (!screenHost || !screenRoot || !viewport) {
        return {
          ok: false,
          reason: 'screen viewport missing',
          focused: false,
          submittedEvents: 0,
        };
      }

      let submittedEvents = 0;
      const handleSubmitted = () => {
        submittedEvents += 1;
      };
      screenHost.addEventListener('tp-terminal-screen-input-submitted', handleSubmitted);
      viewport.focus();
      const directInput = ${JSON.stringify('printf "screen-key-ok\\n"')};
      for (const key of [...directInput, 'Enter']) {
        viewport.dispatchEvent(new KeyboardEvent('keydown', {
          key,
          bubbles: true,
          cancelable: true,
        }));
        await new Promise((resolve) => setTimeout(resolve, 20));
      }
      await new Promise((resolve) => setTimeout(resolve, 1200));
      screenHost.removeEventListener('tp-terminal-screen-input-submitted', handleSubmitted);

      return {
        ok: true,
        focused: screenRoot.activeElement === viewport,
        submittedEvents,
      };
    })()`);
    if (!directScreenInputResult.ok) {
      throw new Error(`Unable to send input through terminal screen: ${JSON.stringify(directScreenInputResult)}`);
    }

    await sleep(2500);

    const afterDirectScreenInput = await evaluate(send, `(() => {
      const debug = window.terminalDemoDebug?.getState?.();
      const terminalScreenText = debug?.attachedSession?.focused_screen?.surface?.lines
        ? debug.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : '';
      return {
        focused: ${JSON.stringify(directScreenInputResult.focused)},
        submittedEvents: ${JSON.stringify(directScreenInputResult.submittedEvents)},
        connectionReady: debug?.connection?.state === 'ready',
        terminalScreenText,
        containsDirectOutput: /screen-key-ok/i.test(terminalScreenText),
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
      afterScreenSearch,
      afterSavedPagination,
      afterThemeSwitch,
      afterDisplaySwitch,
      afterSaveLayout,
      afterCommand: {
        ...afterCommand,
        sequenceAdvanced: initialSequence !== null
          ? afterCommand.focusedSequence !== initialSequence
          : false,
      },
      afterScreenFollowToggle,
      afterHistoryReplay: {
        ...afterHistoryReplay,
        sequenceAdvanced: replayInitialSequence !== null
          ? afterHistoryReplay.focusedSequence !== replayInitialSequence
          : false,
      },
      afterDirectScreenInput,
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
        TERMINAL_DEMO_SESSION_STORE_PATH: sessionStorePath,
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
  await Promise.all([
    fs.rm(sessionStorePath, { force: true }),
    fs.rm(`${sessionStorePath}-shm`, { force: true }),
    fs.rm(`${sessionStorePath}-wal`, { force: true }),
  ]);
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

async function waitForServer(url, options = {}) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < 20_000) {
    const exitState = processExitState(options.child);
    if (exitState) {
      throw new Error(`${options.label ?? "Server"} exited before ${url} became ready - ${exitState}`);
    }

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

function processExitState(child) {
  if (!child) {
    return null;
  }

  if (child.exitCode !== null) {
    return `exit code ${child.exitCode}`;
  }

  if (child.signalCode !== null) {
    return `signal ${child.signalCode}`;
  }

  return null;
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
