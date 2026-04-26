#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs/promises";
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
const viteCliPath = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
const rendererPort = process.env.TERMINAL_DEMO_SMOKE_RENDERER_PORT ?? "4273";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;
const cdpPort = process.env.TERMINAL_DEMO_SMOKE_CDP_PORT ?? "9226";
const screenshotPath = path.join("/tmp", `terminal-demo-browser-smoke-${Date.now()}.png`);
const sessionStorePath = path.join("/tmp", `terminal-demo-browser-smoke-store-${process.pid}-${Date.now()}.sqlite3`);
const autoStartSessionStorePath = path.join(
  "/tmp",
  `terminal-demo-browser-smoke-auto-store-${process.pid}-${Date.now()}.sqlite3`,
);
const themeStorageKey = "terminal-platform-demo.theme";
const fontScaleStorageKey = "terminal-platform-demo.terminal-font-scale";
const lineWrapStorageKey = "terminal-platform-demo.terminal-line-wrap";

let previewProcess = null;
let browserHostProcess = null;
let chromeProcess = null;
let chromeUserDataDir = null;

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
    await waitForHttpServer(rendererUrl, {
      child: previewProcess,
      label: "Renderer preview",
    });

    const chromeLaunch = await launchChromeWithCdp({
      appRoot,
      binaryMissingMessage: "Chrome binary not found. Set TERMINAL_DEMO_CHROME_BIN to run browser smoke.",
      cdpPort,
      extraArgs: [
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--disable-software-rasterizer",
        "--no-first-run",
        "--no-default-browser-check",
        "--no-sandbox",
      ],
      headlessModeEnv: "TERMINAL_DEMO_SMOKE_HEADLESS_MODE",
      logPrefix: "browser-smoke:chrome",
      profilePrefix: "terminal-demo-browser-smoke-profile",
    });
    chromeProcess = chromeLaunch.child;
    chromeUserDataDir = chromeLaunch.userDataDir;

    const autoStartBrowserUrl = await startBrowserHost(rendererUrl, {
      autoStartSession: "1",
      sessionStorePath: autoStartSessionStorePath,
    });
    const autoStartDefaultShellProgram =
      new URL(autoStartBrowserUrl).searchParams.get("demoDefaultShellProgram");
    const autoStartResult = await runAutoStartSmokeScenario(autoStartBrowserUrl);
    if (autoStartResult.issues.length > 0) {
      throw new Error(`Browser auto-start reported runtime issues: ${JSON.stringify(autoStartResult.issues)}`);
    }

    if (
      !autoStartResult.hasReady
      || autoStartResult.hasError
      || autoStartResult.sessionCount !== 1
      || !autoStartResult.attached
      || autoStartResult.demoAutoStartSession !== null
      || !autoStartDefaultShellProgram
      || autoStartResult.demoDefaultShellProgram !== autoStartDefaultShellProgram
      || !autoStartResult.commandInputFocused
      || autoStartResult.documentHorizontalOverflow > 1
      || /default interactive shell is now zsh/i.test(autoStartResult.terminalScreenTextPreview ?? "")
    ) {
      throw new Error(`Host auto-start did not settle into one default shell: ${JSON.stringify(autoStartResult)}`);
    }

    await stopProcess(browserHostProcess);
    browserHostProcess = null;
    await removeSessionStore(autoStartSessionStorePath);

    const browserUrl = await startBrowserHost(rendererUrl, {
      autoStartSession: "0",
      sessionStorePath,
    });
    const result = await runSmokeScenario(browserUrl);

    if (result.issues.length > 0) {
      throw new Error(`Browser reported runtime issues: ${JSON.stringify(result.issues)}`);
    }

    if (
      !result.before.hasWorkspaceShell
      || !result.before.hasStartDefaultShell
      || result.before.sessionCount !== 0
      || result.before.demoAutoStartSession !== null
    ) {
      throw new Error(`Browser smoke should start from an explicit-launch state: ${JSON.stringify(result.before)}`);
    }

    if (
      !result.afterCreate.hasReady
      || result.afterCreate.hasError
      || result.afterCreate.healthPhase !== "ready"
      || !result.afterCreate.hasStatusBar
      || !result.afterCreate.hasCommandDock
      || result.afterCreate.demoShellActive !== "true"
      || result.afterCreate.demoShellMode !== "terminal"
      || result.afterCreate.workspaceHeroVisible !== false
      || result.afterCreate.launcherPanelVisible !== false
      || result.afterCreate.demoShellColumnCount !== 1
      || result.afterCreate.demoMainWidth < 1000
      || result.afterCreate.workspaceHostWidth < 1000
      || result.afterCreate.workspaceHostTopOffset == null
      || result.afterCreate.workspaceHostTopOffset > 20
      || result.afterCreate.workspaceContentWidth < 800
      || result.afterCreate.workspaceHostHeaderDisplay !== "none"
      || result.afterCreate.terminalColumnHeight < 560
      || result.afterCreate.screenViewportHeight < 360
      || result.afterCreate.workspacePanelShadow !== "none"
      || result.afterCreate.documentHorizontalOverflow > 1
      || result.afterCreate.workspaceLayout !== "operations-deck"
      || result.afterCreate.workspaceNavigationMode !== "collapsed"
      || !result.afterCreate.hasOperationsDeck
      || !result.afterCreate.hasNavigationDrawer
      || result.afterCreate.navigationDrawerOpen !== false
      || !result.afterCreate.navigationDrawerOpenedAfterClick
      || !result.afterCreate.navigationDrawerClosedAfterToggle
      || !result.afterCreate.navigationVisibleAfterDrawerOpen
      || result.afterCreate.workspaceInspectorMode !== "collapsed"
      || !result.afterCreate.hasInspectorDrawer
      || result.afterCreate.inspectorDrawerOpen !== false
      || !result.afterCreate.inspectorDrawerOpenedAfterClick
      || !result.afterCreate.inspectorDrawerClosedAfterToggle
      || !result.afterCreate.paneTreeVisibleAfterDrawerOpen
      || result.afterCreate.operationsDeckColumnCount !== 1
      || !result.afterCreate.screenInTerminalColumn
      || !result.afterCreate.commandDockInCommandRegion
      || !result.afterCreate.topologyInInspectorColumn
      || !result.afterCreate.topologyInInspectorDrawer
      || !result.afterCreate.terminalTabStripInTerminalColumn
      || !result.afterCreate.terminalTabStripBeforeScreen
      || result.afterCreate.terminalTabStripGapPx !== 0
      || result.afterCreate.terminalTabStripTabCount !== "1"
      || result.afterCreate.terminalTabStripRenderedTabs !== 1
      || result.afterCreate.terminalTabStripActiveTabs !== 1
      || result.afterCreate.terminalTabStripCloseButtons !== 1
      || result.afterCreate.terminalTabStripEnabledCloseButtons !== 0
      || !result.afterCreate.terminalTabStripNewTabEnabled
      || !result.afterCreate.screenPrecedesCommandDock
      || !result.afterCreate.hasScreenFollowControls
      || !result.afterCreate.hasScreenSearchControls
      || !result.afterCreate.hasScreenCopyControl
      || !result.afterCreate.hasScreenDirectInput
      || result.afterCreate.screenPlacement !== "terminal"
      || /Focused pane output/.test(result.afterCreate.screenVisibleText ?? "")
      || result.afterCreate.screenInputStatus !== "Input ready"
      || result.afterCreate.screenInputTone !== "ready"
      || !result.afterCreate.hasPasteClipboardControl
      || !result.afterCreate.hasSaveLayoutControl
      || result.afterCreate.commandDockCanSave !== "true"
      || result.afterCreate.commandDockSaveCapability !== "known"
      || !result.afterCreate.hasTopologyControls
      || result.afterCreate.topologyStatus !== "Topology ready"
      || result.afterCreate.topologyCapabilityStatus !== "known"
      || result.afterCreate.topologyCanMutateLayout !== "true"
      || !result.afterCreate.hasEnabledTopologyMutationControls
      || !result.afterCreate.hasDisplayControls
      || result.afterCreate.commandDockCanWrite !== "true"
      || result.afterCreate.commandDockInputCapability !== "known"
      || result.afterCreate.commandDockPlacement !== "terminal"
      || Math.abs(result.afterCreate.terminalComposerGapPx ?? 99) > 1
      || Math.abs(result.afterCreate.terminalInputGapPx ?? 99) > 1
      || result.afterCreate.terminalDockBottomOverflowPx !== 0
      || !result.afterCreate.terminalComposerBeforeDockStatus
      || !result.afterCreate.terminalComposerBeforeDockStatusDom
      || !result.afterCreate.terminalComposerFirstInDockDom
      || result.afterCreate.terminalComposerTagName !== "TP-TERMINAL-COMMAND-COMPOSER"
      || result.afterCreate.terminalComposerPromptPart !== "prompt"
      || result.afterCreate.terminalComposerInputPart !== "input"
      || result.afterCreate.terminalComposerActionParts.join("|") !== "send-command|paste-clipboard|send-interrupt|send-enter"
      || result.afterCreate.terminalComposerActionIds.join("|") !== "submit|paste|interrupt|enter"
      || result.afterCreate.terminalComposerActionKeyHints.join("|") !== "Enter||Ctrl+C|Enter"
      || result.afterCreate.terminalComposerActionAriaKeyShortcuts.join("|") !== "Enter|||"
      || result.afterCreate.commandInputRows !== 1
      || result.afterCreate.commandInputRowCount !== "1"
      || result.afterCreate.commandInputMultiline !== "false"
      || result.afterCreate.commandComposerMinRows !== 1
      || result.afterCreate.commandComposerMaxRows !== 5
      || !result.afterCreate.terminalCommandActionsInsideComposer
      || result.afterCreate.terminalFooterActionCount !== 0
      || /Command Input|Focused pane command lane/.test(result.afterCreate.commandDockVisibleText ?? "")
      || result.afterCreate.commandInputStatus !== "Ready"
      || !result.afterCreate.commandInputFocused
      || !result.afterCreate.focusedPaneBadgeText?.includes("Focused pane")
      || result.afterCreate.statusSessionTitle !== result.afterCreate.activeSessionId
      || !result.afterCreate.statusSessionText?.includes("Session")
      || result.afterCreate.activeSessionListIdTitle !== result.afterCreate.activeSessionId
      || !result.afterCreate.activeSessionListIdText?.includes("Session")
      || result.afterCreate.commandActivePaneTitle !== result.afterCreate.activePaneId
      || !result.afterCreate.commandActivePaneText?.includes("Pane")
      || result.afterCreate.statusPaneTitle !== result.afterCreate.activePaneId
      || !result.afterCreate.statusPaneText?.includes("Pane")
      || (
        result.afterCreate.activePaneId?.length > 18
        && (
          result.afterCreate.focusedPaneBadgeTitle !== result.afterCreate.activePaneId
          || result.afterCreate.focusedPaneBadgeText.includes(result.afterCreate.activePaneId)
          || !result.afterCreate.focusedPaneBadgeText.includes("...")
          || result.afterCreate.commandActivePaneText.includes(result.afterCreate.activePaneId)
          || !result.afterCreate.commandActivePaneText.includes("...")
          || result.afterCreate.statusPaneText.includes(result.afterCreate.activePaneId)
          || !result.afterCreate.statusPaneText.includes("...")
        )
      )
      || (
        result.afterCreate.activeSessionId?.length > 18
        && (
          result.afterCreate.statusSessionText.includes(result.afterCreate.activeSessionId)
          || !result.afterCreate.statusSessionText.includes("...")
          || result.afterCreate.activeSessionListIdText.includes(result.afterCreate.activeSessionId)
          || !result.afterCreate.activeSessionListIdText.includes("...")
        )
      )
      || !Array.isArray(result.afterCreate.quickCommandLabels)
      || result.afterCreate.quickCommandLabels.join("|") !== "pwd|ls -la|git status|node -v|hello"
      || !result.afterCreate.quickCommandTitles.includes("Print the active Node.js version")
      || !result.afterQuickCommandDraft.clicked
      || result.afterQuickCommandDraft.draft !== "node -v"
      || result.afterQuickCommandDraft.kernelDraft !== "node -v"
      || !result.afterQuickCommandDraft.inputFocused
      || !result.afterQuickCommandDraft.cursorAtEnd
      || result.afterQuickCommandDraft.rows !== 1
      || result.afterQuickCommandDraft.rowCount !== "1"
      || result.afterQuickCommandDraft.multiline !== "false"
      || result.afterCreate.savedSessionCount !== 0
      || result.afterCreate.savedPanelCount !== "0"
      || result.afterCreate.savedMatchedCount !== "0"
      || result.afterCreate.savedVisibleCount !== "0"
      || result.afterCreate.savedHiddenCount !== "0"
      || result.afterCreate.savedFiltered !== "false"
      || result.afterCreate.hasSavedFilter
      || !result.afterCreate.screenFollowPressed
      || result.afterCreate.savedItemsRendered > 8
      || (result.afterCreate.savedSessionCount > 8 && !result.afterCreate.hasSavedPagination)
      || !result.afterCreate.hasActiveTitle
      || !result.afterCreate.inputEnabled
      || /default interactive shell is now zsh/i.test(result.afterCreate.terminalScreenTextPreview ?? "")
    ) {
      throw new Error(`Session creation did not settle correctly: ${JSON.stringify(result.afterCreate)}`);
    }

    if (
      !result.afterMultilineCommandDraft.applied
      || result.afterMultilineCommandDraft.rows !== 5
      || result.afterMultilineCommandDraft.rowCount !== "5"
      || result.afterMultilineCommandDraft.multiline !== "true"
      || result.afterMultilineCommandDraft.kernelDraft !== result.afterMultilineCommandDraft.draft
      || !result.afterMultilineCommandDraft.inputFocused
      || !result.afterMultilineCommandDraft.cursorAtEnd
      || result.afterMultilineCommandDraft.height <= result.afterQuickCommandDraft.height
      || result.afterMultilineCommandDraft.dockBottomOverflowPx !== 0
    ) {
      throw new Error(`Command composer multiline draft layout did not settle correctly: ${JSON.stringify(result.afterMultilineCommandDraft)}`);
    }

    if (
      !result.afterCreateMobileLayout.checked
      || result.afterCreateMobileLayout.demoShellActive !== "true"
      || result.afterCreateMobileLayout.demoShellMode !== "terminal"
      || result.afterCreateMobileLayout.launcherPanelVisible !== false
      || result.afterCreateMobileLayout.demoShellColumnCount !== 1
      || result.afterCreateMobileLayout.operationsDeckColumnCount !== 1
      || result.afterCreateMobileLayout.workspaceNavigationMode !== "collapsed"
      || result.afterCreateMobileLayout.workspaceInspectorMode !== "collapsed"
      || !result.afterCreateMobileLayout.hasNavigationDrawer
      || !result.afterCreateMobileLayout.hasInspectorDrawer
      || result.afterCreateMobileLayout.inspectorDrawerOpen !== false
      || result.afterCreateMobileLayout.documentHorizontalOverflow > 1
      || result.afterCreateMobileLayout.demoMainWidth < 430
      || result.afterCreateMobileLayout.screenViewportHeight < 260
      || result.afterCreateMobileLayout.terminalScreenHeight > 650
      || result.afterCreateMobileLayout.commandRegionTop > 900
      || Math.abs(result.afterCreateMobileLayout.terminalComposerGapPx ?? 99) > 1
      || Math.abs(result.afterCreateMobileLayout.terminalInputGapPx ?? 99) > 12
      || !result.afterCreateMobileLayout.terminalComposerBeforeDockStatus
      || !result.afterCreateMobileLayout.terminalComposerBeforeDockStatusDom
      || !result.afterCreateMobileLayout.terminalComposerFirstInDockDom
      || result.afterCreateMobileLayout.terminalComposerTagName !== "TP-TERMINAL-COMMAND-COMPOSER"
      || result.afterCreateMobileLayout.terminalComposerActionIds.join("|") !== "submit|paste|interrupt|enter"
      || result.afterCreateMobileLayout.terminalComposerActionKeyHints.join("|") !== "Enter||Ctrl+C|Enter"
      || result.afterCreateMobileLayout.terminalComposerActionAriaKeyShortcuts.join("|") !== "Enter|||"
      || result.afterCreateMobileLayout.commandInputRows !== 1
      || result.afterCreateMobileLayout.commandInputRowCount !== "1"
      || result.afterCreateMobileLayout.commandInputMultiline !== "false"
      || !result.afterCreateMobileLayout.terminalCommandActionsInsideComposer
      || result.afterCreateMobileLayout.terminalFooterActionCount !== 0
      || !result.afterCreateMobileLayout.commandRegionFollowsScreen
      || !result.afterCreateMobileLayout.terminalTabStripBeforeScreenDom
      || result.afterCreateMobileLayout.terminalTabStripTabCount !== "1"
    ) {
      throw new Error(`Mobile active shell layout did not settle correctly: ${JSON.stringify(result.afterCreateMobileLayout)}`);
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
        || result.afterSavedPagination.showMoreCount !== expectedPaginatedItems - result.afterCreate.savedItemsRendered
        || !result.afterSavedPagination.showMoreText?.includes(String(expectedPaginatedItems - result.afterCreate.savedItemsRendered))
        || !result.afterSavedPagination.collapseText?.includes(String(result.afterCreate.savedItemsRendered))
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
      || result.afterThemeSwitch.themeButtonLabel !== "Light"
      || result.afterThemeSwitch.themeButtonTitle !== "Switch workspace theme to Light."
      || result.afterThemeSwitch.demoShellTheme !== "terminal-platform-light"
      || result.afterThemeSwitch.demoShellBgToken !== "#f6f8fb"
      || result.afterThemeSwitch.demoShellTextColor !== "rgb(23, 32, 51)"
      || result.afterThemeSwitch.demoShellMode !== "terminal"
      || result.afterThemeSwitch.workspaceHeroVisible !== false
      || result.afterThemeSwitch.launcherPanelVisible !== false
      || !String(result.afterThemeSwitch.demoShellColorScheme ?? "").includes("light")
      || result.afterThemeSwitch.screenBgToken !== "#f6f8fb"
      || result.afterThemeSwitch.storedTheme !== "terminal-platform-light"
    ) {
      throw new Error(`Theme switch did not apply to SDK and demo shell elements: ${JSON.stringify(result.afterThemeSwitch)}`);
    }

    if (
      !result.afterDisplaySwitch.clicked
      || result.afterDisplaySwitch.fontScale !== "large"
      || result.afterDisplaySwitch.lineWrap !== false
      || result.afterDisplaySwitch.screenFontScale !== "large"
      || result.afterDisplaySwitch.screenLineWrap !== "false"
      || result.afterDisplaySwitch.activeFontScaleButton !== "large"
      || result.afterDisplaySwitch.largeButtonLabel !== "Large"
      || result.afterDisplaySwitch.largeButtonTitle !== "Large terminal font size is active."
      || result.afterDisplaySwitch.wrapPressed !== "false"
      || result.afterDisplaySwitch.wrapLabel !== "Wrap off"
      || result.afterDisplaySwitch.wrapTitle !== "Enable terminal line wrapping."
      || result.afterDisplaySwitch.wrapNext !== "true"
      || result.afterDisplaySwitch.storedFontScale !== "large"
      || result.afterDisplaySwitch.storedLineWrap !== "false"
    ) {
      throw new Error(`Terminal display preferences did not apply to SDK elements: ${JSON.stringify(result.afterDisplaySwitch)}`);
    }

    if (
      !result.afterTopologyActions.ok
      || !result.afterTopologyActions.splitClicked
      || !result.afterTopologyActions.resizeClicked
      || !result.afterTopologyActions.closePanePrompted
      || result.afterTopologyActions.closePaneDanger !== "true"
      || !result.afterTopologyActions.closePaneTitle?.includes("Confirm closing pane")
      || !result.afterTopologyActions.closePaneConfirmed
      || !result.afterTopologyActions.renameClicked
      || !result.afterTopologyActions.newTabClicked
      || !result.afterTopologyActions.closeTabPrompted
      || result.afterTopologyActions.closeTabDanger !== "true"
      || !result.afterTopologyActions.closeTabTitle?.includes("Confirm closing tab")
      || !result.afterTopologyActions.closeTabConfirmed
      || !result.afterTopologyActions.focusOriginalClicked
      || result.afterTopologyActions.completedEvents < 6
      || result.afterTopologyActions.paneCountAfterSplit <= result.afterTopologyActions.paneCountBefore
      || result.afterTopologyActions.focusedPaneAfterSplit === result.afterTopologyActions.focusedPaneBefore
      || result.afterTopologyActions.splitDirection !== "Split vertical"
      || result.afterTopologyActions.resizeColsAfter <= result.afterTopologyActions.resizeColsBefore
      || result.afterTopologyActions.paneCountAfterClosePrompt !== result.afterTopologyActions.paneCountAfterSplit
      || result.afterTopologyActions.paneCountAfterClosePane !== result.afterTopologyActions.paneCountBefore
      || result.afterTopologyActions.renamedTabTitle !== "Smoke Workspace"
      || result.afterTopologyActions.tabCountAfterNewTab <= result.afterTopologyActions.tabCountBefore
      || result.afterTopologyActions.terminalTabStripTabCountAfterNewTab !== String(result.afterTopologyActions.tabCountAfterNewTab)
      || result.afterTopologyActions.terminalTabStripRenderedAfterNewTab !== result.afterTopologyActions.tabCountAfterNewTab
      || result.afterTopologyActions.terminalTabStripActiveAfterNewTab !== 1
      || result.afterTopologyActions.terminalTabStripCloseButtonsAfterNewTab !== result.afterTopologyActions.tabCountAfterNewTab
      || result.afterTopologyActions.terminalTabStripEnabledCloseButtonsAfterNewTab < 1
      || !result.afterTopologyActions.terminalTabStripKeyboardLeftFocusedOriginal
      || !result.afterTopologyActions.terminalTabStripKeyboardRightFocusedNew
      || result.afterTopologyActions.tabCountAfterCloseTabPrompt !== result.afterTopologyActions.tabCountAfterNewTab
      || result.afterTopologyActions.tabCountAfterCloseTab !== result.afterTopologyActions.tabCountBefore
      || result.afterTopologyActions.terminalTabStripTabCountAfterCloseTab !== String(result.afterTopologyActions.tabCountAfterCloseTab)
      || result.afterTopologyActions.terminalTabStripRenderedAfterCloseTab !== result.afterTopologyActions.tabCountAfterCloseTab
      || result.afterTopologyActions.terminalTabStripCloseButtonsAfterCloseTab !== result.afterTopologyActions.tabCountAfterCloseTab
      || result.afterTopologyActions.terminalTabStripEnabledCloseButtonsAfterCloseTab !== 0
      || result.afterTopologyActions.focusedTabAfterFocus !== result.afterTopologyActions.originalTabId
    ) {
      throw new Error(`Topology controls did not mutate and restore focus correctly: ${JSON.stringify(result.afterTopologyActions)}`);
    }

    if (
      !result.afterSaveLayout.clicked
      || result.afterSaveLayout.savedSessionCount < result.afterSaveLayout.beforeSavedSessionCount
      || result.afterSaveLayout.savedPanelCount !== String(result.afterSaveLayout.savedSessionCount)
      || result.afterSaveLayout.savedMatchedCount !== String(result.afterSaveLayout.savedSessionCount)
      || result.afterSaveLayout.savedVisibleCount !== String(result.afterSaveLayout.savedItemsRendered)
      || result.afterSaveLayout.savedItemsRendered < 1
      || result.afterSaveLayout.savedFiltered !== "false"
      || !result.afterSaveLayout.hasSavedFilter
      || result.afterSaveLayout.firstSavedCanRestore !== "true"
      || result.afterSaveLayout.firstSavedRestoreStatus !== "available"
      || result.afterSaveLayout.firstSavedRestoreDisabled !== false
      || !result.afterSaveLayout.firstSavedRestoreTitle?.includes("Restore saved layout")
      || !result.afterSaveLayout.firstSavedSemanticsCodes?.includes("process_state_not_preserved")
      || !result.afterSaveLayout.firstSavedSemanticsCodes?.includes("screen_buffers_not_replayed")
      || !result.afterSaveLayout.firstSavedSemanticsLabels?.includes("processes restart")
      || !result.afterSaveLayout.firstSavedSemanticsLabels?.includes("no screen replay")
      || !result.afterSaveLayout.deletePrompted
      || result.afterSaveLayout.savedSessionCountAfterDeletePrompt !== result.afterSaveLayout.savedSessionCount
      || result.afterSaveLayout.saveEventDetail?.savedSessionCount !== result.afterSaveLayout.savedSessionCount
    ) {
      throw new Error(`Save layout workflow did not complete: ${JSON.stringify(result.afterSaveLayout)}`);
    }

    if (
      !result.afterSavedSearch.searched
      || result.afterSavedSearch.filtered !== "true"
      || result.afterSavedSearch.savedPanelCount !== String(result.afterSaveLayout.savedSessionCount)
      || result.afterSavedSearch.matchedCount < 1
      || result.afterSavedSearch.itemsRendered < 1
      || result.afterSavedSearch.visibleCount !== String(result.afterSavedSearch.itemsRendered)
      || result.afterSavedSearch.hasPruneHiddenWhileFiltered
      || !result.afterSavedSearch.firstTitle?.toLowerCase().includes("workspace")
      || result.afterSavedSearch.afterClearFiltered !== "false"
      || result.afterSavedSearch.afterClearValue !== ""
      || result.afterSavedSearch.afterClearPanelCount !== String(result.afterSaveLayout.savedSessionCount)
    ) {
      throw new Error(`Saved layout filter did not settle correctly: ${JSON.stringify(result.afterSavedSearch)}`);
    }

    if (
      !result.afterPruneHidden.prompted
      || !result.afterPruneHidden.confirmed
      || result.afterPruneHidden.savedSessionCountBefore <= result.afterPruneHidden.visibleCountBefore
      || result.afterPruneHidden.savedSessionCountAfterPrompt !== result.afterPruneHidden.savedSessionCountBefore
      || result.afterPruneHidden.savedSessionCountAfter !== result.afterPruneHidden.visibleCountBefore
      || result.afterPruneHidden.pruneCount !== result.afterPruneHidden.deletedCount
      || result.afterPruneHidden.pruneKeepLatest !== result.afterPruneHidden.visibleCountBefore
      || !result.afterPruneHidden.pruneButtonText?.includes(String(result.afterPruneHidden.deletedCount))
      || !result.afterPruneHidden.pruneButtonTitle?.includes(`keep the latest ${result.afterPruneHidden.visibleCountBefore}`)
      || !result.afterPruneHidden.confirmButtonText?.includes(String(result.afterPruneHidden.deletedCount))
      || result.afterPruneHidden.eventDetail?.deletedCount !== result.afterPruneHidden.deletedCount
      || result.afterPruneHidden.eventDetail?.keptCount !== result.afterPruneHidden.savedSessionCountAfter
    ) {
      throw new Error(`Saved-session prune hidden workflow did not complete: ${JSON.stringify(result.afterPruneHidden)}`);
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
      !result.afterScreenSearchShortcut.tested
      || !result.afterScreenSearchShortcut.defaultPrevented
      || !result.afterScreenSearchShortcut.searchFocused
      || !result.afterScreenSearchShortcut.selectedExistingQuery
      || !result.afterScreenSearchShortcut.viewportFocusedAfterEscape
      || result.afterScreenSearchShortcut.queryAfterEscape !== ""
      || result.afterScreenSearchShortcut.submittedEvents !== 0
    ) {
      throw new Error(`Terminal screen search shortcut did not stay local: ${JSON.stringify(result.afterScreenSearchShortcut)}`);
    }

    if (
      !result.afterCommand.connectionReady
      || !result.afterCommand.screenFollowPressed
      || !result.afterCommand.screenViewportAtBottom
      || !result.afterCommand.commandInputFocused
      || !result.afterCommand.commandInputEmpty
      || !result.afterCommand.commandCursorAtEnd
      || result.afterCommand.commandHistoryCount < 1
      || !result.afterCommand.commandHistoryLatest?.includes("browser-smoke-ok")
      || !result.afterCommand.historyBadgeText?.includes("history")
      || (!result.afterCommand.sequenceAdvanced && !result.afterCommand.containsCommandOutput)
    ) {
      throw new Error(`Command lane did not advance the focused screen: ${JSON.stringify(result.afterCommand)}`);
    }

    if (
      !result.afterCommandActionFocus.tested
      || !result.afterCommandActionFocus.enterFocused
      || !result.afterCommandActionFocus.enterCursorAtEnd
      || !result.afterCommandActionFocus.interruptFocused
      || !result.afterCommandActionFocus.interruptCursorAtEnd
    ) {
      throw new Error(`Command action buttons did not return focus to the command input: ${JSON.stringify(result.afterCommandActionFocus)}`);
    }

    if (
      !result.afterScreenCopy.clicked
      || result.afterScreenCopy.copiedEvents !== 1
      || result.afterScreenCopy.failedEvents !== 0
      || !result.afterScreenCopy.containsCopiedCommandOutput
      || result.afterScreenCopy.buttonText !== "Copied"
      || result.afterScreenCopy.eventDetail?.lineCount < 1
    ) {
      throw new Error(`Terminal screen copy did not write visible output: ${JSON.stringify(result.afterScreenCopy)}`);
    }

    if (
      !result.afterRecentCommandRecall.clicked
      || !result.afterRecentCommandRecall.recalledDraft?.includes("browser-smoke-ok")
      || !result.afterRecentCommandRecall.sendEnabled
      || !result.afterRecentCommandRecall.inputFocused
      || !result.afterRecentCommandRecall.cursorAtEnd
      || !result.afterRecentCommandRecall.historyBadgeText?.includes("history")
    ) {
      throw new Error(`Recent command recall did not update the draft correctly: ${JSON.stringify(result.afterRecentCommandRecall)}`);
    }

    if (
      !result.afterClipboardPaste.clicked
      || !result.afterClipboardPaste.connectionReady
      || result.afterClipboardPaste.submittedEvents !== 1
      || !result.afterClipboardPaste.inputFocused
      || !result.afterClipboardPaste.cursorAtEnd
      || !result.afterClipboardPaste.containsPasteOutput
      || result.afterClipboardPaste.commandHistoryCount !== result.afterCommand.commandHistoryCount
      || !result.afterClipboardPaste.commandHistoryLatest?.includes("browser-smoke-ok")
      || result.afterClipboardPaste.commandHistoryLatest?.includes("browser-paste-ok")
    ) {
      throw new Error(`Clipboard paste did not route through the command dock correctly: ${JSON.stringify(result.afterClipboardPaste)}`);
    }

    if (
      !result.afterScreenFollowToggle.paused
      || !result.afterScreenFollowToggle.resumed
      || !result.afterScreenFollowToggle.startedFollowing
      || !result.afterScreenFollowToggle.screenViewportAtBottom
    ) {
      throw new Error(`Screen follow controls did not toggle correctly: ${JSON.stringify(result.afterScreenFollowToggle)}`);
    }

    if (
      !result.afterHistoryReplay.recalledDraft?.includes("browser-smoke-ok")
      || result.afterHistoryReplay.restoredDraft !== "draft-before-history-replay"
      || !result.afterHistoryReplay.replayClicked
      || !result.afterHistoryReplay.connectionReady
      || result.afterHistoryReplay.commandHistoryCount < 1
      || !result.afterHistoryReplay.commandHistoryLatest?.includes("browser-smoke-ok")
      || (!result.afterHistoryReplay.sequenceAdvanced && !result.afterHistoryReplay.containsCommandOutput)
    ) {
      throw new Error(`Command history replay did not settle correctly: ${JSON.stringify(result.afterHistoryReplay)}`);
    }

    if (
      !result.afterCommandHistoryClear.clicked
      || result.afterCommandHistoryClear.beforeCount < 1
      || !result.afterCommandHistoryClear.firstClickArmed
      || !result.afterCommandHistoryClear.firstClickLabel?.includes("Confirm clear")
      || result.afterCommandHistoryClear.afterFirstCount !== result.afterCommandHistoryClear.beforeCount
      || result.afterCommandHistoryClear.clearedEventsAfterFirst !== 0
      || result.afterCommandHistoryClear.afterCount !== 0
      || result.afterCommandHistoryClear.clearedEvents !== 1
      || result.afterCommandHistoryClear.clearButtonDisabled !== true
      || result.afterCommandHistoryClear.finalConfirming !== false
      || !result.afterCommandHistoryClear.historyBadgeText?.startsWith("0 ")
    ) {
      throw new Error(`Command history clear did not settle correctly: ${JSON.stringify(result.afterCommandHistoryClear)}`);
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
    await send("Browser.grantPermissions", {
      origin: new URL(browserUrl).origin,
      permissions: ["clipboardReadWrite", "clipboardSanitizedWrite"],
    }).catch(() => undefined);
    await send("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 1100,
      deviceScaleFactor: 1,
      mobile: false,
    });

    await sleep(3000);
    await installBrowserSmokeHelpers(send);

    const before = await evaluate(send, `(() => ({
      bodyText: document.body.innerText,
      hasWorkspaceShell: Boolean(document.querySelector('[data-testid="terminal-demo-shell"]')),
      hasStartDefaultShell: Boolean(document.querySelector('[data-testid="start-default-shell"]')),
      sessionCount: window.terminalDemoDebug?.getState?.()?.catalog?.sessions?.length ?? 0,
      demoAutoStartSession: new URLSearchParams(window.location.search).get('demoAutoStartSession'),
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
      const demoShell = document.querySelector('[data-testid="terminal-demo-shell"]') ?? null;
      const demoSidebar = demoShell?.querySelector('.shell__sidebar') ?? null;
      const demoMain = demoShell?.querySelector('.shell__main') ?? null;
      const workspaceHostHeader = document.querySelector('.panel__header--workspace') ?? null;
      const workspaceHostSlot = document.querySelector('[data-testid="terminal-workspace-host"]') ?? null;
      const workspaceHost = document.querySelector('tp-terminal-workspace');
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const workspaceFrame = workspaceRoot?.querySelector('[part="workspace"]') ?? null;
      const statusRoot = workspaceRoot?.querySelector('tp-terminal-status-bar')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const sessionListRoot = workspaceRoot?.querySelector('tp-terminal-session-list')?.shadowRoot ?? null;
      const savedRoot = workspaceRoot?.querySelector('tp-terminal-saved-sessions')?.shadowRoot ?? null;
      const savedPanel = savedRoot?.querySelector('[data-testid="tp-saved-sessions"]') ?? null;
      const toolbarRoot = workspaceRoot?.querySelector('tp-terminal-toolbar')?.shadowRoot ?? null;
      const paneTreeRoot = workspaceRoot?.querySelector('tp-terminal-pane-tree')?.shadowRoot ?? null;
      const layoutRoot = workspaceRoot?.querySelector('[data-testid="tp-workspace-layout"]') ?? null;
      const navigationDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-navigation-drawer"]') ?? null;
      const operationsDeck = workspaceRoot?.querySelector('[data-testid="tp-workspace-operations-deck"]') ?? null;
      const terminalColumn = workspaceRoot?.querySelector('[data-testid="tp-workspace-terminal-column"]') ?? null;
      const inspectorColumn = workspaceRoot?.querySelector('[data-testid="tp-workspace-inspector-column"]') ?? null;
      const inspectorDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-inspector-drawer"]') ?? null;
      const commandRegion = workspaceRoot?.querySelector('[data-testid="tp-workspace-command-region"]') ?? null;
      const workspaceContent = workspaceRoot?.querySelector('[part="content"]') ?? null;
      const tabStripHost = workspaceRoot?.querySelector('tp-terminal-tab-strip') ?? null;
      const tabStripRoot = tabStripHost?.shadowRoot ?? null;
      const tabStripPanel = tabStripRoot?.querySelector('[data-testid="tp-terminal-tab-strip"]') ?? null;
      const terminalTabs = [...(tabStripRoot?.querySelectorAll('[data-testid="tp-terminal-tab"]') ?? [])];
      const terminalTabCloseButtons =
        [...(tabStripRoot?.querySelectorAll('[data-testid="tp-terminal-tab-close"]') ?? [])];
      const terminalNewTab = tabStripRoot?.querySelector('[data-testid="tp-terminal-new-tab"]') ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const paneTreeHost = workspaceRoot?.querySelector('tp-terminal-pane-tree') ?? null;
      const sessionListHost = workspaceRoot?.querySelector('tp-terminal-session-list') ?? null;
      const savedSessionsHost = workspaceRoot?.querySelector('tp-terminal-saved-sessions') ?? null;
      const commandDockHost = workspaceRoot?.querySelector('tp-terminal-command-dock') ?? null;
      const screenRoot = screenHost?.shadowRoot ?? null;
      const screenFollow = screenRoot?.querySelector('[data-testid="tp-screen-follow"]') ?? null;
      const screenSearch = screenRoot?.querySelector('[data-testid="tp-screen-search"]') ?? null;
      const screenCopy = screenRoot?.querySelector('[data-testid="tp-screen-copy"]') ?? null;
      const screenInputStatus = screenRoot?.querySelector('[data-testid="tp-screen-input-status"]') ?? null;
      const screenViewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      const screenPanel = screenRoot?.querySelector('[data-testid="tp-terminal-screen"]') ?? null;
      const saveLayout = commandRoot?.querySelector('[data-testid="tp-save-layout"]') ?? null;
      const pasteClipboard = commandRoot?.querySelector('[data-testid="tp-paste-clipboard"]') ?? null;
      const quickCommands = [...(commandRoot?.querySelectorAll('[data-testid="tp-quick-command"]') ?? [])];
      const commandDockPanel = commandRoot?.querySelector('[data-testid="tp-command-dock"]') ?? null;
      const commandActivePane = commandRoot?.querySelector('[data-testid="tp-command-active-pane"]') ?? null;
      const commandInputStatus = commandRoot?.querySelector('[data-testid="tp-command-input-status"]') ?? null;
      const commandActionButtons = [
        commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null,
        commandRoot?.querySelector('[data-testid="tp-paste-clipboard"]') ?? null,
        commandRoot?.querySelector('[data-testid="tp-send-interrupt"]') ?? null,
        commandRoot?.querySelector('[data-testid="tp-send-enter"]') ?? null,
      ];
      const statusSession = statusRoot?.querySelector('[data-testid="tp-status-session-id"]') ?? null;
      const statusPane = statusRoot?.querySelector('[data-testid="tp-status-pane-id"]') ?? null;
      const activeSessionListId = sessionListRoot
        ?.querySelector('[data-active="true"] [data-testid="tp-session-id"]') ?? null;
      const paneTreePanel = paneTreeRoot?.querySelector('[data-testid="tp-pane-tree"]') ?? null;
      const topologyStatus = paneTreeRoot?.querySelector('[data-testid="tp-topology-status"]') ?? null;
      const topologyMutationControls = [
        paneTreeRoot?.querySelector('[data-testid="tp-new-tab"]') ?? null,
        paneTreeRoot?.querySelector('[data-testid="tp-split-right"]') ?? null,
        paneTreeRoot?.querySelector('[data-testid="tp-split-down"]') ?? null,
        paneTreeRoot?.querySelector('[data-testid="tp-rename-tab"]') ?? null,
        paneTreeRoot?.querySelector('[data-testid="tp-resize-wider"]') ?? null,
      ];
      const terminalScreenText = debug?.attachedSession?.focused_screen?.surface?.lines
        ? debug.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : (screenRoot?.querySelector('[part=\"screen-lines\"]')?.textContent?.trim() ?? null);
      const activeTitle = document.querySelector('.workspace-summary__title')?.textContent?.trim() ?? null;
      const focusedPaneBadge = document.querySelector('[data-testid="workspace-focused-pane-badge"]') ?? null;
      const input = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const screenRect = screenHost?.getBoundingClientRect() ?? null;
      const tabStripRect = tabStripHost?.getBoundingClientRect() ?? null;
      const screenViewportRect = screenViewport?.getBoundingClientRect() ?? null;
      const commandRegionRect = commandRegion?.getBoundingClientRect() ?? null;
      const commandComposer = commandRoot?.querySelector('[part="composer"]') ?? null;
      const commandComposerPrompt = commandComposer?.querySelector('[part="prompt"]') ?? null;
      const commandComposerInput = commandComposer?.querySelector('[part="input"]') ?? null;
      const commandComposerRect = commandComposer?.getBoundingClientRect() ?? null;
      const commandDockPanelRect = commandDockPanel?.getBoundingClientRect() ?? null;
      const dockHeaderRect = commandRoot?.querySelector('.dock-header')?.getBoundingClientRect() ?? null;
      const inspectorDrawerSummary = inspectorDrawer?.querySelector('summary') ?? null;
      inspectorDrawerSummary?.click();
      const inspectorDrawerOpenedAfterClick = inspectorDrawer?.hasAttribute('open') ?? false;
      const paneTreeVisibleAfterDrawerOpen = Boolean(
        paneTreeHost
        && paneTreeHost.getBoundingClientRect().height > 0
        && paneTreeHost.getBoundingClientRect().width > 0
      );
      inspectorDrawerSummary?.click();
      const inspectorDrawerClosedAfterToggle = inspectorDrawer ? !inspectorDrawer.hasAttribute('open') : false;
      const navigationDrawerSummary = navigationDrawer?.querySelector('summary') ?? null;
      navigationDrawerSummary?.click();
      const navigationDrawerOpenedAfterClick = navigationDrawer?.hasAttribute('open') ?? false;
      const navigationVisibleAfterDrawerOpen = Boolean(
        sessionListHost
        && savedSessionsHost
        && sessionListHost.getBoundingClientRect().height > 0
        && savedSessionsHost.getBoundingClientRect().height > 0
      );
      navigationDrawerSummary?.click();
      const navigationDrawerClosedAfterToggle = navigationDrawer ? !navigationDrawer.hasAttribute('open') : false;
      return {
        hasReady: debug?.connection?.state === 'ready',
        hasError: debug?.connection?.state === 'error',
        activeSessionId: debug?.selection?.activeSessionId ?? null,
        activePaneId: debug?.selection?.activePaneId ?? debug?.attachedSession?.focused_screen?.pane_id ?? null,
        savedSessionCount: debug?.catalog?.savedSessions?.length ?? 0,
        savedPanelCount: savedPanel?.getAttribute('data-saved-count') ?? null,
        savedMatchedCount: savedPanel?.getAttribute('data-matched-count') ?? null,
        savedVisibleCount: savedPanel?.getAttribute('data-visible-count') ?? null,
        savedHiddenCount: savedPanel?.getAttribute('data-hidden-count') ?? null,
        savedFiltered: savedPanel?.getAttribute('data-filtered') ?? null,
        savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        hasSavedPagination: Boolean(savedRoot?.querySelector('[part="show-more"]')),
        hasSavedFilter: Boolean(savedRoot?.querySelector('[data-testid="tp-saved-session-filter"]')),
        healthPhase: debug?.attachedSession?.health?.phase ?? null,
        demoShellActive: demoShell?.getAttribute('data-has-active-session') ?? null,
        demoShellMode: demoShell?.getAttribute('data-shell-mode') ?? null,
        workspaceHeroVisible: Boolean(demoShell?.querySelector('[data-testid="terminal-demo-workspace-hero"]')),
        launcherPanelVisible: Boolean(demoSidebar),
        demoShellColumnCount: demoShell
          ? getComputedStyle(demoShell).gridTemplateColumns.split(' ').filter(Boolean).length
          : 0,
        demoMainWidth: Math.round(demoMain?.getBoundingClientRect().width ?? 0),
        workspaceHostWidth: Math.round(workspaceHostSlot?.getBoundingClientRect().width ?? 0),
        workspaceHostTopOffset: demoMain && workspaceHostSlot
          ? Math.round(workspaceHostSlot.getBoundingClientRect().top - demoMain.getBoundingClientRect().top)
          : null,
        workspaceHostHeaderDisplay: workspaceHostHeader ? getComputedStyle(workspaceHostHeader).display : null,
        workspaceContentWidth: Math.round(workspaceContent?.getBoundingClientRect().width ?? 0),
        terminalColumnHeight: Math.round(terminalColumn?.getBoundingClientRect().height ?? 0),
        screenViewportHeight: Math.round(screenViewport?.getBoundingClientRect().height ?? 0),
        workspacePanelShadow: workspaceFrame
          ? getComputedStyle(workspaceFrame).getPropertyValue('--tp-shadow-panel').trim()
          : null,
        documentHorizontalOverflow: Math.max(
          0,
          document.documentElement.scrollWidth - document.documentElement.clientWidth,
        ),
        focusedSequence: debug?.attachedSession?.focused_screen?.sequence != null
          ? String(debug.attachedSession.focused_screen.sequence)
          : null,
        hasScreenFollowControls: Boolean(screenFollow && screenRoot?.querySelector('[data-testid="tp-screen-scroll-latest"]')),
        hasScreenSearchControls: Boolean(screenSearch),
        hasScreenCopyControl: Boolean(screenCopy && !screenCopy.disabled),
        hasScreenDirectInput: Boolean(
          screenViewport
          && screenViewport.getAttribute('tabindex') === '0'
          && screenPanel?.getAttribute('data-direct-input') === 'true'
          && screenPanel?.getAttribute('data-input-capability') === 'known'
          && screenPanel?.getAttribute('data-input-status') === 'ready'
        ),
        screenInputStatus: screenInputStatus?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        screenInputTone: screenInputStatus?.getAttribute('data-input-tone') ?? null,
        screenInputTitle: screenInputStatus?.getAttribute('title') ?? null,
        screenPlacement: screenPanel?.getAttribute('data-placement') ?? null,
        screenVisibleText: screenPanel?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        hasPasteClipboardControl: Boolean(pasteClipboard && !pasteClipboard.disabled),
        hasSaveLayoutControl: Boolean(saveLayout && !saveLayout.disabled),
        hasTopologyControls: Boolean(
          paneTreeRoot?.querySelector('[data-testid="tp-new-tab"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-split-right"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-split-down"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-rename-tab"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-close-tab"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-pane-size"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-resize-wider"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-pane-node"]')
          && paneTreeRoot?.querySelector('[data-testid="tp-close-pane"]')
        ),
        topologyStatus: topologyStatus?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        topologyStatusTitle: topologyStatus?.getAttribute('title') ?? null,
        topologyCapabilityStatus: paneTreePanel?.getAttribute('data-capability-status') ?? null,
        topologyStatusTone: paneTreePanel?.getAttribute('data-topology-status') ?? null,
        topologyCanMutateLayout: paneTreePanel?.getAttribute('data-layout-write') ?? null,
        hasEnabledTopologyMutationControls: topologyMutationControls.every((control) => control && !control.disabled),
        hasDisplayControls: Boolean(
          toolbarRoot?.querySelector('[data-testid="tp-font-scale-option"][data-font-scale="large"]')
          && toolbarRoot?.querySelector('[data-testid="tp-line-wrap-option"]')
        ),
        commandDockCanWrite: commandDockPanel?.getAttribute('data-command-input') ?? null,
        commandDockInputCapability: commandDockPanel?.getAttribute('data-input-capability') ?? null,
        commandDockCanSave: commandDockPanel?.getAttribute('data-save-layout') ?? null,
        commandDockSaveCapability: commandDockPanel?.getAttribute('data-save-capability') ?? null,
        commandDockPlacement: commandDockPanel?.getAttribute('data-placement') ?? null,
        terminalComposerGapPx: screenRect && commandRegionRect
          ? Math.round(commandRegionRect.top - screenRect.bottom)
          : null,
        terminalInputGapPx: screenViewportRect && commandComposerRect
          ? Math.round(commandComposerRect.top - screenViewportRect.bottom)
          : null,
        terminalDockBottomOverflowPx: commandDockPanelRect
          ? Math.max(0, Math.round(commandDockPanelRect.bottom - window.innerHeight))
          : null,
        terminalComposerBeforeDockStatus: commandComposerRect && dockHeaderRect
          ? commandComposerRect.top <= dockHeaderRect.top
          : false,
        terminalComposerBeforeDockStatusDom: Boolean(
          commandComposer
          && commandInputStatus
          && (commandComposer.compareDocumentPosition(commandInputStatus) & Node.DOCUMENT_POSITION_FOLLOWING)
        ),
        terminalComposerFirstInDockDom: commandDockPanel?.firstElementChild === commandComposer,
        terminalComposerTagName: commandComposer?.tagName ?? null,
        terminalComposerPromptPart: commandComposerPrompt?.getAttribute('part') ?? null,
        terminalComposerInputPart: commandComposerInput?.getAttribute('part') ?? null,
        terminalComposerActionParts: commandActionButtons.map((button) => button?.getAttribute('part') ?? null),
        terminalComposerActionIds: commandActionButtons.map((button) => button?.getAttribute('data-action') ?? ''),
        terminalComposerActionKeyHints: commandActionButtons.map((button) => button?.getAttribute('data-key-hint') ?? ''),
        terminalComposerActionAriaKeyShortcuts: commandActionButtons.map((button) =>
          button?.getAttribute('aria-keyshortcuts') ?? '',
        ),
        commandInputRows: input?.rows ?? null,
        commandInputRowCount: input?.getAttribute('data-row-count') ?? null,
        commandInputMultiline: input?.getAttribute('data-multiline') ?? null,
        commandInputHeight: Math.round(input?.getBoundingClientRect().height ?? 0),
        commandComposerMinRows: commandComposer?.minRows ?? null,
        commandComposerMaxRows: commandComposer?.maxRows ?? null,
        terminalCommandActionsInsideComposer: Boolean(
          commandComposer
          && commandActionButtons.every((button) => button && commandComposer.contains(button))
        ),
        terminalFooterActionCount: commandRoot?.querySelectorAll('.dock-footer .actions button')?.length ?? 0,
        commandActionLabels: commandActionButtons.map((button) => button?.textContent?.replace(/\\s+/g, ' ').trim() ?? null),
        commandActionAriaLabels: commandActionButtons.map((button) => button?.getAttribute('aria-label') ?? null),
        commandDockVisibleText: commandDockPanel?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        commandInputFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, input) === true,
        saveLayoutTitle: saveLayout?.getAttribute('title') ?? null,
        focusedPaneBadgeText: focusedPaneBadge?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        focusedPaneBadgeTitle: focusedPaneBadge?.getAttribute('title') ?? null,
        commandActivePaneText: commandActivePane?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        commandActivePaneTitle: commandActivePane?.getAttribute('title') ?? null,
        statusSessionText: statusSession?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        statusSessionTitle: statusSession?.getAttribute('title') ?? null,
        statusPaneText: statusPane?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        statusPaneTitle: statusPane?.getAttribute('title') ?? null,
        activeSessionListIdText: activeSessionListId?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        activeSessionListIdTitle: activeSessionListId?.getAttribute('title') ?? null,
        commandInputStatus: commandInputStatus?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        commandInputStatusTitle: commandInputStatus?.getAttribute('title') ?? null,
        quickCommandLabels: quickCommands.map((button) => button.textContent?.replace(/\\s+/g, ' ').trim() ?? ''),
        quickCommandTitles: quickCommands.map((button) => button.getAttribute('title')),
        screenFollowPressed: screenFollow?.getAttribute('aria-pressed') === 'true',
        screenViewportAtBottom: screenViewport
          ? screenViewport.scrollHeight - screenViewport.scrollTop - screenViewport.clientHeight <= 2
          : false,
        workspaceLayout: layoutRoot?.getAttribute('data-layout') ?? null,
        workspaceNavigationMode: layoutRoot?.getAttribute('data-navigation-mode') ?? null,
        hasOperationsDeck: Boolean(layoutRoot && operationsDeck),
        hasNavigationDrawer: Boolean(navigationDrawer),
        navigationDrawerOpen: navigationDrawer?.hasAttribute('open') ?? null,
        navigationDrawerOpenedAfterClick,
        navigationDrawerClosedAfterToggle,
        navigationVisibleAfterDrawerOpen,
        workspaceInspectorMode: operationsDeck?.getAttribute('data-inspector-mode') ?? null,
        hasInspectorDrawer: Boolean(inspectorDrawer),
        inspectorDrawerOpen: inspectorDrawer?.hasAttribute('open') ?? null,
        inspectorDrawerOpenedAfterClick,
        inspectorDrawerClosedAfterToggle,
        paneTreeVisibleAfterDrawerOpen,
        operationsDeckColumnCount: operationsDeck
          ? getComputedStyle(operationsDeck).gridTemplateColumns.split(' ').filter(Boolean).length
          : 0,
        commandRegionPosition: commandRegion ? getComputedStyle(commandRegion).position : null,
        screenInTerminalColumn: Boolean(terminalColumn && screenHost && terminalColumn.contains(screenHost)),
        commandDockInCommandRegion: Boolean(commandRegion && commandDockHost && commandRegion.contains(commandDockHost)),
        topologyInInspectorColumn: Boolean(inspectorColumn && paneTreeHost && inspectorColumn.contains(paneTreeHost)),
        topologyInInspectorDrawer: Boolean(inspectorDrawer && paneTreeHost && inspectorDrawer.contains(paneTreeHost)),
        terminalTabStripInTerminalColumn: Boolean(terminalColumn && tabStripHost && terminalColumn.contains(tabStripHost)),
        terminalTabStripBeforeScreen: Boolean(
          terminalColumn
          && tabStripHost
          && screenHost
          && [...terminalColumn.children].indexOf(tabStripHost) > -1
          && [...terminalColumn.children].indexOf(tabStripHost) < [...terminalColumn.children].indexOf(screenHost)
        ),
        terminalTabStripGapPx: tabStripRect && screenRect
          ? Math.round(screenRect.top - tabStripRect.bottom)
          : null,
        terminalTabStripTabCount: tabStripPanel?.getAttribute('data-tab-count') ?? null,
        terminalTabStripRenderedTabs: terminalTabs.length,
        terminalTabStripActiveTabs: terminalTabs.filter((tab) => tab.getAttribute('aria-pressed') === 'true').length,
        terminalTabStripCloseButtons: terminalTabCloseButtons.length,
        terminalTabStripEnabledCloseButtons: terminalTabCloseButtons.filter((button) => !button.disabled).length,
        terminalTabStripNewTabEnabled: Boolean(terminalNewTab && !terminalNewTab.disabled),
        screenPrecedesCommandDock: Boolean(
          terminalColumn
          && screenHost
          && commandRegion
          && [...terminalColumn.children].indexOf(screenHost) > -1
          && [...terminalColumn.children].indexOf(screenHost) < [...terminalColumn.children].indexOf(commandRegion)
        ),
        hasScreen: Boolean(terminalScreenText),
        terminalScreenTextPreview: terminalScreenText?.slice(0, 240) ?? null,
        hasStatusBar: Boolean(statusRoot?.querySelector('[part="status-bar"]')),
        hasCommandDock: Boolean(commandRoot?.querySelector('[part="command-dock"]')),
        hasActiveTitle: Boolean(activeTitle && activeTitle !== 'Pick a session to inspect'),
        inputEnabled: Boolean(input && !input.disabled),
      };
    })()`);

    await send("Emulation.setDeviceMetricsOverride", {
      width: 500,
      height: 900,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await sleep(300);
    const afterCreateMobileLayout = await evaluate(send, `(() => {
      const demoShell = document.querySelector('[data-testid="terminal-demo-shell"]') ?? null;
      const demoSidebar = demoShell?.querySelector('.shell__sidebar') ?? null;
      const demoMain = demoShell?.querySelector('.shell__main') ?? null;
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const layoutRoot = workspaceRoot?.querySelector('[data-testid="tp-workspace-layout"]') ?? null;
      const operationsDeck = workspaceRoot?.querySelector('[data-testid="tp-workspace-operations-deck"]') ?? null;
      const navigationDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-navigation-drawer"]') ?? null;
      const inspectorDrawer = workspaceRoot?.querySelector('[data-testid="tp-workspace-inspector-drawer"]') ?? null;
      const terminalColumn = workspaceRoot?.querySelector('[data-testid="tp-workspace-terminal-column"]') ?? null;
      const tabStripHost = workspaceRoot?.querySelector('tp-terminal-tab-strip') ?? null;
      const tabStripRoot = tabStripHost?.shadowRoot ?? null;
      const tabStripPanel = tabStripRoot?.querySelector('[data-testid="tp-terminal-tab-strip"]') ?? null;
      const terminalScreen = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const screenRoot = terminalScreen?.shadowRoot ?? null;
      const screenViewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      const commandRegion = workspaceRoot?.querySelector('[data-testid="tp-workspace-command-region"]') ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const commandDockPanel = commandRoot?.querySelector('[data-testid="tp-command-dock"]') ?? null;
      const commandComposer = commandRoot?.querySelector('[part="composer"]') ?? null;
      const commandInputStatus = commandRoot?.querySelector('[data-testid="tp-command-input-status"]') ?? null;
      const commandActionButtons = [
        commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null,
        commandRoot?.querySelector('[data-testid="tp-paste-clipboard"]') ?? null,
        commandRoot?.querySelector('[data-testid="tp-send-interrupt"]') ?? null,
        commandRoot?.querySelector('[data-testid="tp-send-enter"]') ?? null,
      ];
      const terminalScreenRect = terminalScreen?.getBoundingClientRect() ?? null;
      const screenViewportRect = screenViewport?.getBoundingClientRect() ?? null;
      const commandRegionRect = commandRegion?.getBoundingClientRect() ?? null;
      const commandDockPanelRect = commandDockPanel?.getBoundingClientRect() ?? null;
      const commandComposerRect = commandComposer?.getBoundingClientRect() ?? null;
      const dockHeaderRect = commandRoot?.querySelector('.dock-header')?.getBoundingClientRect() ?? null;
      return {
        checked: Boolean(demoShell && demoMain && operationsDeck),
        demoShellActive: demoShell?.getAttribute('data-has-active-session') ?? null,
        demoShellMode: demoShell?.getAttribute('data-shell-mode') ?? null,
        launcherPanelVisible: Boolean(demoSidebar),
        demoShellColumnCount: demoShell
          ? getComputedStyle(demoShell).gridTemplateColumns.split(' ').filter(Boolean).length
          : 0,
        operationsDeckColumnCount: operationsDeck
          ? getComputedStyle(operationsDeck).gridTemplateColumns.split(' ').filter(Boolean).length
          : 0,
        workspaceNavigationMode: layoutRoot?.getAttribute('data-navigation-mode') ?? null,
        workspaceInspectorMode: operationsDeck?.getAttribute('data-inspector-mode') ?? null,
        hasNavigationDrawer: Boolean(navigationDrawer),
        hasInspectorDrawer: Boolean(inspectorDrawer),
        inspectorDrawerOpen: inspectorDrawer?.hasAttribute('open') ?? null,
        demoMainWidth: Math.round(demoMain?.getBoundingClientRect().width ?? 0),
        documentHorizontalOverflow: Math.max(
          0,
          document.documentElement.scrollWidth - document.documentElement.clientWidth,
        ),
        terminalScreenHeight: Math.round(terminalScreen?.getBoundingClientRect().height ?? 0),
        screenViewportHeight: Math.round(screenViewport?.getBoundingClientRect().height ?? 0),
        commandRegionTop: Math.round(commandRegion?.getBoundingClientRect().top ?? 0),
        terminalComposerGapPx: terminalScreenRect && commandRegionRect
          ? Math.round(commandRegionRect.top - terminalScreenRect.bottom)
          : null,
        terminalInputGapPx: screenViewportRect && commandComposerRect
          ? Math.round(commandComposerRect.top - screenViewportRect.bottom)
          : null,
        terminalDockBottomOverflowPx: commandDockPanelRect
          ? Math.max(0, Math.round(commandDockPanelRect.bottom - window.innerHeight))
          : null,
        terminalComposerBeforeDockStatus: commandComposerRect && dockHeaderRect
          ? commandComposerRect.top <= dockHeaderRect.top
          : false,
        terminalComposerBeforeDockStatusDom: Boolean(
          commandComposer
          && commandInputStatus
          && (commandComposer.compareDocumentPosition(commandInputStatus) & Node.DOCUMENT_POSITION_FOLLOWING)
        ),
        terminalComposerFirstInDockDom: commandDockPanel?.firstElementChild === commandComposer,
        terminalComposerTagName: commandComposer?.tagName ?? null,
        terminalComposerActionIds: commandActionButtons.map((button) => button?.getAttribute('data-action') ?? ''),
        terminalComposerActionKeyHints: commandActionButtons.map((button) => button?.getAttribute('data-key-hint') ?? ''),
        terminalComposerActionAriaKeyShortcuts: commandActionButtons.map((button) =>
          button?.getAttribute('aria-keyshortcuts') ?? '',
        ),
        commandInputRows: commandRoot?.querySelector('[data-testid="tp-command-input"]')?.rows ?? null,
        commandInputRowCount: commandRoot?.querySelector('[data-testid="tp-command-input"]')?.getAttribute('data-row-count') ?? null,
        commandInputMultiline: commandRoot?.querySelector('[data-testid="tp-command-input"]')?.getAttribute('data-multiline') ?? null,
        terminalCommandActionsInsideComposer: Boolean(
          commandComposer
          && commandActionButtons.every((button) => button && commandComposer.contains(button))
        ),
        terminalFooterActionCount: commandRoot?.querySelectorAll('.dock-footer .actions button')?.length ?? 0,
        commandRegionFollowsScreen: Boolean(
          terminalScreen
          && commandRegion
          && terminalScreen.getBoundingClientRect().bottom <= commandRegion.getBoundingClientRect().top
        ),
        terminalTabStripBeforeScreenDom: Boolean(
          terminalColumn
          && tabStripHost
          && terminalScreen
          && [...terminalColumn.children].indexOf(tabStripHost) > -1
          && [...terminalColumn.children].indexOf(tabStripHost) < [...terminalColumn.children].indexOf(terminalScreen)
        ),
        terminalTabStripTabCount: tabStripPanel?.getAttribute('data-tab-count') ?? null,
      };
    })()`);
    await send("Emulation.setDeviceMetricsOverride", {
      width: 1440,
      height: 1100,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await sleep(300);

    const initialSequence = afterCreate.focusedSequence;
    const afterQuickCommandDraft = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const quickCommand = [...(commandRoot?.querySelectorAll('[data-testid="tp-quick-command"]') ?? [])]
        .find((button) => button.textContent?.trim() === 'node -v') ?? null;
      if (!textarea || !quickCommand) {
        return {
          clicked: false,
          reason: !textarea ? 'textarea missing' : 'node quick command missing',
          draft: textarea?.value ?? null,
          kernelDraft: null,
          inputFocused: false,
          cursorAtEnd: false,
        };
      }

      quickCommand.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const debug = window.terminalDemoDebug?.getState?.();
      const paneId = debug?.selection?.activePaneId ?? debug?.attachedSession?.focused_screen?.pane_id ?? null;
      return {
        clicked: true,
        draft: textarea.value,
        kernelDraft: paneId ? (debug?.drafts?.[paneId] ?? null) : null,
        inputFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true,
        cursorAtEnd: textarea.selectionStart === textarea.value.length && textarea.selectionEnd === textarea.value.length,
        rows: textarea.rows,
        rowCount: textarea.getAttribute('data-row-count'),
        multiline: textarea.getAttribute('data-multiline'),
        height: Math.round(textarea.getBoundingClientRect().height),
      };
    })()`);
    const multilineCommandDraft = "cat <<'EOF' > smoke.sh\nprintf one\nprintf two\nprintf three\nprintf four\nprintf five\nEOF";
    const afterMultilineCommandDraft = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const commandDockPanel = commandRoot?.querySelector('[data-testid="tp-command-dock"]') ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const draft = ${JSON.stringify(multilineCommandDraft)};
      if (!textarea) {
        return {
          applied: false,
          reason: 'textarea missing',
          draft: null,
          kernelDraft: null,
          rows: null,
          rowCount: null,
          multiline: null,
          inputFocused: false,
          cursorAtEnd: false,
          height: 0,
          dockBottomOverflowPx: null,
        };
      }

      const descriptor = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value');
      descriptor?.set?.call(textarea, draft);
      textarea.setSelectionRange(draft.length, draft.length);
      textarea.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const debug = window.terminalDemoDebug?.getState?.();
      const paneId = debug?.selection?.activePaneId ?? debug?.attachedSession?.focused_screen?.pane_id ?? null;
      return {
        applied: true,
        draft: textarea.value,
        kernelDraft: paneId ? (debug?.drafts?.[paneId] ?? null) : null,
        rows: textarea.rows,
        rowCount: textarea.getAttribute('data-row-count'),
        multiline: textarea.getAttribute('data-multiline'),
        inputFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true,
        cursorAtEnd: textarea.selectionStart === textarea.value.length && textarea.selectionEnd === textarea.value.length,
        height: Math.round(textarea.getBoundingClientRect().height),
        dockBottomOverflowPx: commandDockPanel
          ? Math.max(0, Math.round(commandDockPanel.getBoundingClientRect().bottom - window.innerHeight))
          : null,
      };
    })()`);
    let afterScreenSearch = {
      searched: false,
      reason: "deferred until command output is present",
      matchCount: 0,
      hasHighlights: false,
      hasActiveHighlight: false,
      nextClicked: false,
    };
    let afterScreenSearchShortcut = {
      tested: false,
      reason: "deferred until command output is present",
      defaultPrevented: false,
      searchFocused: false,
      selectedExistingQuery: false,
      viewportFocusedAfterEscape: false,
      queryAfterEscape: null,
      submittedEvents: 0,
    };
    let afterScreenCopy = {
      clicked: false,
      reason: "deferred until command output is present",
      copiedEvents: 0,
      failedEvents: 0,
      containsCopiedCommandOutput: false,
      buttonText: null,
      eventDetail: null,
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
          showMoreCount: 0,
          showMoreText: null,
          showMoreTitle: null,
          collapseText: null,
          collapseTitle: null,
          summaryText: savedRoot?.querySelector('[part="list-summary"]')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        };
      }
      const showMoreCount = Number(showMoreButton.getAttribute('data-show-count') ?? '0');
      const showMoreText = showMoreButton.textContent?.replace(/\\s+/g, ' ').trim() ?? null;
      const showMoreTitle = showMoreButton.getAttribute('title');
      showMoreButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const collapseButton = savedRoot?.querySelector('[part="collapse"]') ?? null;
      return {
        clicked: true,
        savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        hasCollapse: Boolean(collapseButton),
        showMoreCount,
        showMoreText,
        showMoreTitle,
        collapseText: collapseButton?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        collapseTitle: collapseButton?.getAttribute('title') ?? null,
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
      const demoShell = document.querySelector('[data-testid="terminal-demo-shell"]') ?? null;
      const heroTitle = document.querySelector('.hero__title') ?? null;
      const themeButton = [...(toolbarRoot?.querySelectorAll('[part="theme-option"]') ?? [])]
        .find((button) => button.getAttribute('data-theme-id') === 'terminal-platform-light') ?? null;
      if (!themeButton) {
        return {
          clicked: false,
          reason: 'light theme button missing',
          themeId: debug?.theme?.themeId ?? null,
        };
      }

      const themeButtonLabel = themeButton.getAttribute('data-theme-label')
        ?? themeButton.textContent?.replace(/\\s+/g, ' ').trim()
        ?? null;
      const themeButtonTitle = themeButton.getAttribute('title');

      themeButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const demoShellStyle = demoShell ? getComputedStyle(demoShell) : null;
      const heroTitleStyle = heroTitle ? getComputedStyle(heroTitle) : null;

      return {
        clicked: true,
        themeId: window.terminalDemoDebug?.getState?.()?.theme?.themeId ?? null,
        demoShellTheme: demoShell?.getAttribute('data-workspace-theme') ?? null,
        demoShellMode: demoShell?.getAttribute('data-shell-mode') ?? null,
        workspaceHeroVisible: Boolean(demoShell?.querySelector('[data-testid="terminal-demo-workspace-hero"]')),
        launcherPanelVisible: Boolean(demoShell?.querySelector('.shell__sidebar')),
        demoShellBgToken: demoShellStyle?.getPropertyValue('--bg').trim() ?? null,
        demoShellTextColor: demoShellStyle?.color ?? null,
        demoShellColorScheme: demoShellStyle?.colorScheme ?? null,
        heroTitleColor: heroTitleStyle?.color ?? null,
        workspaceTheme: workspaceHost?.getAttribute('data-tp-theme') ?? null,
        screenTheme: screenHost?.getAttribute('data-tp-theme') ?? null,
        commandDockTheme: commandDockHost?.getAttribute('data-tp-theme') ?? null,
        themeButtonLabel,
        themeButtonTitle,
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
      const updatedWrapButton = toolbarRoot?.querySelector('[data-testid="tp-line-wrap-option"]') ?? wrapButton;
      return {
        clicked: true,
        fontScale: state?.terminalDisplay?.fontScale ?? null,
        lineWrap: state?.terminalDisplay?.lineWrap ?? null,
        screenFontScale: screenHost?.getAttribute('data-font-scale') ?? null,
        screenLineWrap: screenHost?.getAttribute('data-line-wrap') ?? null,
        activeFontScaleButton: toolbarRoot?.querySelector('[part="font-scale-option"][aria-pressed="true"]')
          ?.getAttribute('data-font-scale') ?? null,
        largeButtonLabel: largeButton.getAttribute('data-font-scale-label')
          ?? largeButton.textContent?.replace(/\\s+/g, ' ').trim()
          ?? null,
        largeButtonTitle: largeButton.getAttribute('title'),
        wrapPressed: updatedWrapButton.getAttribute('aria-pressed'),
        wrapLabel: updatedWrapButton.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        wrapTitle: updatedWrapButton.getAttribute('title'),
        wrapNext: updatedWrapButton.getAttribute('data-line-wrap-next'),
        storedFontScale: window.localStorage.getItem(${JSON.stringify(fontScaleStorageKey)}),
        storedLineWrap: window.localStorage.getItem(${JSON.stringify(lineWrapStorageKey)}),
      };
    })()`);

    const afterTopologyActions = await evaluate(send, `(async () => {
      const countPanes = (node) => {
        if (!node) {
          return 0;
        }
        return node.kind === 'leaf'
          ? 1
          : countPanes(node.first) + countPanes(node.second);
      };
      const settle = () => new Promise((resolve) => setTimeout(resolve, 1600));
      const settleFrame = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const setInputValue = (input, value) => {
        const descriptor = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
        descriptor?.set?.call(input, value);
        input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
      };
      const findPaneCloseButton = (root, paneId) => [...(root?.querySelectorAll('[data-testid="tp-close-pane"]') ?? [])]
        .find((button) => button.getAttribute('data-pane-id') === paneId) ?? null;
      const readTerminalTabStripState = () => {
        const tabStripRoot = workspaceHost?.shadowRoot?.querySelector('tp-terminal-tab-strip')?.shadowRoot ?? null;
        const tabStripPanel = tabStripRoot?.querySelector('[data-testid="tp-terminal-tab-strip"]') ?? null;
        const tabStripTabs = [...(tabStripRoot?.querySelectorAll('[data-testid="tp-terminal-tab"]') ?? [])];
        const tabStripCloseButtons =
          [...(tabStripRoot?.querySelectorAll('[data-testid="tp-terminal-tab-close"]') ?? [])];
        return {
          tabCount: tabStripPanel?.getAttribute('data-tab-count') ?? null,
          rendered: tabStripTabs.length,
          active: tabStripTabs.filter((tab) => tab.getAttribute('aria-pressed') === 'true').length,
          activeTabButton: tabStripTabs.find((tab) => tab.getAttribute('aria-pressed') === 'true') ?? null,
          closeButtons: tabStripCloseButtons.length,
          enabledCloseButtons: tabStripCloseButtons.filter((button) => !button.disabled).length,
          activeCloseButton:
            tabStripCloseButtons.find((button) => {
              const tabId = button.getAttribute('data-tab-id');
              return tabStripTabs.some((tab) =>
                tab.getAttribute('data-tab-id') === tabId
                && tab.getAttribute('aria-pressed') === 'true'
              );
            }) ?? null,
        };
      };
      const stateBefore = window.terminalDemoDebug?.getState?.();
      const topologyBefore = stateBefore?.attachedSession?.topology ?? null;
      const focusedTabBefore = topologyBefore?.tabs?.find((tab) => tab.tab_id === topologyBefore.focused_tab)
        ?? topologyBefore?.tabs?.[0]
        ?? null;
      const workspaceHost = document.querySelector('tp-terminal-workspace') ?? null;
      const paneTreeRoot = workspaceHost?.shadowRoot?.querySelector('tp-terminal-pane-tree')?.shadowRoot ?? null;
      const splitButton = paneTreeRoot?.querySelector('[data-testid="tp-split-right"]') ?? null;
      const newTabButton = paneTreeRoot?.querySelector('[data-testid="tp-new-tab"]') ?? null;
      const renameButton = paneTreeRoot?.querySelector('[data-testid="tp-rename-tab"]') ?? null;
      const closeTabButton = paneTreeRoot?.querySelector('[data-testid="tp-close-tab"]') ?? null;
      const resizeWiderButton = paneTreeRoot?.querySelector('[data-testid="tp-resize-wider"]') ?? null;
      if (!topologyBefore || !focusedTabBefore || !splitButton || !newTabButton || !renameButton || !closeTabButton || !resizeWiderButton) {
        return {
          ok: false,
          reason: 'topology controls missing',
          completedEvents: 0,
        };
      }

      let completedEvents = 0;
      const handleCompleted = () => {
        completedEvents += 1;
      };
      workspaceHost?.addEventListener('tp-terminal-topology-action-completed', handleCompleted);
      workspaceHost?.addEventListener('tp-terminal-tab-strip-action-completed', handleCompleted);

      const paneCountBefore = countPanes(focusedTabBefore.root);
      const focusedPaneBefore = focusedTabBefore.focused_pane;
      splitButton.click();
      await settle();
      const stateAfterSplit = window.terminalDemoDebug?.getState?.();
      const splitTab = stateAfterSplit?.attachedSession?.topology?.tabs?.find(
        (tab) => tab.tab_id === focusedTabBefore.tab_id,
      ) ?? null;
      const paneCountAfterSplit = splitTab ? countPanes(splitTab.root) : 0;
      const resizeColsBefore = stateAfterSplit?.attachedSession?.focused_screen?.cols ?? 0;
      const splitDirection = paneTreeRoot?.querySelector('[part="split"]')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null;
      resizeWiderButton.click();
      await settle();
      const stateAfterResize = window.terminalDemoDebug?.getState?.();
      const resizeColsAfter = stateAfterResize?.attachedSession?.focused_screen?.cols ?? 0;
      const paneToClose = [...(paneTreeRoot?.querySelectorAll('[data-testid="tp-close-pane"]') ?? [])]
        .map((button) => button.getAttribute('data-pane-id'))
        .find((paneId) => paneId && paneId !== splitTab?.focused_pane) ?? null;
      const closePaneButton = paneToClose ? findPaneCloseButton(paneTreeRoot, paneToClose) : null;
      closePaneButton?.click();
      await settleFrame();
      const armedPaneCloseButton = paneToClose ? findPaneCloseButton(paneTreeRoot, paneToClose) : null;
      const closePanePrompted = Boolean(
        armedPaneCloseButton?.getAttribute('data-confirming') === 'true'
        && /confirm close/i.test(armedPaneCloseButton.textContent ?? ''),
      );
      const closePaneDanger = armedPaneCloseButton?.getAttribute('data-danger') ?? null;
      const closePaneTitle = armedPaneCloseButton?.getAttribute('title') ?? null;
      const stateAfterClosePanePrompt = window.terminalDemoDebug?.getState?.();
      const promptedTab = stateAfterClosePanePrompt?.attachedSession?.topology?.tabs?.find(
        (tab) => tab.tab_id === focusedTabBefore.tab_id,
      ) ?? null;
      armedPaneCloseButton?.click();
      await settle();
      const stateAfterClosePane = window.terminalDemoDebug?.getState?.();
      const tabAfterClosePane = stateAfterClosePane?.attachedSession?.topology?.tabs?.find(
        (tab) => tab.tab_id === focusedTabBefore.tab_id,
      ) ?? null;

      renameButton.click();
      await settleFrame();
      const renameInput = paneTreeRoot?.querySelector('[data-testid="tp-rename-tab-input"]') ?? null;
      const renameSave = paneTreeRoot?.querySelector('[data-testid="tp-rename-tab-save"]') ?? null;
      if (renameInput && renameSave) {
        setInputValue(renameInput, 'Smoke Workspace');
        renameSave.click();
        await settle();
      }
      const stateAfterRename = window.terminalDemoDebug?.getState?.();
      const renamedTab = stateAfterRename?.attachedSession?.topology?.tabs?.find(
        (tab) => tab.tab_id === focusedTabBefore.tab_id,
      ) ?? null;

      newTabButton.click();
      await settle();
      const stateAfterNewTab = window.terminalDemoDebug?.getState?.();
      const topologyAfterNewTab = stateAfterNewTab?.attachedSession?.topology ?? null;
      const terminalTabStripAfterNewTab = readTerminalTabStripState();
      const newTabId = topologyAfterNewTab?.focused_tab ?? null;
      terminalTabStripAfterNewTab.activeTabButton?.focus();
      terminalTabStripAfterNewTab.activeTabButton?.dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true,
        composed: true,
        key: 'ArrowLeft',
      }));
      await settle();
      const topologyAfterKeyboardLeft = window.terminalDemoDebug?.getState?.()?.attachedSession?.topology ?? null;
      const terminalTabStripAfterKeyboardLeft = readTerminalTabStripState();
      terminalTabStripAfterKeyboardLeft.activeTabButton?.dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true,
        composed: true,
        key: 'ArrowRight',
      }));
      await settle();
      const topologyAfterKeyboardRight = window.terminalDemoDebug?.getState?.()?.attachedSession?.topology ?? null;
      const terminalTabStripAfterKeyboardRight = readTerminalTabStripState();
      const closeTabButtonAfterNewTab =
        terminalTabStripAfterKeyboardRight.activeCloseButton
        ?? paneTreeRoot?.querySelector('[data-testid="tp-close-tab"]')
        ?? null;
      closeTabButtonAfterNewTab?.click();
      await settleFrame();
      const armedCloseTabButton =
        readTerminalTabStripState().activeCloseButton
        ?? paneTreeRoot?.querySelector('[data-testid="tp-close-tab"]')
        ?? null;
      const closeTabPrompted = Boolean(
        armedCloseTabButton?.getAttribute('data-confirming') === 'true'
        && /confirm/i.test(armedCloseTabButton.getAttribute('title') ?? armedCloseTabButton.textContent ?? ''),
      );
      const closeTabDanger = armedCloseTabButton?.getAttribute('data-danger') ?? null;
      const closeTabTitle = armedCloseTabButton?.getAttribute('title') ?? null;
      const tabCountAfterCloseTabPrompt =
        window.terminalDemoDebug?.getState?.()?.attachedSession?.topology?.tabs?.length ?? 0;
      armedCloseTabButton?.click();
      await settle();
      const topologyAfterCloseTab = window.terminalDemoDebug?.getState?.()?.attachedSession?.topology ?? null;
      const terminalTabStripAfterCloseTab = readTerminalTabStripState();
      const originalTabButton = [...(paneTreeRoot?.querySelectorAll('[data-testid="tp-topology-tab"]') ?? [])]
        .find((button) => button.getAttribute('data-tab-id') === focusedTabBefore.tab_id) ?? null;
      if (originalTabButton) {
        originalTabButton.click();
        await settle();
      }
      const topologyAfterFocus = window.terminalDemoDebug?.getState?.()?.attachedSession?.topology ?? null;
      workspaceHost?.removeEventListener('tp-terminal-topology-action-completed', handleCompleted);
      workspaceHost?.removeEventListener('tp-terminal-tab-strip-action-completed', handleCompleted);

      return {
        ok: true,
        splitClicked: true,
        resizeClicked: true,
        closePanePrompted,
        closePaneDanger,
        closePaneTitle,
        closePaneConfirmed: Boolean(paneToClose && tabAfterClosePane && countPanes(tabAfterClosePane.root) < paneCountAfterSplit),
        renameClicked: Boolean(renameInput && renameSave),
        newTabClicked: true,
        closeTabPrompted,
        closeTabDanger,
        closeTabTitle,
        closeTabConfirmed: Boolean(topologyAfterCloseTab && topologyAfterCloseTab.tabs.length < (topologyAfterNewTab?.tabs?.length ?? 0)),
        focusOriginalClicked: Boolean(originalTabButton),
        completedEvents,
        tabCountBefore: topologyBefore.tabs.length,
        tabCountAfterNewTab: topologyAfterNewTab?.tabs?.length ?? 0,
        terminalTabStripTabCountAfterNewTab: terminalTabStripAfterNewTab.tabCount,
        terminalTabStripRenderedAfterNewTab: terminalTabStripAfterNewTab.rendered,
        terminalTabStripActiveAfterNewTab: terminalTabStripAfterNewTab.active,
        terminalTabStripCloseButtonsAfterNewTab: terminalTabStripAfterNewTab.closeButtons,
        terminalTabStripEnabledCloseButtonsAfterNewTab: terminalTabStripAfterNewTab.enabledCloseButtons,
        terminalTabStripKeyboardLeftFocusedOriginal: topologyAfterKeyboardLeft?.focused_tab === focusedTabBefore.tab_id,
        terminalTabStripKeyboardRightFocusedNew: Boolean(newTabId && topologyAfterKeyboardRight?.focused_tab === newTabId),
        tabCountAfterCloseTabPrompt,
        tabCountAfterCloseTab: topologyAfterCloseTab?.tabs?.length ?? 0,
        terminalTabStripTabCountAfterCloseTab: terminalTabStripAfterCloseTab.tabCount,
        terminalTabStripRenderedAfterCloseTab: terminalTabStripAfterCloseTab.rendered,
        terminalTabStripCloseButtonsAfterCloseTab: terminalTabStripAfterCloseTab.closeButtons,
        terminalTabStripEnabledCloseButtonsAfterCloseTab: terminalTabStripAfterCloseTab.enabledCloseButtons,
        paneCountBefore,
        paneCountAfterSplit,
        paneCountAfterClosePrompt: promptedTab ? countPanes(promptedTab.root) : 0,
        paneCountAfterClosePane: tabAfterClosePane ? countPanes(tabAfterClosePane.root) : 0,
        focusedPaneBefore,
        focusedPaneAfterSplit: splitTab?.focused_pane ?? null,
        splitDirection,
        resizeColsBefore,
        resizeColsAfter,
        renamedTabTitle: renamedTab?.title ?? null,
        focusedTabAfterFocus: topologyAfterFocus?.focused_tab ?? null,
        originalTabId: focusedTabBefore.tab_id,
      };
    })()`);

    const afterSaveLayout = await evaluate(send, `(async () => {
      const beforeSavedSessionCount = window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      const workspaceHost = document.querySelector('tp-terminal-workspace') ?? null;
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const savedRoot = workspaceRoot?.querySelector('tp-terminal-saved-sessions')?.shadowRoot ?? null;
      const savedPanel = savedRoot?.querySelector('[data-testid="tp-saved-sessions"]') ?? null;
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
      const restoreButton = savedRoot?.querySelector('[data-testid="tp-restore-saved-session"]') ?? null;
      const restoreSemantics = [...(savedRoot?.querySelectorAll('[data-testid="tp-saved-session-restore-semantics"]') ?? [])];
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
        savedPanelCount: savedPanel?.getAttribute('data-saved-count') ?? null,
        savedMatchedCount: savedPanel?.getAttribute('data-matched-count') ?? null,
        savedVisibleCount: savedPanel?.getAttribute('data-visible-count') ?? null,
        savedHiddenCount: savedPanel?.getAttribute('data-hidden-count') ?? null,
        savedFiltered: savedPanel?.getAttribute('data-filtered') ?? null,
        savedItemsRendered: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
        hasSavedFilter: Boolean(savedRoot?.querySelector('[data-testid="tp-saved-session-filter"]')),
        firstSavedTitle: state?.catalog?.savedSessions?.[0]?.title ?? null,
        firstSavedCanRestore: restoreButton?.getAttribute('data-can-restore') ?? null,
        firstSavedRestoreStatus: restoreButton?.getAttribute('data-restore-status') ?? null,
        firstSavedRestoreDisabled: restoreButton?.disabled ?? null,
        firstSavedRestoreTitle: restoreButton?.getAttribute('title') ?? null,
        firstSavedSemanticsCodes: restoreSemantics.map((note) => note.getAttribute('data-semantics-code')),
        firstSavedSemanticsLabels: restoreSemantics.map((note) => note.textContent?.replace(/\\s+/g, ' ').trim() ?? ''),
        saveEventDetail,
        ...deletePromptResult,
      };
    })()`);

    const afterSavedSearch = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const savedRoot = workspaceRoot?.querySelector('tp-terminal-saved-sessions')?.shadowRoot ?? null;
      const savedPanel = savedRoot?.querySelector('[data-testid="tp-saved-sessions"]') ?? null;
      const input = savedRoot?.querySelector('[data-testid="tp-saved-session-filter"]') ?? null;
      const clearButton = savedRoot?.querySelector('[data-testid="tp-clear-saved-session-filter"]') ?? null;
      if (!input || !clearButton || !savedPanel) {
        return {
          searched: false,
          reason: !savedPanel ? 'saved panel missing' : !input ? 'filter input missing' : 'clear button missing',
          savedPanelCount: savedPanel?.getAttribute('data-saved-count') ?? null,
        };
      }

      input.value = 'Workspace';
      input.dispatchEvent(new InputEvent('input', {
        bubbles: true,
        inputType: 'insertText',
        data: 'Workspace',
      }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

      const filteredState = {
        searched: true,
        filtered: savedPanel.getAttribute('data-filtered'),
        savedPanelCount: savedPanel.getAttribute('data-saved-count'),
        matchedCount: Number(savedPanel.getAttribute('data-matched-count') ?? '0'),
        visibleCount: savedPanel.getAttribute('data-visible-count'),
        hiddenCount: savedPanel.getAttribute('data-hidden-count'),
        itemsRendered: savedRoot.querySelectorAll('[part="item"]').length,
        firstTitle: savedRoot.querySelector('[part="item"] strong')?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        hasEmptyState: Boolean(savedRoot.querySelector('[data-testid="tp-saved-session-filter-empty"]')),
        hasPruneHiddenWhileFiltered: Boolean(savedRoot.querySelector('[data-testid="tp-prune-hidden-saved-sessions"]')),
      };

      clearButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

      return {
        ...filteredState,
        afterClearFiltered: savedPanel.getAttribute('data-filtered'),
        afterClearValue: input.value,
        afterClearPanelCount: savedPanel.getAttribute('data-saved-count'),
        afterClearMatchedCount: savedPanel.getAttribute('data-matched-count'),
      };
    })()`);

    const afterPruneHidden = await evaluate(send, `(async () => {
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const waitForState = async (predicate, timeoutMs = 8000) => {
        const startedAt = Date.now();
        while (Date.now() - startedAt < timeoutMs) {
          const state = window.terminalDemoDebug?.getState?.();
          if (predicate(state)) {
            return state;
          }
          await wait(200);
        }
        return window.terminalDemoDebug?.getState?.();
      };
      const waitForSavedCount = async (expectedMin) => {
        for (let attempt = 0; attempt < 30; attempt += 1) {
          const count = window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
          if (count >= expectedMin) {
            return count;
          }
          await wait(200);
        }
        return window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      };
      const workspaceHost = document.querySelector('tp-terminal-workspace') ?? null;
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const savedRoot = workspaceRoot?.querySelector('tp-terminal-saved-sessions')?.shadowRoot ?? null;
      const sessionTools = commandRoot?.querySelector('[data-testid="tp-session-tools"]') ?? null;
      const kernelCommands = window.terminalDemoDebug?.controller?.commands ?? null;
      const defaultProgram = new URL(window.location.href).searchParams.get('demoDefaultShellProgram') || 'zsh';
      if (!workspaceHost || !commandRoot || !savedRoot || !sessionTools || !kernelCommands?.createSession) {
        return {
          prompted: false,
          confirmed: false,
          reason: 'saved-session setup controls missing',
          savedSessionCountBefore: window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0,
          visibleCountBefore: savedRoot?.querySelectorAll('[part="item"]')?.length ?? 0,
          savedSessionCountAfterPrompt: 0,
          savedSessionCountAfter: 0,
          deletedCount: 0,
          eventDetail: null,
        };
      }

      sessionTools.open = true;
      let savedCount = window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      let setupAttempts = 0;
      while (savedCount < 5 && setupAttempts < 8) {
        setupAttempts += 1;
        const stateBeforeCreate = window.terminalDemoDebug?.getState?.();
        const sessionCountBefore = stateBeforeCreate?.catalog?.sessions?.length ?? 0;
        const activeSessionBefore = stateBeforeCreate?.selection?.activeSessionId ?? null;
        await kernelCommands.createSession('native', {
          title: \`Smoke prune seed \${setupAttempts}\`,
          launch: {
            program: defaultProgram,
            args: [],
            cwd: null,
          },
        });
        const stateAfterCreate = await waitForState((state) => {
          const nextSessionCount = state?.catalog?.sessions?.length ?? 0;
          return nextSessionCount > sessionCountBefore
            && state?.selection?.activeSessionId
            && state.selection.activeSessionId !== activeSessionBefore;
        });
        if ((stateAfterCreate?.catalog?.sessions?.length ?? 0) <= sessionCountBefore) {
          await wait(200);
          continue;
        }
        await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

        const saveLayoutButton = commandRoot?.querySelector('[data-testid="tp-save-layout"]') ?? null;
        if (!saveLayoutButton) {
          return {
            prompted: false,
            confirmed: false,
            reason: 'save layout missing while seeding prune workflow',
            savedSessionCountBefore: savedCount,
            visibleCountBefore: savedRoot.querySelectorAll('[part="item"]')?.length ?? 0,
            savedSessionCountAfterPrompt: savedCount,
            savedSessionCountAfter: savedCount,
            deletedCount: 0,
            eventDetail: null,
          };
        }
        if (saveLayoutButton.disabled) {
          await wait(200);
          continue;
        }

        saveLayoutButton.click();
        const nextSavedCount = await waitForSavedCount(savedCount + 1);
        if (nextSavedCount <= savedCount) {
          await wait(200);
          continue;
        }
        savedCount = nextSavedCount;
      }

      if (savedCount < 5) {
        return {
          prompted: false,
          confirmed: false,
          reason: 'unable to seed enough saved layouts for prune workflow',
          savedSessionCountBefore: savedCount,
          visibleCountBefore: savedRoot.querySelectorAll('[part="item"]')?.length ?? 0,
          savedSessionCountAfterPrompt: savedCount,
          savedSessionCountAfter: savedCount,
          deletedCount: 0,
          eventDetail: null,
        };
      }
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

      const savedSessionCountBefore = window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      const visibleCountBefore = savedRoot.querySelectorAll('[part="item"]')?.length ?? 0;
      const pruneButton = savedRoot.querySelector('[data-testid="tp-prune-hidden-saved-sessions"]') ?? null;
      if (!pruneButton || pruneButton.disabled) {
        return {
          prompted: false,
          confirmed: false,
          reason: pruneButton ? 'prune hidden disabled' : 'prune hidden missing',
          savedSessionCountBefore,
          visibleCountBefore,
          pruneCount: Number(pruneButton?.getAttribute('data-prune-count') ?? '0'),
          pruneKeepLatest: Number(pruneButton?.getAttribute('data-prune-keep-latest') ?? '0'),
          pruneButtonText: pruneButton?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
          pruneButtonTitle: pruneButton?.getAttribute('title') ?? null,
          confirmButtonText: null,
          savedSessionCountAfterPrompt: savedSessionCountBefore,
          savedSessionCountAfter: savedSessionCountBefore,
          deletedCount: Math.max(0, savedSessionCountBefore - visibleCountBefore),
          eventDetail: null,
        };
      }

      let eventDetail = null;
      workspaceHost.addEventListener('tp-saved-sessions-pruned', (event) => {
        eventDetail = event.detail ?? null;
      }, { once: true });

      const pruneCount = Number(pruneButton.getAttribute('data-prune-count') ?? '0');
      const pruneKeepLatest = Number(pruneButton.getAttribute('data-prune-keep-latest') ?? '0');
      const pruneButtonText = pruneButton.textContent?.replace(/\\s+/g, ' ').trim() ?? null;
      const pruneButtonTitle = pruneButton.getAttribute('title');
      pruneButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const armedButton = savedRoot.querySelector('[data-testid="tp-prune-hidden-saved-sessions"]') ?? null;
      const confirmButtonText = armedButton?.textContent?.replace(/\\s+/g, ' ').trim() ?? null;
      const prompted = Boolean(
        armedButton?.getAttribute('data-confirming') === 'true'
        && /confirm prune/i.test(armedButton.textContent ?? ''),
      );
      const savedSessionCountAfterPrompt =
        window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      armedButton?.click();
      await wait(1200);

      const savedSessionCountAfter = window.terminalDemoDebug?.getState?.()?.catalog?.savedSessions?.length ?? 0;
      const visibleCountAfter = savedRoot.querySelectorAll('[part="item"]')?.length ?? 0;
      const hiddenAfter = Boolean(savedRoot.querySelector('[data-testid="tp-prune-hidden-saved-sessions"]'));

      return {
        prompted,
        confirmed: savedSessionCountAfter < savedSessionCountAfterPrompt,
        savedSessionCountBefore,
        visibleCountBefore,
        pruneCount,
        pruneKeepLatest,
        pruneButtonText,
        pruneButtonTitle,
        confirmButtonText,
        savedSessionCountAfterPrompt,
        savedSessionCountAfter,
        visibleCountAfter,
        hiddenAfter,
        deletedCount: Math.max(0, savedSessionCountBefore - savedSessionCountAfter),
        eventDetail,
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
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
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
        commandHistoryCount: debug?.commandHistory?.entries?.length ?? 0,
        commandHistoryLatest: debug?.commandHistory?.entries?.at(-1) ?? null,
        commandHistoryLimit: debug?.commandHistory?.limit ?? null,
        commandInputFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true,
        commandInputEmpty: textarea?.value === '',
        commandCursorAtEnd: Boolean(
          textarea
          && textarea.selectionStart === textarea.value.length
          && textarea.selectionEnd === textarea.value.length,
        ),
        historyBadgeText: commandRoot?.querySelector('[data-testid="tp-command-history-count"]')
          ?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        terminalScreenText,
        containsCommandOutput: /browser-smoke-ok/i.test(terminalScreenText),
      };
    })()`);
    const replayInitialSequence = afterCommand.focusedSequence;

    const afterCommandActionFocus = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const enterButton = commandRoot?.querySelector('[data-testid="tp-send-enter"]') ?? null;
      const interruptButton = commandRoot?.querySelector('[data-testid="tp-send-interrupt"]') ?? null;
      const waitForFocusReturn = async () => {
        for (let attempt = 0; attempt < 10; attempt += 1) {
          await new Promise((resolve) => setTimeout(resolve, 150));
          if (
            window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true
            && !textarea?.disabled
          ) {
            return true;
          }
        }
        return window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true;
      };
      const cursorAtEnd = () => Boolean(
        textarea
        && textarea.selectionStart === textarea.value.length
        && textarea.selectionEnd === textarea.value.length,
      );

      if (!textarea || !enterButton || !interruptButton) {
        return {
          tested: false,
          reason: !textarea ? 'textarea missing' : !enterButton ? 'enter button missing' : 'interrupt button missing',
          enterFocused: false,
          enterCursorAtEnd: false,
          interruptFocused: false,
          interruptCursorAtEnd: false,
        };
      }
      if (enterButton.disabled || interruptButton.disabled) {
        return {
          tested: false,
          reason: enterButton.disabled ? 'enter button disabled' : 'interrupt button disabled',
          enterFocused: false,
          enterCursorAtEnd: false,
          interruptFocused: false,
          interruptCursorAtEnd: false,
        };
      }

      enterButton.click();
      await waitForFocusReturn();
      const enterFocused = window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true;
      const enterCursorAtEnd = cursorAtEnd();

      interruptButton.click();
      await waitForFocusReturn();
      return {
        tested: true,
        enterFocused,
        enterCursorAtEnd,
        interruptFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true,
        interruptCursorAtEnd: cursorAtEnd(),
      };
    })()`);

    afterScreenCopy = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const screenRoot = screenHost?.shadowRoot ?? null;
      const copyButton = screenRoot?.querySelector('[data-testid="tp-screen-copy"]') ?? null;
      if (!screenHost || !copyButton) {
        return {
          clicked: false,
          reason: screenHost ? 'copy button missing' : 'screen host missing',
          copiedEvents: 0,
          failedEvents: 0,
          containsCopiedCommandOutput: false,
          buttonText: copyButton?.textContent?.trim() ?? null,
          eventDetail: null,
        };
      }

      let copiedEvents = 0;
      let failedEvents = 0;
      let eventDetail = null;
      screenHost.addEventListener('tp-terminal-screen-copied', (event) => {
        copiedEvents += 1;
        eventDetail = event.detail ?? null;
      }, { once: true });
      screenHost.addEventListener('tp-terminal-screen-copy-failed', () => {
        failedEvents += 1;
      }, { once: true });

      if (copyButton.disabled) {
        return {
          clicked: false,
          reason: 'copy button disabled',
          copiedEvents,
          failedEvents,
          containsCopiedCommandOutput: false,
          buttonText: copyButton.textContent?.trim() ?? null,
          eventDetail,
        };
      }

      copyButton.click();
      await new Promise((resolve) => setTimeout(resolve, 350));
      let clipboardText = '';
      try {
        clipboardText = await Promise.race([
          navigator.clipboard.readText(),
          new Promise((_, reject) => setTimeout(() => reject(new Error('clipboard read timeout')), 1500)),
        ]);
      } catch (error) {
        return {
          clicked: true,
          readError: String(error?.message ?? error),
          copiedEvents,
          failedEvents,
          containsCopiedCommandOutput: false,
          buttonText: copyButton.textContent?.trim() ?? null,
          eventDetail,
        };
      }

      return {
        clicked: true,
        copiedEvents,
        failedEvents,
        containsCopiedCommandOutput: /browser-smoke-ok/i.test(clipboardText),
        clipboardPreview: clipboardText.slice(0, 120),
        buttonText: copyButton.textContent?.trim() ?? null,
        eventDetail,
      };
    })()`);

    const afterRecentCommandRecall = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const historyEntry = commandRoot?.querySelector('[data-testid="tp-command-history-entry"]') ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      if (!historyEntry || !textarea) {
        return {
          clicked: false,
          reason: historyEntry ? 'textarea missing' : 'history entry missing',
          recalledDraft: textarea?.value ?? null,
          sendEnabled: false,
          inputFocused: false,
          cursorAtEnd: false,
          historyBadgeText: commandRoot?.querySelector('[data-testid="tp-command-history-count"]')
            ?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        };
      }

      historyEntry.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const sendButton = commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null;
      return {
        clicked: true,
        entryTitle: historyEntry.getAttribute('title'),
        recalledDraft: textarea.value,
        sendEnabled: Boolean(sendButton && !sendButton.disabled),
        inputFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true,
        cursorAtEnd: textarea.selectionStart === textarea.value.length && textarea.selectionEnd === textarea.value.length,
        historyBadgeText: commandRoot?.querySelector('[data-testid="tp-command-history-count"]')
          ?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
      };
    })()`);

    const clipboardSeedResult = await evaluate(send, `(async () => {
      try {
        await navigator.clipboard.writeText(${JSON.stringify('printf "browser-paste-ok\\n"\n')});
        return { ok: true };
      } catch (error) {
        return { ok: false, error: String(error?.message ?? error) };
      }
    })()`);
    if (!clipboardSeedResult.ok) {
      throw new Error(`Unable to seed clipboard for paste workflow: ${JSON.stringify(clipboardSeedResult)}`);
    }

    const pasteResult = await evaluate(send, `(async () => {
      const workspaceHost = document.querySelector('tp-terminal-workspace') ?? null;
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const commandHost = workspaceRoot?.querySelector('tp-terminal-command-dock') ?? null;
      const commandRoot = commandHost?.shadowRoot ?? null;
      const pasteButton = commandRoot?.querySelector('[data-testid="tp-paste-clipboard"]') ?? null;
      if (!workspaceHost || !commandHost || !pasteButton) {
        return {
          ok: false,
          reason: !workspaceHost ? 'workspace missing' : !commandHost ? 'command dock missing' : 'paste button missing',
          pasteButtonEnabled: false,
          submittedEvents: 0,
        };
      }

      let submittedEvents = 0;
      commandHost.addEventListener('tp-terminal-paste-submitted', () => {
        submittedEvents += 1;
      }, { once: true });

      if (pasteButton.disabled) {
        return {
          ok: false,
          reason: 'paste button disabled',
          pasteButtonEnabled: false,
          submittedEvents,
          title: pasteButton.getAttribute('title'),
        };
      }

      pasteButton.click();
      await new Promise((resolve) => setTimeout(resolve, 1600));

      return {
        ok: true,
        pasteButtonEnabled: !pasteButton.disabled,
        submittedEvents,
        title: pasteButton.getAttribute('title'),
      };
    })()`);
    if (!pasteResult.ok) {
      throw new Error(`Unable to paste clipboard through command dock: ${JSON.stringify(pasteResult)}`);
    }

    const afterClipboardPaste = await evaluate(send, `(() => {
      const debug = window.terminalDemoDebug?.getState?.();
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const terminalScreenText = debug?.attachedSession?.focused_screen?.surface?.lines
        ? debug.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : '';
      return {
        clicked: true,
        pasteButtonEnabled: ${JSON.stringify(pasteResult.pasteButtonEnabled)},
        submittedEvents: ${JSON.stringify(pasteResult.submittedEvents)},
        connectionReady: debug?.connection?.state === 'ready',
        commandHistoryCount: debug?.commandHistory?.entries?.length ?? 0,
        commandHistoryLatest: debug?.commandHistory?.entries?.at(-1) ?? null,
        inputFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, textarea) === true,
        cursorAtEnd: Boolean(
          textarea
          && textarea.selectionStart === textarea.value.length
          && textarea.selectionEnd === textarea.value.length,
        ),
        terminalScreenText,
        containsPasteOutput: /browser-paste-ok/i.test(terminalScreenText),
      };
    })()`);

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
        hasHighlights: Boolean(screenRoot?.querySelector('[part~="search-match"]')),
        hasActiveHighlight: Boolean(screenRoot?.querySelector('[part~="active-search-match"]')),
        nextClicked,
      };
    })()`);

    afterScreenSearchShortcut = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const screenHost = workspaceRoot?.querySelector('tp-terminal-screen') ?? null;
      const screenRoot = screenHost?.shadowRoot ?? null;
      const viewport = screenRoot?.querySelector('[data-testid="tp-screen-viewport"]') ?? null;
      const searchInput = screenRoot?.querySelector('[data-testid="tp-screen-search"]') ?? null;
      if (!screenHost || !screenRoot || !viewport || !searchInput) {
        return {
          tested: false,
          reason: !screenHost ? 'screen host missing' : !viewport ? 'viewport missing' : 'search input missing',
          defaultPrevented: false,
          searchFocused: false,
          selectedExistingQuery: false,
          viewportFocusedAfterEscape: false,
          queryAfterEscape: null,
          submittedEvents: 0,
        };
      }

      let submittedEvents = 0;
      const handleSubmitted = () => {
        submittedEvents += 1;
      };
      screenHost.addEventListener('tp-terminal-screen-input-submitted', handleSubmitted);
      viewport.focus();
      const shortcutEvent = new KeyboardEvent('keydown', {
        key: 'f',
        metaKey: true,
        bubbles: true,
        cancelable: true,
      });
      const dispatchResult = viewport.dispatchEvent(shortcutEvent);
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const searchFocused = screenRoot.activeElement === searchInput;
      const selectedExistingQuery =
        searchInput.selectionStart === 0 && searchInput.selectionEnd === searchInput.value.length;

      searchInput.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'Escape',
        bubbles: true,
        cancelable: true,
      }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      screenHost.removeEventListener('tp-terminal-screen-input-submitted', handleSubmitted);

      return {
        tested: true,
        defaultPrevented: !dispatchResult || shortcutEvent.defaultPrevented,
        searchFocused,
        selectedExistingQuery,
        viewportFocusedAfterEscape: screenRoot.activeElement === viewport,
        queryAfterEscape: searchInput.value,
        submittedEvents,
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
          startedFollowing: false,
          reason: 'screen controls missing',
          screenViewportAtBottom: false,
        };
      }

      if (followButton.getAttribute('aria-pressed') !== 'true') {
        scrollLatestButton.click();
        await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      }
      const startedFollowing =
        followButton.getAttribute('aria-pressed') === 'true' && followButton.textContent?.trim() === 'Following';

      followButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const paused = followButton.getAttribute('aria-pressed') === 'false' && followButton.textContent?.trim() === 'Paused';

      scrollLatestButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const resumed = followButton.getAttribute('aria-pressed') === 'true' && followButton.textContent?.trim() === 'Following';

      return {
        paused,
        resumed,
        startedFollowing,
        screenViewportAtBottom: viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= 2,
      };
    })()`);

    const historyReplayResult = await evaluate(send, `(async () => {
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const textarea = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      if (!textarea) {
        return { ok: false, reason: 'textarea missing', recalledDraft: null, restoredDraft: null };
      }
      const draftBeforeReplay = 'draft-before-history-replay';
      const descriptor = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value');
      descriptor?.set?.call(textarea, draftBeforeReplay);
      textarea.dispatchEvent(new Event('input', { bubbles: true }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));

      textarea.focus();
      textarea.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'ArrowUp',
        bubbles: true,
        cancelable: true,
      }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const recalledDraft = textarea.value;
      textarea.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'ArrowDown',
        bubbles: true,
        cancelable: true,
      }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const restoredDraft = textarea.value;
      textarea.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'ArrowUp',
        bubbles: true,
        cancelable: true,
      }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const replayDraft = textarea.value;
      const button = commandRoot?.querySelector('[data-testid="tp-send-command"]') ?? null;
      if (!button) {
        return { ok: false, reason: 'send button missing', recalledDraft: replayDraft, restoredDraft };
      }
      if (button.disabled) {
        return {
          ok: false,
          reason: 'send button disabled after history recall',
          recalledDraft: replayDraft,
          restoredDraft,
        };
      }
      button.click();
      return { ok: true, recalledDraft: replayDraft, restoredDraft };
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
        restoredDraft: ${JSON.stringify(historyReplayResult.restoredDraft)},
        connectionReady: debug?.connection?.state === 'ready',
        focusedSequence: debug?.attachedSession?.focused_screen?.sequence != null
          ? String(debug.attachedSession.focused_screen.sequence)
          : null,
        commandHistoryCount: debug?.commandHistory?.entries?.length ?? 0,
        commandHistoryLatest: debug?.commandHistory?.entries?.at(-1) ?? null,
        terminalScreenText,
        containsCommandOutput: /browser-smoke-ok/i.test(terminalScreenText),
      };
    })()`);

    const afterCommandHistoryClear = await evaluate(send, `(async () => {
      const workspaceHost = document.querySelector('tp-terminal-workspace') ?? null;
      const workspaceRoot = workspaceHost?.shadowRoot ?? null;
      const commandHost = workspaceRoot?.querySelector('tp-terminal-command-dock') ?? null;
      const commandRoot = commandHost?.shadowRoot ?? null;
      const sessionTools = commandRoot?.querySelector('[data-testid="tp-session-tools"]') ?? null;
      const clearButton = commandRoot?.querySelector('[data-testid="tp-clear-command-history"]') ?? null;
      const beforeCount = window.terminalDemoDebug?.getState?.()?.commandHistory?.entries?.length ?? 0;
      if (!workspaceHost || !sessionTools || !clearButton) {
        return {
          clicked: false,
          reason: 'clear history controls missing',
          beforeCount,
          afterFirstCount: beforeCount,
          afterCount: beforeCount,
          clearedEvents: 0,
          clearedEventsAfterFirst: 0,
          firstClickArmed: false,
          firstClickLabel: null,
          finalConfirming: false,
          clearButtonDisabled: clearButton?.disabled ?? null,
          historyBadgeText: commandRoot?.querySelector('[data-testid="tp-command-history-count"]')
            ?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        };
      }

      let clearedEvents = 0;
      workspaceHost.addEventListener('tp-terminal-command-history-cleared', () => {
        clearedEvents += 1;
      }, { once: true });

      sessionTools.open = true;
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      if (clearButton.disabled) {
        return {
          clicked: false,
          reason: 'clear history disabled',
          beforeCount,
          afterFirstCount: beforeCount,
          afterCount: beforeCount,
          clearedEvents,
          clearedEventsAfterFirst: clearedEvents,
          firstClickArmed: false,
          firstClickLabel: clearButton.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
          finalConfirming: clearButton.getAttribute('data-confirming') === 'true',
          clearButtonDisabled: true,
          historyBadgeText: commandRoot?.querySelector('[data-testid="tp-command-history-count"]')
            ?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        };
      }

      clearButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const armedButton = commandRoot.querySelector('[data-testid="tp-clear-command-history"]');
      const firstClickLabel = armedButton?.textContent?.replace(/\\s+/g, ' ').trim() ?? null;
      const firstClickArmed = armedButton?.getAttribute('data-confirming') === 'true';
      const afterFirstCount = window.terminalDemoDebug?.getState?.()?.commandHistory?.entries?.length ?? 0;
      const clearedEventsAfterFirst = clearedEvents;

      if (!armedButton || armedButton.disabled) {
        return {
          clicked: true,
          reason: armedButton ? 'clear history confirmation disabled' : 'clear history confirmation missing',
          beforeCount,
          afterFirstCount,
          afterCount: afterFirstCount,
          clearedEvents,
          clearedEventsAfterFirst,
          firstClickArmed,
          firstClickLabel,
          finalConfirming: firstClickArmed,
          clearButtonDisabled: armedButton?.disabled ?? null,
          historyBadgeText: commandRoot?.querySelector('[data-testid="tp-command-history-count"]')
            ?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
        };
      }

      armedButton.click();
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const finalButton = commandRoot.querySelector('[data-testid="tp-clear-command-history"]');
      const state = window.terminalDemoDebug?.getState?.();
      return {
        clicked: true,
        beforeCount,
        afterFirstCount,
        afterCount: state?.commandHistory?.entries?.length ?? 0,
        clearedEvents,
        clearedEventsAfterFirst,
        firstClickArmed,
        firstClickLabel,
        finalConfirming: finalButton?.getAttribute('data-confirming') === 'true',
        clearButtonDisabled: finalButton?.disabled ?? null,
        historyBadgeText: commandRoot?.querySelector('[data-testid="tp-command-history-count"]')
          ?.textContent?.replace(/\\s+/g, ' ').trim() ?? null,
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
      afterCreateMobileLayout,
      afterQuickCommandDraft,
      afterMultilineCommandDraft,
      afterScreenSearch,
      afterScreenSearchShortcut,
      afterSavedPagination,
      afterThemeSwitch,
      afterDisplaySwitch,
      afterTopologyActions,
      afterSaveLayout,
      afterSavedSearch,
      afterPruneHidden,
      afterCommand: {
        ...afterCommand,
        sequenceAdvanced: initialSequence !== null
          ? afterCommand.focusedSequence !== initialSequence
          : false,
      },
      afterCommandActionFocus,
      afterScreenCopy,
      afterRecentCommandRecall,
      afterClipboardPaste,
      afterScreenFollowToggle,
      afterHistoryReplay: {
        ...afterHistoryReplay,
        sequenceAdvanced: replayInitialSequence !== null
          ? afterHistoryReplay.focusedSequence !== replayInitialSequence
          : false,
      },
      afterCommandHistoryClear,
      afterDirectScreenInput,
      issues,
    };
  } finally {
    await closeWebSocket(socket);
  }
}

