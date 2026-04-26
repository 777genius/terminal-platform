import type { SessionId } from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  compactTerminalId,
  resolveTerminalTopologyControlState,
  type TerminalTopologyControlState,
} from "./terminal-topology-controls.js";

export interface TerminalTabStripControlOptions {
  armedCloseTabKey?: string | null;
  pending: boolean;
}

export interface TerminalTabStripItemControlState {
  active: boolean;
  canClose: boolean;
  canFocus: boolean;
  closeArmed: boolean;
  closeLabel: string;
  closeTitle: string;
  index: number;
  itemKey: string;
  label: string;
  metaLabel: string;
  tabId: string;
  title: string;
}

export interface TerminalTabStripControlState {
  activeSessionId: SessionId | null;
  canCloseTab: boolean;
  canCreateTab: boolean;
  canFocusTab: boolean;
  capabilityStatus: TerminalTopologyControlState["capabilityStatus"];
  tabCount: number;
  tabs: TerminalTabStripItemControlState[];
}

export function resolveTerminalTabStripControlState(
  snapshot: WorkspaceSnapshot,
  options: TerminalTabStripControlOptions,
): TerminalTabStripControlState {
  const topology = snapshot.attachedSession?.topology ?? null;
  const topologyControls = resolveTerminalTopologyControlState(snapshot);
  const activeTabId = topologyControls.activeTab?.tab_id ?? null;
  const activeTabIndex = topology?.tabs.findIndex((tab) => tab.tab_id === activeTabId) ?? -1;
  const canFocusTab = Boolean(!options.pending && topologyControls.canFocusTab);
  const canCloseTab = Boolean(!options.pending && topologyControls.canCloseTab);

  return {
    activeSessionId: topologyControls.activeSessionId,
    canCloseTab,
    canCreateTab: Boolean(!options.pending && topologyControls.canCreateTab),
    canFocusTab,
    capabilityStatus: topologyControls.capabilityStatus,
    tabCount: topologyControls.tabCount,
    tabs: topology?.tabs.map((tab, index) => {
      const itemKey = `${tab.tab_id}:${index}`;
      const metaLabel = compactTerminalId(tab.tab_id);
      const label = tab.title?.trim() || metaLabel;
      const closeArmed = options.armedCloseTabKey === itemKey;
      return {
        active: index === activeTabIndex,
        canClose: canCloseTab,
        canFocus: canFocusTab,
        closeArmed,
        closeLabel: closeArmed ? "Confirm close tab" : "Close tab",
        closeTitle: `${closeArmed ? "Confirm closing" : "Close"} tab ${label}`,
        index,
        itemKey,
        label,
        metaLabel,
        tabId: tab.tab_id,
        title: tab.tab_id,
      };
    }) ?? [],
  };
}
