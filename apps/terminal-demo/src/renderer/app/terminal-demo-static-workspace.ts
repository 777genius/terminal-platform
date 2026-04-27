import type {
  BackendCapabilitiesInfo,
  BackendKind,
  MuxCommand,
  MuxCommandResult,
  PaneTreeNode,
  PaneId,
  SavedSessionSummary,
  ScreenSnapshot,
  SessionId,
  SplitDirection,
  TabSnapshot,
} from "@terminal-platform/runtime-types";
import {
  DEFAULT_COMMAND_HISTORY_LIMIT,
  createInitialWorkspaceSnapshot,
  type WorkspaceCommands,
  type WorkspaceDiagnostics,
  type WorkspaceKernel,
  type WorkspaceSelectors,
  type WorkspaceSnapshot,
} from "@terminal-platform/workspace-core";

export interface TerminalDemoPreviewBootstrapConfig {
  runtimeSlug: string;
}

export const DEFAULT_TERMINAL_DEMO_DISPLAY = {
  fontScale: "default",
  lineWrap: true,
} satisfies WorkspaceSnapshot["terminalDisplay"];

const STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE =
  "static preview: simulated output, no native host is attached";

interface StaticWorkspaceRuntimeState {
  nextTopologyIndex: number;
  screensByPaneId: Map<PaneId, ScreenSnapshot>;
}

export function createStaticWorkspaceKernel(snapshot: WorkspaceSnapshot): WorkspaceKernel {
  let currentSnapshot: WorkspaceSnapshot = {
    ...snapshot,
    commandHistory: snapshot.commandHistory ?? {
      entries: [],
      limit: DEFAULT_COMMAND_HISTORY_LIMIT,
    },
    terminalDisplay: snapshot.terminalDisplay ?? DEFAULT_TERMINAL_DEMO_DISPLAY,
  };
  const runtimeState: StaticWorkspaceRuntimeState = createStaticWorkspaceRuntimeState(currentSnapshot);
  const listeners = new Set<() => void>();
  const notify = () => {
    for (const listener of listeners) {
      listener();
    }
  };
  const updateSnapshot = (nextSnapshot: WorkspaceSnapshot) => {
    currentSnapshot = nextSnapshot;
    notify();
  };
  const noopAsync = async () => {};
  const noopDiagnostics: WorkspaceDiagnostics = {
    list: () => currentSnapshot.diagnostics,
    clear: () => updateSnapshot({
      ...currentSnapshot,
      diagnostics: [],
    }),
  };
  const noopCommands: WorkspaceCommands = {
    bootstrap: noopAsync,
    refreshSessions: noopAsync,
    refreshSavedSessions: noopAsync,
    discoverSessions: noopAsync,
    getBackendCapabilities: async (backend: BackendKind) => {
      const capabilities = currentSnapshot.catalog.backendCapabilities[backend];
      if (!capabilities) {
        throw new Error(`No static backend capabilities for ${backend}`);
      }

      return capabilities;
    },
    createSession: noopAsync,
    importSession: noopAsync,
    attachSession: noopAsync,
    restoreSavedSession: noopAsync,
    deleteSavedSession: noopAsync,
    pruneSavedSessions: async (keepLatest: number) => ({
      deleted_count: Math.max(0, currentSnapshot.catalog.savedSessions.length - keepLatest),
      kept_count: Math.min(currentSnapshot.catalog.savedSessions.length, keepLatest),
    }),
    dispatchMuxCommand: async (sessionId: SessionId, command: MuxCommand) =>
      dispatchStaticMuxCommand(currentSnapshot, updateSnapshot, runtimeState, sessionId, command),
    openSubscription: async () => {
      throw new Error("not implemented in static kernel");
    },
    setActiveSession: (sessionId) => updateSnapshot({
      ...currentSnapshot,
      selection: {
        ...currentSnapshot.selection,
        activeSessionId: sessionId,
      },
    }),
    setActivePane: (paneId) => updateSnapshot({
      ...currentSnapshot,
      selection: {
        ...currentSnapshot.selection,
        activePaneId: paneId,
      },
    }),
    updateDraft: (paneId, value) => updateSnapshot({
      ...currentSnapshot,
      drafts: {
        ...currentSnapshot.drafts,
        [paneId]: value,
      },
    }),
    clearDraft: (paneId) => {
      const nextDrafts = { ...currentSnapshot.drafts };
      delete nextDrafts[paneId];
      updateSnapshot({
        ...currentSnapshot,
        drafts: nextDrafts,
      });
    },
    recordCommandHistory: (value) => {
      const normalizedValue = value.trim();
      if (!normalizedValue) {
        return;
      }

      const limit = currentSnapshot.commandHistory.limit;
      updateSnapshot({
        ...currentSnapshot,
        commandHistory: {
          ...currentSnapshot.commandHistory,
          entries: [
            ...currentSnapshot.commandHistory.entries.filter((entry) => entry !== normalizedValue),
            normalizedValue,
          ].slice(-limit),
        },
      });
    },
    clearCommandHistory: () => updateSnapshot({
      ...currentSnapshot,
      commandHistory: {
        ...currentSnapshot.commandHistory,
        entries: [],
      },
    }),
    setTheme: (themeId) => updateSnapshot({
      ...currentSnapshot,
      theme: { themeId },
    }),
    setTerminalFontScale: (fontScale) => updateSnapshot({
      ...currentSnapshot,
      terminalDisplay: {
        ...currentSnapshot.terminalDisplay,
        fontScale: fontScale as WorkspaceSnapshot["terminalDisplay"]["fontScale"],
      },
    }),
    setTerminalLineWrap: (lineWrap) => updateSnapshot({
      ...currentSnapshot,
      terminalDisplay: {
        ...currentSnapshot.terminalDisplay,
        lineWrap,
      },
    }),
    clearDiagnostics: () => noopDiagnostics.clear(),
  };
  const noopSelectors: WorkspaceSelectors = {
    connection: () => currentSnapshot.connection,
    sessions: () => currentSnapshot.catalog.sessions,
    savedSessions: () => currentSnapshot.catalog.savedSessions,
    activeSession: () => currentSnapshot.catalog.sessions.find(
      (item) => item.session_id === currentSnapshot.selection.activeSessionId,
    ) ?? null,
    activePaneId: () => currentSnapshot.selection.activePaneId,
    attachedSession: () => currentSnapshot.attachedSession,
    diagnostics: () => currentSnapshot.diagnostics,
    themeId: () => currentSnapshot.theme.themeId,
    terminalDisplay: () => currentSnapshot.terminalDisplay,
    commandHistory: () => currentSnapshot.commandHistory,
  };

  return {
    getSnapshot: () => currentSnapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    bootstrap: noopAsync,
    dispose: noopAsync,
    commands: noopCommands,
    selectors: noopSelectors,
    diagnostics: noopDiagnostics,
  };
}