async function runAutoStartSmokeScenario(browserUrl) {
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
    await installBrowserSmokeHelpers(send);

    const result = await evaluate(send, `(async () => {
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      for (let attempt = 0; attempt < 30; attempt += 1) {
        const state = window.terminalDemoDebug?.getState?.();
        if (
          state?.connection?.state === 'ready'
          && state?.catalog?.sessions?.length === 1
          && state?.attachedSession?.focused_screen
        ) {
          break;
        }
        await wait(200);
      }

      const state = window.terminalDemoDebug?.getState?.();
      const workspaceRoot = document.querySelector('tp-terminal-workspace')?.shadowRoot ?? null;
      const commandRoot = workspaceRoot?.querySelector('tp-terminal-command-dock')?.shadowRoot ?? null;
      const input = commandRoot?.querySelector('[data-testid="tp-command-input"]') ?? null;
      const terminalScreenText = state?.attachedSession?.focused_screen?.surface?.lines
        ? state.attachedSession.focused_screen.surface.lines.map((line) => line.text).join('\\n').trim()
        : null;

      return {
        hasReady: state?.connection?.state === 'ready',
        hasError: state?.connection?.state === 'error',
        sessionCount: state?.catalog?.sessions?.length ?? 0,
        attached: Boolean(state?.attachedSession?.focused_screen),
        demoAutoStartSession: new URLSearchParams(window.location.search).get('demoAutoStartSession'),
        demoDefaultShellProgram: new URLSearchParams(window.location.search).get('demoDefaultShellProgram'),
        commandInputFocused: window.__terminalDemoSmokeCommandInputFocused?.(commandRoot, input) === true,
        documentHorizontalOverflow: Math.max(
          0,
          document.documentElement.scrollWidth - document.documentElement.clientWidth,
        ),
        terminalScreenTextPreview: terminalScreenText?.slice(0, 240) ?? null,
      };
    })()`);

    return {
      ...result,
      issues,
    };
  } finally {
    await closeWebSocket(socket);
    await closePageTarget(target.id);
  }
}

