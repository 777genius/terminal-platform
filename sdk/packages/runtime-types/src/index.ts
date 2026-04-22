import type { NodeScreenSnapshot } from "./generated/raw/NodeScreenSnapshot.js";
import type { NodeSessionSummary } from "./generated/raw/NodeSessionSummary.js";
import type { NodeSubscriptionMeta } from "./generated/raw/NodeSubscriptionMeta.js";
import type { NodeTabSnapshot } from "./generated/raw/NodeTabSnapshot.js";

export const RUNTIME_TYPES_SCHEMA_VERSION = 1 as const;

export type { NodeAttachedSession as AttachedSession } from "./generated/raw/NodeAttachedSession.js";
export type {
  NodeBackendCapabilities as BackendCapabilities,
} from "./generated/raw/NodeBackendCapabilities.js";
export type {
  NodeBackendCapabilitiesInfo as BackendCapabilitiesInfo,
} from "./generated/raw/NodeBackendCapabilitiesInfo.js";
export type { NodeBackendKind as BackendKind } from "./generated/raw/NodeBackendKind.js";
export type {
  NodeCreateSessionRequest as CreateSessionRequest,
} from "./generated/raw/NodeCreateSessionRequest.js";
export type {
  NodeDaemonCapabilities as DaemonCapabilities,
} from "./generated/raw/NodeDaemonCapabilities.js";
export type { NodeDaemonPhase as DaemonPhase } from "./generated/raw/NodeDaemonPhase.js";
export type {
  NodeDeleteSavedSessionResult as DeleteSavedSessionResult,
} from "./generated/raw/NodeDeleteSavedSessionResult.js";
export type { NodeDiscoveredSession as DiscoveredSession } from "./generated/raw/NodeDiscoveredSession.js";
export type {
  NodeExternalSessionRef as ExternalSessionRef,
} from "./generated/raw/NodeExternalSessionRef.js";
export type { NodeHandshake as Handshake } from "./generated/raw/NodeHandshake.js";
export type { NodeMuxCommand as MuxCommand } from "./generated/raw/NodeMuxCommand.js";
export type {
  NodeMuxCommandResult as MuxCommandResult,
} from "./generated/raw/NodeMuxCommandResult.js";
export type { NodeNewTabCommand as NewTabCommand } from "./generated/raw/NodeNewTabCommand.js";
export type {
  NodeOverrideLayoutCommand as OverrideLayoutCommand,
} from "./generated/raw/NodeOverrideLayoutCommand.js";
export type { NodePaneSplit as PaneSplit } from "./generated/raw/NodePaneSplit.js";
export type { NodePaneTreeNode as PaneTreeNode } from "./generated/raw/NodePaneTreeNode.js";
export type {
  NodeProjectionSource as ProjectionSource,
} from "./generated/raw/NodeProjectionSource.js";
export type {
  NodeProtocolCompatibility as ProtocolCompatibility,
} from "./generated/raw/NodeProtocolCompatibility.js";
export type {
  NodeProtocolCompatibilityStatus as ProtocolCompatibilityStatus,
} from "./generated/raw/NodeProtocolCompatibilityStatus.js";
export type { NodeProtocolVersion as ProtocolVersion } from "./generated/raw/NodeProtocolVersion.js";
export type {
  NodePruneSavedSessionsResult as PruneSavedSessionsResult,
} from "./generated/raw/NodePruneSavedSessionsResult.js";
export type {
  NodeRenameTabCommand as RenameTabCommand,
} from "./generated/raw/NodeRenameTabCommand.js";
export type {
  NodeResizePaneCommand as ResizePaneCommand,
} from "./generated/raw/NodeResizePaneCommand.js";
export type { NodeRestoredSession as RestoredSession } from "./generated/raw/NodeRestoredSession.js";
export type { NodeRouteAuthority as RouteAuthority } from "./generated/raw/NodeRouteAuthority.js";
export type { NodeSavedSessionCompatibility as SavedSessionCompatibility } from "./generated/raw/NodeSavedSessionCompatibility.js";
export type {
  NodeSavedSessionCompatibilityStatus as SavedSessionCompatibilityStatus,
} from "./generated/raw/NodeSavedSessionCompatibilityStatus.js";
export type { NodeSavedSessionManifest as SavedSessionManifest } from "./generated/raw/NodeSavedSessionManifest.js";
export type { NodeSavedSessionRecord as SavedSessionRecord } from "./generated/raw/NodeSavedSessionRecord.js";
export type {
  NodeSavedSessionRestoreSemantics as SavedSessionRestoreSemantics,
} from "./generated/raw/NodeSavedSessionRestoreSemantics.js";
export type { NodeSavedSessionSummary as SavedSessionSummary } from "./generated/raw/NodeSavedSessionSummary.js";
export type { NodeScreenCursor as ScreenCursor } from "./generated/raw/NodeScreenCursor.js";
export type { NodeScreenDelta as ScreenDelta } from "./generated/raw/NodeScreenDelta.js";
export type { NodeScreenLine as ScreenLine } from "./generated/raw/NodeScreenLine.js";
export type { NodeScreenLinePatch as ScreenLinePatch } from "./generated/raw/NodeScreenLinePatch.js";
export type { NodeScreenPatch as ScreenPatch } from "./generated/raw/NodeScreenPatch.js";
export type { NodeScreenSnapshot as ScreenSnapshot } from "./generated/raw/NodeScreenSnapshot.js";
export type { NodeScreenSurface as ScreenSurface } from "./generated/raw/NodeScreenSurface.js";
export type { NodeSendInputCommand as SendInputCommand } from "./generated/raw/NodeSendInputCommand.js";
export type { NodeSendPasteCommand as SendPasteCommand } from "./generated/raw/NodeSendPasteCommand.js";
export type { NodeSessionRoute as SessionRoute } from "./generated/raw/NodeSessionRoute.js";
export type { NodeSessionSummary as SessionSummary } from "./generated/raw/NodeSessionSummary.js";
export type { NodeShellLaunchSpec as ShellLaunchSpec } from "./generated/raw/NodeShellLaunchSpec.js";
export type { NodeSplitDirection as SplitDirection } from "./generated/raw/NodeSplitDirection.js";
export type { NodeSplitPaneCommand as SplitPaneCommand } from "./generated/raw/NodeSplitPaneCommand.js";
export type {
  NodeSubscriptionEvent as SubscriptionEvent,
} from "./generated/raw/NodeSubscriptionEvent.js";
export type { NodeSubscriptionMeta as SubscriptionMeta } from "./generated/raw/NodeSubscriptionMeta.js";
export type { NodeSubscriptionSpec as SubscriptionSpec } from "./generated/raw/NodeSubscriptionSpec.js";
export type { NodeTabSnapshot as TabSnapshot } from "./generated/raw/NodeTabSnapshot.js";
export type { NodeTopologySnapshot as TopologySnapshot } from "./generated/raw/NodeTopologySnapshot.js";

export type SessionId = NodeSessionSummary["session_id"];
export type PaneId = NodeScreenSnapshot["pane_id"];
export type TabId = NodeTabSnapshot["tab_id"];
export type SubscriptionId = NodeSubscriptionMeta["subscription_id"];