function dispatchStaticMuxCommand(
  snapshot: WorkspaceSnapshot,
  updateSnapshot: (snapshot: WorkspaceSnapshot) => void,
  runtimeState: StaticWorkspaceRuntimeState,
  sessionId: SessionId,
  command: MuxCommand,
): Promise<MuxCommandResult> {
  if (!snapshot.attachedSession || snapshot.attachedSession.session.session_id !== sessionId) {
    return Promise.resolve({ changed: false });
  }

  if (command.kind === "send_input" || command.kind === "send_paste") {
    const nextSnapshot = appendStaticPreviewInput(
      snapshot,
      runtimeState,
      command.pane_id as PaneId,
      command.data,
      command.kind,
    );
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "new_tab") {
    updateSnapshot(createStaticPreviewTab(snapshot, runtimeState, command.title));
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "focus_tab") {
    const nextSnapshot = focusStaticPreviewTab(snapshot, runtimeState, command.tab_id);
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "rename_tab") {
    const nextSnapshot = renameStaticPreviewTab(snapshot, runtimeState, command.tab_id, command.title);
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "close_tab") {
    const nextSnapshot = closeStaticPreviewTab(snapshot, runtimeState, command.tab_id);
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "focus_pane") {
    const nextSnapshot = focusStaticPreviewPane(snapshot, runtimeState, command.pane_id as PaneId);
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "split_pane") {
    const nextSnapshot = splitStaticPreviewPane(
      snapshot,
      runtimeState,
      command.pane_id as PaneId,
      command.direction,
    );
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "close_pane") {
    const nextSnapshot = closeStaticPreviewPane(snapshot, runtimeState, command.pane_id as PaneId);
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "resize_pane") {
    const nextSnapshot = resizeStaticPreviewPane(
      snapshot,
      runtimeState,
      command.pane_id as PaneId,
      command.rows,
      command.cols,
    );
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "override_layout") {
    const nextSnapshot = overrideStaticPreviewLayout(
      snapshot,
      runtimeState,
      command.tab_id,
      command.root,
    );
    if (nextSnapshot === snapshot) {
      return Promise.resolve({ changed: false });
    }

    updateSnapshot(nextSnapshot);
    return Promise.resolve({ changed: true });
  }

  if (command.kind === "save_session") {
    updateSnapshot({
      ...snapshot,
      catalog: {
        ...snapshot.catalog,
        savedSessions: [createStaticSavedSessionSummary(snapshot), ...snapshot.catalog.savedSessions].slice(0, 6),
      },
    });
    return Promise.resolve({ changed: true });
  }

  return Promise.resolve({ changed: false });
}

