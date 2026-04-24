import type {
  BackendCapabilitiesInfo,
  PaneTreeNode,
  SessionId,
  TabSnapshot,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

export interface TerminalTopologyControlState {
  activeSessionId: SessionId | null;
  activeTab: TabSnapshot | null;
  activePaneId: string | null;
  capabilityStatus: "known" | "unknown";
  canCreateTab: boolean;
  canClosePane: boolean;
  canCloseTab: boolean;
  canFocusPane: boolean;
  canFocusTab: boolean;
  canRenameTab: boolean;
  canSplitPane: boolean;
  paneCount: number;
  tabCount: number;
}

export function resolveTerminalTopologyControlState(
  snapshot: WorkspaceSnapshot,
): TerminalTopologyControlState {
  const topology = snapshot.attachedSession?.topology ?? null;
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
  const backend = topology?.backend_kind ?? snapshot.attachedSession?.session.route.backend ?? null;
  const capabilities = backend ? snapshot.catalog.backendCapabilities[backend] ?? null : null;
  const paneCount = activeTab ? countPaneTreeLeaves(activeTab.root) : 0;
  const tabCount = topology?.tabs.length ?? 0;

  return {
    activeSessionId,
    activeTab,
    activePaneId,
    capabilityStatus: capabilities ? "known" : "unknown",
    canCreateTab: Boolean(activeSessionId && capabilityEnabled(capabilities, "tab_create")),
    canClosePane: Boolean(activeSessionId && activePaneId && paneCount > 1 && capabilityEnabled(capabilities, "pane_close")),
    canCloseTab: Boolean(activeSessionId && activeTab && tabCount > 1 && capabilityEnabled(capabilities, "tab_close")),
    canFocusPane: Boolean(activeSessionId && capabilityEnabled(capabilities, "pane_focus")),
    canFocusTab: Boolean(activeSessionId && capabilityEnabled(capabilities, "tab_focus")),
    canRenameTab: Boolean(activeSessionId && activeTab && capabilityEnabled(capabilities, "tab_rename")),
    canSplitPane: Boolean(activeSessionId && activePaneId && capabilityEnabled(capabilities, "pane_split")),
    paneCount,
    tabCount,
  };
}

export function countPaneTreeLeaves(node: PaneTreeNode): number {
  if (node.kind === "leaf") {
    return 1;
  }

  return countPaneTreeLeaves(node.first) + countPaneTreeLeaves(node.second);
}

export function compactTerminalId(id: string): string {
  if (id.length <= 18) {
    return id;
  }

  return `${id.slice(0, 8)}...${id.slice(-6)}`;
}

function capabilityEnabled(
  capabilities: BackendCapabilitiesInfo | null,
  key: keyof BackendCapabilitiesInfo["capabilities"],
): boolean {
  return capabilities?.capabilities[key] ?? true;
}
