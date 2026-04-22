export type TerminalWorkspaceCatalogTone = "brand" | "neutral" | "danger";

export interface TerminalWorkspaceCatalogBadgeModel {
  label: string;
  tone: TerminalWorkspaceCatalogTone;
}

export interface TerminalWorkspaceCatalogDegradedReasonModel {
  id: string;
  badge: TerminalWorkspaceCatalogBadgeModel;
  detail: string;
}

export interface TerminalWorkspaceCatalogDefinitionItemModel {
  label: string;
  value: string;
}

export interface TerminalWorkspaceCatalogFormModel {
  title: string;
  program: string;
  args: string;
  cwd: string;
}

export interface TerminalWorkspaceCatalogSessionItemModel {
  sessionId: string;
  title: string;
  meta: string;
  isActive: boolean;
}

export interface TerminalWorkspaceCatalogDiscoveredSessionModel {
  importHandle: string;
  backend: "tmux" | "zellij";
  title: string;
  sourceLabel: string;
  degradedReasons: TerminalWorkspaceCatalogDegradedReasonModel[];
}

export interface TerminalWorkspaceCatalogDiscoveredGroupModel {
  key: string;
  label: string;
  sessions: TerminalWorkspaceCatalogDiscoveredSessionModel[];
  emptyText: string;
}

export interface TerminalWorkspaceCatalogModel {
  statusBadge: TerminalWorkspaceCatalogBadgeModel;
  sessionStatusBadge: TerminalWorkspaceCatalogBadgeModel;
  sessionStreamBadge: TerminalWorkspaceCatalogBadgeModel;
  gatewayInfo: TerminalWorkspaceCatalogDefinitionItemModel[];
  handshakeDegradedReasons: TerminalWorkspaceCatalogDegradedReasonModel[];
  createForm: TerminalWorkspaceCatalogFormModel;
  sessionItems: TerminalWorkspaceCatalogSessionItemModel[];
  discoveredGroups: TerminalWorkspaceCatalogDiscoveredGroupModel[];
}