function appendStaticPreviewInput(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  paneId: PaneId,
  data: string,
  kind: "send_input" | "send_paste",
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const focusedScreen = attachedSession?.focused_screen ?? null;
  if (!attachedSession || !focusedScreen || focusedScreen.pane_id !== paneId) {
    return snapshot;
  }

  const previewLines = formatStaticPreviewInputLines(data, kind);
  if (previewLines.length === 0) {
    return snapshot;
  }

  const nextLines = [
    ...focusedScreen.surface.lines,
    ...previewLines.map((text) => ({ text })),
  ].slice(-focusedScreen.rows);
  const nextScreen: ScreenSnapshot = {
    ...focusedScreen,
    sequence: focusedScreen.sequence + 1n,
    surface: {
      ...focusedScreen.surface,
      lines: nextLines,
    },
  };
  runtimeState.screensByPaneId.set(paneId, nextScreen);

  return {
    ...snapshot,
    attachedSession: {
      ...attachedSession,
      focused_screen: nextScreen,
    },
  };
}

function formatStaticPreviewInputLines(
  data: string,
  kind: "send_input" | "send_paste",
): string[] {
  if (data === "\u0003") {
    return ["^C"];
  }

  if (data === "\r" || data === "\n") {
    return [];
  }

  const normalizedData = data.replace(/\r\n/g, "\n").replace(/\r/g, "\n").trimEnd();
  const prefix = kind === "send_paste" ? "paste" : "$";
  const commands = normalizedData
    .split("\n")
    .filter((line) => line.length > 0)
    .map((line) => line.trimEnd());

  return commands.flatMap((line) => [
    `${prefix} ${line}`,
    ...resolveStaticPreviewCommandOutput(line),
  ]);
}

function resolveStaticPreviewCommandOutput(command: string): string[] {
  const normalizedCommand = command.trim();
  const normalizedLookup = normalizedCommand.toLowerCase();

  if (!normalizedLookup) {
    return [];
  }

  if (normalizedLookup === "git status") {
    return [
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
      "On branch demo/static-preview",
      "nothing to commit, working tree clean",
    ];
  }

  if (normalizedLookup === "pwd") {
    return [
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
      "/Users/demo/terminal-platform",
    ];
  }

  if (normalizedLookup === "ls -la") {
    return [
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
      "drwxr-xr-x  demo  apps",
      "drwxr-xr-x  demo  sdk",
      "-rw-r--r--  demo  README.md",
    ];
  }

  if (normalizedLookup === "node -v") {
    return [
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
      "v22.21.1",
    ];
  }

  if (normalizedLookup === "hello") {
    return [
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
      "hello from Terminal Platform static preview",
    ];
  }

  const printfOutput = resolveStaticPreviewPrintfOutput(normalizedCommand);
  if (printfOutput.length > 0) {
    return [
      ...printfOutput,
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
    ];
  }

  return [
    "static preview: command was not executed because no native host is attached",
    "run npm run dev:browser for a live local shell",
  ];
}