async function startBrowserHost(rendererUrlValue, options) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error("Timed out waiting for TERMINAL_DEMO_BROWSER_URL"));
    }, 20_000);

    browserHostProcess = spawn("node", ["./dist/host/browser/index.js"], {
      cwd: appRoot,
      env: {
        ...process.env,
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

async function shutdown() {
  await stopProcess(browserHostProcess);
  await stopProcess(previewProcess);
  await stopProcess(chromeProcess);
  if (chromeUserDataDir) {
    await fs.rm(chromeUserDataDir, { recursive: true, force: true });
  }
  await removeSessionStore(autoStartSessionStorePath);
  await removeSessionStore(sessionStorePath);
}

async function removeSessionStore(storePath) {
  await Promise.all([
    fs.rm(storePath, { force: true }),
    fs.rm(`${storePath}-shm`, { force: true }),
    fs.rm(`${storePath}-wal`, { force: true }),
  ]);
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

async function installBrowserSmokeHelpers(send) {
  await evaluate(send, `(() => {
    window.__terminalDemoSmokeCommandInputFocused = (commandRoot, input) => Boolean(
      input
      && (
        commandRoot?.activeElement === input
        || input.matches(':focus')
        || input.closest('tp-terminal-command-composer')?.matches(':focus-within')
      )
    );
  })()`);
}

async function closePageTarget(targetId) {
  await fetch(`http://127.0.0.1:${cdpPort}/json/close/${targetId}`).catch(() => undefined);
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

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
