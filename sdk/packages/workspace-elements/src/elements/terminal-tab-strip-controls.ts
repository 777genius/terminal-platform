import type { SessionId } from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  compactTerminalId,
  resolveTerminalTopologyControlState,
  type TerminalTopologyControlState,
} from "./terminal-topology-controls.js";

export interface TerminalTabStripControlOptions {
  pending: boolean;
}

export interface TerminalTabStripItemControlState {
  active: boolean;
  canFocus: boolean;
  label: string;
  metaLabel: string;
  tabId: string;
  title: string;
}

export interface TerminalTabStripControlState {
  activeSessionId: SessionId | null;
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

  return {
    activeSessionId: topologyControls.activeSessionId,
    canCreateTab: Boolean(!options.pending && topologyControls.canCreateTab),
    canFocusTab,
    capabilityStatus: topologyControls.capabilityStatus,
    tabCount: topologyControls.tabCount,
    tabs: topology?.tabs.map((tab, index) => {
      const metaLabel = compactTerminalId(tab.tab_id);
      return {
        active: index === activeTabIndex,
        canFocus: canFocusTab,
        label: tab.title?.trim() || metaLabel,
        metaLabel,
        tabId: tab.tab_id,
        title: tab.tab_id,
      };
    }) ?? [],
  };
}