function resolveStaticPreviewPrintfOutput(command: string): string[] {
  const match = /^printf\s+(["'])(.*)\1$/u.exec(command);
  if (!match) {
    return [];
  }

  const rendered = (match[2] ?? "")
    .replace(/\\n/gu, "\n")
    .replace(/\\r/gu, "\r")
    .replace(/\\t/gu, "\t")
    .replace(/\\"/gu, "\"")
    .replace(/\\'/gu, "'");

  return rendered
    .replace(/\s+$/u, "")
    .split(/\r?\n/u)
    .filter((line) => line.length > 0);
}

function createStaticWorkspaceRuntimeState(snapshot: WorkspaceSnapshot): StaticWorkspaceRuntimeState {
  const screensByPaneId = new Map<PaneId, ScreenSnapshot>();
  const focusedScreen = snapshot.attachedSession?.focused_screen ?? null;
  if (focusedScreen) {
    screensByPaneId.set(focusedScreen.pane_id as PaneId, focusedScreen);
  }

  return {
    nextTopologyIndex: countStaticPreviewPanes(snapshot.attachedSession?.topology.tabs ?? []),
    screensByPaneId,
  };
}

function createStaticPreviewTab(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  title: string | null,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  if (!attachedSession) {
    return snapshot;
  }

  const tabNumber = attachedSession.topology.tabs.length + 1;
  const tabTitle = normalizeStaticPreviewTabTitle(title, tabNumber);
  const topologyIndex = nextStaticPreviewTopologyIndex(runtimeState, attachedSession.topology.tabs);
  const paneId = `preview-pane-${topologyIndex}` as PaneId;
  const tabId = `preview-tab-${topologyIndex}`;
  const screen = createStaticPreviewScreen(
    paneId,
    tabTitle,
    [
      `static preview tab: ${tabTitle}`,
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
      "run npm run dev:browser for live native terminal tabs",
    ],
    attachedSession.focused_screen,
  );
  const tab: TabSnapshot = {
    tab_id: tabId,
    title: tabTitle,
    focused_pane: paneId,
    root: {
      kind: "leaf",
      pane_id: paneId,
    },
  };

  runtimeState.screensByPaneId.set(paneId, screen);

  return updateStaticPreviewAttachedSession(snapshot, {
    ...attachedSession.topology,
    tabs: [...attachedSession.topology.tabs, tab],
    focused_tab: tabId,
  }, screen);
}

function focusStaticPreviewTab(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  tabId: string,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const tab = attachedSession?.topology.tabs.find((candidate) => candidate.tab_id === tabId) ?? null;
  if (!attachedSession || !tab) {
    return snapshot;
  }

  const paneId = (tab.focused_pane ?? firstStaticPreviewPaneId(tab.root)) as PaneId;
  const focusedTab: TabSnapshot = { ...tab, focused_pane: paneId };
  const screen = getOrCreateStaticPreviewScreen(runtimeState, paneId, focusedTab.title, attachedSession.focused_screen);

  return updateStaticPreviewAttachedSession(snapshot, {
    ...attachedSession.topology,
    tabs: attachedSession.topology.tabs.map((candidate) =>
      candidate.tab_id === tabId ? focusedTab : candidate
    ),
    focused_tab: tabId,
  }, screen);
}

function renameStaticPreviewTab(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  tabId: string,
  title: string,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const tab = attachedSession?.topology.tabs.find((candidate) => candidate.tab_id === tabId) ?? null;
  if (!attachedSession || !tab) {
    return snapshot;
  }

  const nextTitle = normalizeStaticPreviewTabTitle(title, attachedSession.topology.tabs.length);
  const nextTabs = attachedSession.topology.tabs.map((candidate) =>
    candidate.tab_id === tabId ? { ...candidate, title: nextTitle } : candidate
  );
  const focusedScreen = attachedSession.focused_screen;
  const nextFocusedScreen = attachedSession.topology.focused_tab === tabId && focusedScreen
    ? {
        ...focusedScreen,
        surface: {
          ...focusedScreen.surface,
          title: nextTitle,
        },
      }
    : focusedScreen;
  if (nextFocusedScreen) {
    runtimeState.screensByPaneId.set(nextFocusedScreen.pane_id as PaneId, nextFocusedScreen);
  }

  return {
    ...snapshot,
    attachedSession: {
      ...attachedSession,
      topology: {
        ...attachedSession.topology,
        tabs: nextTabs,
      },
      focused_screen: nextFocusedScreen,
    },
  };
}

function closeStaticPreviewTab(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  tabId: string,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const tabIndex = attachedSession?.topology.tabs.findIndex((candidate) => candidate.tab_id === tabId) ?? -1;
  if (!attachedSession || tabIndex < 0 || attachedSession.topology.tabs.length <= 1) {
    return snapshot;
  }

  const closedTab = attachedSession.topology.tabs[tabIndex]!;
  for (const paneId of collectStaticPreviewPaneIds(closedTab.root)) {
    runtimeState.screensByPaneId.delete(paneId as PaneId);
  }

  const tabs = attachedSession.topology.tabs.filter((candidate) => candidate.tab_id !== tabId);
  const focusedTab = attachedSession.topology.focused_tab === tabId
    ? tabs[Math.min(tabIndex, tabs.length - 1)] ?? tabs[0]
    : tabs.find((candidate) => candidate.tab_id === attachedSession.topology.focused_tab) ?? tabs[0];
  if (!focusedTab) {
    return snapshot;
  }

  const focusedPaneId = (focusedTab.focused_pane ?? firstStaticPreviewPaneId(focusedTab.root)) as PaneId;
  const nextFocusedTab: TabSnapshot = { ...focusedTab, focused_pane: focusedPaneId };
  const screen = getOrCreateStaticPreviewScreen(
    runtimeState,
    focusedPaneId,
    nextFocusedTab.title,
    attachedSession.focused_screen,
  );

  return updateStaticPreviewAttachedSession(snapshot, {
    ...attachedSession.topology,
    tabs: tabs.map((candidate) => candidate.tab_id === nextFocusedTab.tab_id ? nextFocusedTab : candidate),
    focused_tab: nextFocusedTab.tab_id,
  }, screen);
}

function splitStaticPreviewPane(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  paneId: PaneId,
  direction: SplitDirection,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const tab = attachedSession?.topology.tabs.find((candidate) => containsStaticPreviewPane(candidate.root, paneId))
    ?? null;
  if (!attachedSession || !tab) {
    return snapshot;
  }

  const nextPaneId = nextStaticPreviewPaneId(runtimeState, attachedSession.topology.tabs);
  const nextRoot = splitStaticPreviewPaneTree(tab.root, paneId, direction, nextPaneId);
  if (!nextRoot) {
    return snapshot;
  }

  const screen = createStaticPreviewScreen(
    nextPaneId,
    tab.title ?? "Split pane",
    [
      `static preview split: ${direction}`,
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
    ],
    attachedSession.focused_screen,
  );
  runtimeState.screensByPaneId.set(nextPaneId, screen);

  return updateStaticPreviewAttachedSession(snapshot, {
    ...attachedSession.topology,
    tabs: attachedSession.topology.tabs.map((candidate) =>
      candidate.tab_id === tab.tab_id
        ? {
            ...candidate,
            root: nextRoot,
            focused_pane: nextPaneId,
          }
        : candidate
    ),
    focused_tab: tab.tab_id,
  }, screen);
}

function focusStaticPreviewPane(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  paneId: PaneId,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const tab = attachedSession?.topology.tabs.find((candidate) => containsStaticPreviewPane(candidate.root, paneId))
    ?? null;
  if (!attachedSession || !tab) {
    return snapshot;
  }

  const screen = getOrCreateStaticPreviewScreen(runtimeState, paneId, tab.title, attachedSession.focused_screen);
  return updateStaticPreviewAttachedSession(snapshot, {
    ...attachedSession.topology,
    tabs: attachedSession.topology.tabs.map((candidate) =>
      candidate.tab_id === tab.tab_id ? { ...candidate, focused_pane: paneId } : candidate
    ),
    focused_tab: tab.tab_id,
  }, screen);
}

function closeStaticPreviewPane(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  paneId: PaneId,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const tab = attachedSession?.topology.tabs.find((candidate) => containsStaticPreviewPane(candidate.root, paneId))
    ?? null;
  if (!attachedSession || !tab || countStaticPreviewPaneLeaves(tab.root) <= 1) {
    return snapshot;
  }

  const nextRoot = removeStaticPreviewPaneLeaf(tab.root, paneId);
  if (!nextRoot) {
    return snapshot;
  }

  runtimeState.screensByPaneId.delete(paneId);
  const nextPaneId = tab.focused_pane === paneId || !containsStaticPreviewPane(nextRoot, tab.focused_pane as PaneId)
    ? firstStaticPreviewPaneId(nextRoot)
    : tab.focused_pane as PaneId;
  const screen = getOrCreateStaticPreviewScreen(runtimeState, nextPaneId, tab.title, attachedSession.focused_screen);

  return updateStaticPreviewAttachedSession(snapshot, {
    ...attachedSession.topology,
    tabs: attachedSession.topology.tabs.map((candidate) =>
      candidate.tab_id === tab.tab_id
        ? {
            ...candidate,
            root: nextRoot,
            focused_pane: nextPaneId,
          }
        : candidate
    ),
    focused_tab: tab.tab_id,
  }, screen);
}

function resizeStaticPreviewPane(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  paneId: PaneId,
  rows: number,
  cols: number,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  if (!attachedSession || !attachedSession.topology.tabs.some((tab) => containsStaticPreviewPane(tab.root, paneId))) {
    return snapshot;
  }

  const currentScreen = getOrCreateStaticPreviewScreen(runtimeState, paneId, null, attachedSession.focused_screen);
  const nextRows = Math.max(1, Math.trunc(rows));
  const nextCols = Math.max(1, Math.trunc(cols));
  if (currentScreen.rows === nextRows && currentScreen.cols === nextCols) {
    return snapshot;
  }

  const nextScreen: ScreenSnapshot = {
    ...currentScreen,
    sequence: currentScreen.sequence + 1n,
    rows: nextRows,
    cols: nextCols,
    surface: {
      ...currentScreen.surface,
      lines: currentScreen.surface.lines.slice(-nextRows),
      cursor: currentScreen.surface.cursor
        ? {
            row: Math.min(currentScreen.surface.cursor.row, nextRows - 1),
            col: Math.min(currentScreen.surface.cursor.col, nextCols - 1),
          }
        : null,
    },
  };
  runtimeState.screensByPaneId.set(paneId, nextScreen);

  if (attachedSession.focused_screen?.pane_id !== paneId) {
    return snapshot;
  }

  return {
    ...snapshot,
    attachedSession: {
      ...attachedSession,
      focused_screen: nextScreen,
    },
  };
}

function overrideStaticPreviewLayout(
  snapshot: WorkspaceSnapshot,
  runtimeState: StaticWorkspaceRuntimeState,
  tabId: string,
  root: PaneTreeNode,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  const tab = attachedSession?.topology.tabs.find((candidate) => candidate.tab_id === tabId) ?? null;
  if (!attachedSession || !tab) {
    return snapshot;
  }

  const nextPaneId = firstStaticPreviewPaneId(root);
  for (const paneId of collectStaticPreviewPaneIds(root)) {
    getOrCreateStaticPreviewScreen(runtimeState, paneId as PaneId, tab.title, attachedSession.focused_screen);
  }
  const screen = getOrCreateStaticPreviewScreen(runtimeState, nextPaneId, tab.title, attachedSession.focused_screen);

  return updateStaticPreviewAttachedSession(snapshot, {
    ...attachedSession.topology,
    tabs: attachedSession.topology.tabs.map((candidate) =>
      candidate.tab_id === tabId
        ? {
            ...candidate,
            root,
            focused_pane: nextPaneId,
          }
        : candidate
    ),
    focused_tab: tabId,
  }, screen);
}

function updateStaticPreviewAttachedSession(
  snapshot: WorkspaceSnapshot,
  topology: NonNullable<WorkspaceSnapshot["attachedSession"]>["topology"],
  focusedScreen: ScreenSnapshot,
): WorkspaceSnapshot {
  const attachedSession = snapshot.attachedSession;
  if (!attachedSession) {
    return snapshot;
  }

  return {
    ...snapshot,
    selection: {
      ...snapshot.selection,
      activePaneId: focusedScreen.pane_id,
    },
    attachedSession: {
      ...attachedSession,
      topology,
      focused_screen: focusedScreen,
    },
  };
}

function normalizeStaticPreviewTabTitle(title: string | null, tabNumber: number): string {
  const normalizedTitle = title?.trim();
  return normalizedTitle ? normalizedTitle : `Tab ${tabNumber}`;
}

function nextStaticPreviewPaneId(
  runtimeState: StaticWorkspaceRuntimeState,
  tabs: readonly TabSnapshot[],
): PaneId {
  return nextStaticPreviewTopologyId(runtimeState, tabs, "pane") as PaneId;
}

function nextStaticPreviewTopologyId(
  runtimeState: StaticWorkspaceRuntimeState,
  tabs: readonly TabSnapshot[],
  kind: "pane" | "tab",
): string {
  const existingIds = new Set(
    kind === "tab"
      ? tabs.map((tab) => tab.tab_id)
      : tabs.flatMap((tab) => collectStaticPreviewPaneIds(tab.root)),
  );

  let candidate = "";
  do {
    runtimeState.nextTopologyIndex += 1;
    candidate = `preview-${kind}-${runtimeState.nextTopologyIndex}`;
  } while (existingIds.has(candidate));

  return candidate;
}

function nextStaticPreviewTopologyIndex(
  runtimeState: StaticWorkspaceRuntimeState,
  tabs: readonly TabSnapshot[],
): number {
  const existingIds = new Set([
    ...tabs.map((tab) => tab.tab_id),
    ...tabs.flatMap((tab) => collectStaticPreviewPaneIds(tab.root)),
  ]);

  do {
    runtimeState.nextTopologyIndex += 1;
  } while (
    existingIds.has(`preview-tab-${runtimeState.nextTopologyIndex}`)
    || existingIds.has(`preview-pane-${runtimeState.nextTopologyIndex}`)
  );

  return runtimeState.nextTopologyIndex;
}

function createStaticPreviewScreen(
  paneId: PaneId,
  title: string | null,
  lines: string[],
  referenceScreen: ScreenSnapshot | null,
): ScreenSnapshot {
  const rows = referenceScreen?.rows ?? 24;
  return {
    pane_id: paneId,
    sequence: 1n,
    rows,
    cols: referenceScreen?.cols ?? 96,
    source: "native_emulator",
    surface: {
      title,
      cursor: null,
      lines: lines.map((text) => ({ text })).slice(-rows),
    },
  };
}

function getOrCreateStaticPreviewScreen(
  runtimeState: StaticWorkspaceRuntimeState,
  paneId: PaneId,
  title: string | null,
  referenceScreen: ScreenSnapshot | null,
): ScreenSnapshot {
  const screen = runtimeState.screensByPaneId.get(paneId);
  if (screen) {
    return screen;
  }

  const nextScreen = createStaticPreviewScreen(
    paneId,
    title,
    [
      `static preview pane: ${paneId}`,
      STATIC_PREVIEW_SIMULATED_OUTPUT_NOTICE,
    ],
    referenceScreen,
  );
  runtimeState.screensByPaneId.set(paneId, nextScreen);
  return nextScreen;
}

function splitStaticPreviewPaneTree(
  node: PaneTreeNode,
  paneId: PaneId,
  direction: SplitDirection,
  nextPaneId: PaneId,
): PaneTreeNode | null {
  if (node.kind === "leaf") {
    return node.pane_id === paneId
      ? {
          kind: "split",
          direction,
          first: node,
          second: {
            kind: "leaf",
            pane_id: nextPaneId,
          },
        }
      : null;
  }

  const first = splitStaticPreviewPaneTree(node.first, paneId, direction, nextPaneId);
  if (first) {
    return { ...node, first };
  }

  const second = splitStaticPreviewPaneTree(node.second, paneId, direction, nextPaneId);
  return second ? { ...node, second } : null;
}

function removeStaticPreviewPaneLeaf(node: PaneTreeNode, paneId: PaneId): PaneTreeNode | null {
  if (node.kind === "leaf") {
    return node.pane_id === paneId ? null : node;
  }

  const first = removeStaticPreviewPaneLeaf(node.first, paneId);
  if (!first) {
    return node.second;
  }

  const second = removeStaticPreviewPaneLeaf(node.second, paneId);
  return second ? { ...node, first, second } : node.first;
}

function containsStaticPreviewPane(node: PaneTreeNode, paneId: PaneId | null): boolean {
  if (!paneId) {
    return false;
  }

  if (node.kind === "leaf") {
    return node.pane_id === paneId;
  }

  return containsStaticPreviewPane(node.first, paneId) || containsStaticPreviewPane(node.second, paneId);
}

function firstStaticPreviewPaneId(node: PaneTreeNode): PaneId {
  if (node.kind === "leaf") {
    return node.pane_id as PaneId;
  }

  return firstStaticPreviewPaneId(node.first);
}

function collectStaticPreviewPaneIds(node: PaneTreeNode): string[] {
  if (node.kind === "leaf") {
    return [node.pane_id];
  }

  return [...collectStaticPreviewPaneIds(node.first), ...collectStaticPreviewPaneIds(node.second)];
}

function countStaticPreviewPaneLeaves(node: PaneTreeNode): number {
  if (node.kind === "leaf") {
    return 1;
  }

  return countStaticPreviewPaneLeaves(node.first) + countStaticPreviewPaneLeaves(node.second);
}

function countStaticPreviewPanes(tabs: readonly TabSnapshot[]): number {
  return tabs.reduce((count, tab) => count + countStaticPreviewPaneLeaves(tab.root), 0);
}

function createStaticSavedSessionSummary(snapshot: WorkspaceSnapshot): SavedSessionSummary {
  const session = snapshot.attachedSession?.session ?? snapshot.catalog.sessions[0];
  if (!session) {
    throw new Error("Static preview cannot save a layout without a session");
  }

  return {
    session_id: "preview-saved-session" as SessionId,
    route: session.route,
    title: "Preview saved layout",
    saved_at_ms: BigInt(Date.now()),
    manifest: {
      format_version: 1,
      binary_version: "preview",
      protocol_major: snapshot.connection.handshake?.protocol_version.major ?? 0,
      protocol_minor: snapshot.connection.handshake?.protocol_version.minor ?? 2,
    },
    compatibility: {
      can_restore: true,
      status: "compatible",
    },
    has_launch: true,
    tab_count: snapshot.attachedSession?.topology.tabs.length ?? 1,
    pane_count: countStaticPreviewPanes(snapshot.attachedSession?.topology.tabs ?? []),
    restore_semantics: {
      restores_topology: true,
      restores_focus_state: true,
      restores_tab_titles: true,
      uses_saved_launch_spec: true,
      replays_saved_screen_buffers: false,
      preserves_process_state: false,
    },
  };
}

export function createDemoPreviewWorkspaceSnapshot(config: TerminalDemoPreviewBootstrapConfig): WorkspaceSnapshot {
  const sessionId = "preview-session-native" as SessionId;
  const paneId = "preview-pane-main";
  const tabId = "preview-tab-shell";
  const session = {
    session_id: sessionId,
    origin: {
      backend: "native" as const,
      authority: "local_daemon" as const,
      foreign_reference_label: null,
    },
    route: {
      backend: "native" as const,
      authority: "local_daemon" as const,
      external: null,
    },
    title: "Terminal Platform preview",
    degraded_semantics: [],
  };

  return {
    ...createInitialWorkspaceSnapshot({
      terminalFontScale: DEFAULT_TERMINAL_DEMO_DISPLAY.fontScale,
      terminalLineWrap: DEFAULT_TERMINAL_DEMO_DISPLAY.lineWrap,
    }),
    connection: {
      state: "ready",
      handshake: {
        protocol_version: { major: 0, minor: 2 },
        binary_version: "preview",
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
        session_scope: config.runtimeSlug,
      },
      lastError: null,
    },
    catalog: {
      sessions: [session],
      savedSessions: [],
      discoveredSessions: {},
      backendCapabilities: {
        native: createDemoPreviewBackendCapabilities("native"),
      },
    },
    selection: {
      activeSessionId: sessionId,
      activePaneId: paneId,
    },
    attachedSession: {
      session,
      health: {
        session_id: sessionId,
        phase: "ready",
        can_attach: true,
        invalidated: false,
        reason: null,
        detail: null,
      },
      topology: {
        session_id: sessionId,
        backend_kind: "native",
        focused_tab: tabId,
        tabs: [
          {
            tab_id: tabId,
            title: "Shell",
            focused_pane: paneId,
            root: {
              kind: "leaf",
              pane_id: paneId,
            },
          },
        ],
      },
      focused_screen: {
        pane_id: paneId,
        sequence: 1n,
        rows: 24,
        cols: 96,
        source: "native_emulator",
        surface: {
          title: "Shell",
          cursor: null,
          lines: [
            { text: "terminal-platform demo static preview" },
            { text: "$ npm run build:renderer" },
            { text: "[ok] renderer bundle ready" },
            { text: "$ npm run smoke:browser" },
            { text: "waiting for native runtime and Chrome CDP..." },
          ],
        },
      },
    },
    drafts: {
      [paneId]: "git status",
    },
    commandHistory: {
      entries: ["npm run build:renderer", "npm run smoke:browser", "git status"],
      limit: DEFAULT_COMMAND_HISTORY_LIMIT,
    },
  };
}

export function createDemoPreviewBackendCapabilities(backend: BackendKind): BackendCapabilitiesInfo {
  return {
    backend,
    capabilities: {
      tiled_panes: true,
      floating_panes: false,
      split_resize: true,
      tab_create: true,
      tab_close: true,
      tab_focus: true,
      tab_rename: true,
      session_scoped_tab_refs: true,
      session_scoped_pane_refs: true,
      pane_split: true,
      pane_close: true,
      pane_focus: true,
      pane_input_write: true,
      pane_paste_write: true,
      raw_output_stream: false,
      rendered_viewport_stream: true,
      rendered_viewport_snapshot: true,
      rendered_scrollback_snapshot: false,
      layout_dump: true,
      layout_override: true,
      read_only_client_mode: false,
      explicit_session_save: true,
      explicit_session_restore: true,
      plugin_panes: false,
      advisory_metadata_subscriptions: true,
      independent_resize_authority: true,
    },
  };
}
