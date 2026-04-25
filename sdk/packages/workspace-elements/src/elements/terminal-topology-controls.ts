import type {
  BackendCapabilitiesInfo,
  BackendKind,
  MuxCommand,
  PaneTreeNode,
  PaneId,
  SessionId,
  TabSnapshot,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

export { compactTerminalId } from "./terminal-identity.js";

export const TERMINAL_PANE_MIN_ROWS = 4;
export const TERMINAL_PANE_MIN_COLS = 20;
export const TERMINAL_PANE_MAX_ROWS = 80;
export const TERMINAL_PANE_MAX_COLS = 240;

export interface TerminalPaneSize {
  rows: number;
  cols: number;
}

export interface TerminalPaneResizeDelta {
  rows?: number;
  cols?: number;
}

export interface TerminalTopologyControlState {
  activeSessionId: SessionId | null;
  activeTab: TabSnapshot | null;
  activePaneId: string | null;
  activePaneSize: TerminalPaneSize | null;
  capabilityStatus: "known" | "unknown";
  canCreateTab: boolean;
  canClosePane: boolean;
  canCloseTab: boolean;
  canFocusPane: boolean;
  canFocusTab: boolean;
  canRenameTab: boolean;
  canResizePane: boolean;
  canSplitPane: boolean;
  paneCount: number;
  tabCount: number;
}

export function resolveTerminalTopologyControlState(
  snapshot: WorkspaceSnapshot,
): TerminalTopologyControlState {
  const topology = snapshot.attachedSession?.topology ?? null;
  const hasTopology = Boolean(topology && topology.tabs.length > 0);
  const activeSessionId = snapshot.selection.activeSessionId
    ?? snapshot.attachedSession?.session.session_id
    ?? null;
  const activeTab = topology?.tabs.find((tab) => tab.tab_id === topology.focused_tab)
    ?? topology?.tabs[0]
    ?? null;
  const activePaneId = snapshot.selection.activePaneId
    ?? activeTab?.focused_pane
    ?? snapshot.attachedSession?.focused_screen?.pane_id
    ?? null;
  const backend = topology?.backend_kind
    ?? snapshot.attachedSession?.session.route.backend
    ?? resolveCatalogSessionBackend(snapshot, activeSessionId);
  const capabilities = backend ? snapshot.catalog.backendCapabilities[backend] ?? null : null;
  const paneCount = activeTab ? countPaneTreeLeaves(activeTab.root) : 0;
  const tabCount = topology?.tabs.length ?? 0;
  const focusedScreen = snapshot.attachedSession?.focused_screen ?? null;
  const activePaneSize = focusedScreen?.pane_id === activePaneId
    ? {
        rows: focusedScreen.rows,
        cols: focusedScreen.cols,
      }
    : null;

  return {
    activeSessionId,
    activeTab,
    activePaneId,
    activePaneSize,
    capabilityStatus: capabilities ? "known" : "unknown",
    canCreateTab: Boolean(activeSessionId && hasTopology && capabilityEnabled(capabilities, "tab_create")),
    canClosePane: Boolean(activeSessionId && activePaneId && paneCount > 1 && capabilityEnabled(capabilities, "pane_close")),
    canCloseTab: Boolean(activeSessionId && activeTab && tabCount > 1 && capabilityEnabled(capabilities, "tab_close")),
    canFocusPane: Boolean(activeSessionId && activePaneId && capabilityEnabled(capabilities, "pane_focus")),
    canFocusTab: Boolean(activeSessionId && activeTab && capabilityEnabled(capabilities, "tab_focus")),
    canRenameTab: Boolean(activeSessionId && activeTab && capabilityEnabled(capabilities, "tab_rename")),
    canResizePane: Boolean(activeSessionId && activePaneId && activePaneSize && capabilityEnabled(capabilities, "split_resize")),
    canSplitPane: Boolean(activeSessionId && activePaneId && capabilityEnabled(capabilities, "pane_split")),
    paneCount,
    tabCount,
  };
}

export function resolvePaneResizeCommand(
  snapshot: WorkspaceSnapshot,
  delta: TerminalPaneResizeDelta,
): MuxCommand | null {
  const controls = resolveTerminalTopologyControlState(snapshot);
  if (!controls.activePaneId || !controls.activePaneSize || !controls.canResizePane) {
    return null;
  }

  const rows = clampTerminalDimension(
    controls.activePaneSize.rows + (delta.rows ?? 0),
    TERMINAL_PANE_MIN_ROWS,
    TERMINAL_PANE_MAX_ROWS,
  );
  const cols = clampTerminalDimension(
    controls.activePaneSize.cols + (delta.cols ?? 0),
    TERMINAL_PANE_MIN_COLS,
    TERMINAL_PANE_MAX_COLS,
  );

  if (rows === controls.activePaneSize.rows && cols === controls.activePaneSize.cols) {
    return null;
  }

  return {
    kind: "resize_pane",
    pane_id: controls.activePaneId as PaneId,
    rows,
    cols,
  };
}

export function canRunTerminalTopologyCommand(
  controls: TerminalTopologyControlState,
  command: MuxCommand,
): boolean {
  switch (command.kind) {
    case "new_tab":
      return controls.canCreateTab;
    case "split_pane":
      return controls.canSplitPane;
    case "resize_pane":
      return controls.canResizePane;
    case "focus_tab":
      return controls.canFocusTab;
    case "focus_pane":
      return controls.canFocusPane;
    case "rename_tab":
      return controls.canRenameTab;
    case "close_pane":
      return controls.canClosePane;
    case "close_tab":
      return controls.canCloseTab;
    default:
      return false;
  }
}

export function countPaneTreeLeaves(node: PaneTreeNode): number {
  if (node.kind === "leaf") {
    return 1;
  }

  return countPaneTreeLeaves(node.first) + countPaneTreeLeaves(node.second);
}

function capabilityEnabled(
  capabilities: BackendCapabilitiesInfo | null,
  key: keyof BackendCapabilitiesInfo["capabilities"],
): boolean {
  return capabilities?.capabilities[key] ?? true;
}

function resolveCatalogSessionBackend(
  snapshot: WorkspaceSnapshot,
  activeSessionId: SessionId | null,
): BackendKind | null {
  if (!activeSessionId) {
    return null;
  }

  return snapshot.catalog.sessions.find((session) => session.session_id === activeSessionId)?.route.backend ?? null;
}

function clampTerminalDimension(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, Math.trunc(value)));
}
