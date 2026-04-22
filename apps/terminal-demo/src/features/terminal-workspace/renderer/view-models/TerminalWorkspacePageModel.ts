export type TerminalWorkspaceTone = "brand" | "neutral" | "danger";
export type TerminalWorkspaceBannerTone = "default" | "subtle" | "warning";

export interface TerminalWorkspaceBadgeModel {
  label: string;
  tone: TerminalWorkspaceTone;
}

export interface TerminalWorkspaceBannerModel {
  tone: TerminalWorkspaceBannerTone;
  title: string | null;
  detail: string;
}

export interface TerminalWorkspaceDegradedReasonModel {
  id: string;
  badge: TerminalWorkspaceBadgeModel;
  detail: string;
}

export interface TerminalWorkspaceDefinitionItemModel {
  label: string;
  value: string;
}

export interface TerminalWorkspaceSessionItemModel {
  sessionId: string;
  title: string;
  meta: string;
  isActive: boolean;
}

export interface TerminalWorkspaceDiscoveredSessionModel {
  importHandle: string;
  backend: "tmux" | "zellij";
  title: string;
  sourceLabel: string;
  degradedReasons: TerminalWorkspaceDegradedReasonModel[];
}

export interface TerminalWorkspaceDiscoveredGroupModel {
  key: string;
  label: string;
  sessions: TerminalWorkspaceDiscoveredSessionModel[];
  emptyText: string;
}

export interface TerminalWorkspaceSavedSessionItemModel {
  sessionId: string;
  title: string;
  meta: string;
  degradedReasons: TerminalWorkspaceDegradedReasonModel[];
  canRestore: boolean;
}

export type TerminalWorkspacePaneTreeNodeModel =
  | {
      kind: "leaf";
      paneId: string;
      label: string;
      isFocused: boolean;
    }
  | {
      kind: "split";
      direction: "horizontal" | "vertical";
      first: TerminalWorkspacePaneTreeNodeModel;
      second: TerminalWorkspacePaneTreeNodeModel;
    };

export interface TerminalWorkspaceTopologyTabModel {
  tabId: string;
  title: string;
  isFocused: boolean;
  root: TerminalWorkspacePaneTreeNodeModel;
}

export interface TerminalWorkspaceScreenLineModel {
  key: string;
  gutter: string;
  text: string;
}

export interface TerminalWorkspaceScreenModel {
  sizeBadge: TerminalWorkspaceBadgeModel;
  sequenceBadge: TerminalWorkspaceBadgeModel;
  meta: string[];
  lines: TerminalWorkspaceScreenLineModel[];
}

export interface TerminalWorkspaceCapabilitiesModel {
  badges: TerminalWorkspaceBadgeModel[];
  degradedReasons: TerminalWorkspaceDegradedReasonModel[];
}

export interface TerminalWorkspaceToolbarModel {
  canNewTab: boolean;
  canSplit: boolean;
  canSave: boolean;
}

export interface TerminalWorkspaceCreateFormModel {
  title: string;
  program: string;
  args: string;
  cwd: string;
}

export interface TerminalWorkspaceInputModel {
  draft: string;
  canWrite: boolean;
}

export interface TerminalWorkspacePageModel {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
  statusBadge: TerminalWorkspaceBadgeModel;
  sessionStatusBadge: TerminalWorkspaceBadgeModel;
  sessionStreamBadge: TerminalWorkspaceBadgeModel;
  gatewayInfo: TerminalWorkspaceDefinitionItemModel[];
  handshakeDegradedReasons: TerminalWorkspaceDegradedReasonModel[];
  createForm: TerminalWorkspaceCreateFormModel;
  sessionItems: TerminalWorkspaceSessionItemModel[];
  discoveredGroups: TerminalWorkspaceDiscoveredGroupModel[];
  savedSessionItems: TerminalWorkspaceSavedSessionItemModel[];
  hiddenSavedSessionsCount: number;
  showAllSavedSessions: boolean;
  activeSessionTitle: string;
  activeBackendBadge: TerminalWorkspaceBadgeModel | null;
  errorBanner: TerminalWorkspaceBannerModel | null;
  sessionStreamBanner: TerminalWorkspaceBannerModel | null;
  actionDegradedBanner: TerminalWorkspaceBannerModel | null;
  actionErrorBanner: TerminalWorkspaceBannerModel | null;
  toolbar: TerminalWorkspaceToolbarModel;
  topologyTabs: TerminalWorkspaceTopologyTabModel[];
  screen: TerminalWorkspaceScreenModel | null;
  input: TerminalWorkspaceInputModel;
  capabilities: TerminalWorkspaceCapabilitiesModel | null;
}
