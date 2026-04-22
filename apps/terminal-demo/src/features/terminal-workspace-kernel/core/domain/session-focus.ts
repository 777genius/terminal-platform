import type { TerminalSessionState } from "../../contracts/terminal-workspace-contracts.js";

export function focusedPaneId(
  state: TerminalSessionState | null,
): string | null {
  if (!state) {
    return null;
  }

  const focusedTabId = state.topology.focused_tab;
  const focusedTab = state.topology.tabs.find(
    (tab: TerminalSessionState["topology"]["tabs"][number]) =>
      tab.tab_id === focusedTabId,
  );
  return focusedTab?.focused_pane ?? state.focusedScreen?.pane_id ?? null;
}
