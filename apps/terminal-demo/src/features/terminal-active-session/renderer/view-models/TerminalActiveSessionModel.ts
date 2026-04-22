export type TerminalActiveSessionTone = "brand" | "neutral" | "danger";
export type TerminalActiveSessionBannerTone = "default" | "subtle" | "warning";

export interface TerminalActiveSessionBadgeModel {
  label: string;
  tone: TerminalActiveSessionTone;
}

export interface TerminalActiveSessionBannerModel {
  tone: TerminalActiveSessionBannerTone;
  title: string | null;
  detail: string;
}

export interface TerminalActiveSessionDegradedReasonModel {
  id: string;
  badge: TerminalActiveSessionBadgeModel;
  detail: string;
}

export type TerminalActiveSessionPaneTreeNodeModel =
  | {
      kind: "leaf";
      paneId: string;
      label: string;
      isFocused: boolean;
    }
  | {
      kind: "split";
      direction: "horizontal" | "vertical";
      first: TerminalActiveSessionPaneTreeNodeModel;
      second: TerminalActiveSessionPaneTreeNodeModel;
    };

export interface TerminalActiveSessionTopologyTabModel {
  tabId: string;
  title: string;
  isFocused: boolean;
  root: TerminalActiveSessionPaneTreeNodeModel;
}

export interface TerminalActiveSessionScreenLineModel {
  key: string;
  gutter: string;
  text: string;
}

export interface TerminalActiveSessionScreenModel {
  sizeBadge: TerminalActiveSessionBadgeModel;
  sequenceBadge: TerminalActiveSessionBadgeModel;
  meta: string[];
  lines: TerminalActiveSessionScreenLineModel[];
}

export interface TerminalActiveSessionCapabilitiesModel {
  badges: TerminalActiveSessionBadgeModel[];
  degradedReasons: TerminalActiveSessionDegradedReasonModel[];
}

export interface TerminalActiveSessionToolbarModel {
  canNewTab: boolean;
  canSplit: boolean;
  canSave: boolean;
}

export interface TerminalActiveSessionModel {
  activeSessionTitle: string;
  activeBackendBadge: TerminalActiveSessionBadgeModel | null;
  errorBanner: TerminalActiveSessionBannerModel | null;
  sessionStreamBanner: TerminalActiveSessionBannerModel | null;
  actionDegradedBanner: TerminalActiveSessionBannerModel | null;
  actionErrorBanner: TerminalActiveSessionBannerModel | null;
  toolbar: TerminalActiveSessionToolbarModel;
  topologyTabs: TerminalActiveSessionTopologyTabModel[];
  screen: TerminalActiveSessionScreenModel | null;
  capabilities: TerminalActiveSessionCapabilitiesModel | null;
}
