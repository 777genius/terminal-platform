mod client;
mod dto;
mod export;
mod ids;
mod subscription;

pub use client::NodeHostClient;
pub use export::{export_typescript_bindings, export_typescript_bindings_to};
pub use subscription::NodeSubscriptionHandle;

pub use dto::{
    NodeAttachedSession, NodeBackendCapabilities, NodeBackendCapabilitiesInfo, NodeBackendKind,
    NodeBindingVersion, NodeCommandHistoryEntry, NodeCreateSessionRequest, NodeDaemonCapabilities,
    NodeDaemonPhase, NodeDeleteSavedSessionResult, NodeDiscoveredSession, NodeExternalSessionRef,
    NodeHandshake, NodeHandshakeAssessment, NodeHandshakeAssessmentStatus, NodeHandshakeInfo,
    NodeMuxCommand, NodeMuxCommandResult, NodeNewTabCommand, NodeOverrideLayoutCommand,
    NodePaneHistory, NodePaneHistoryGap, NodePaneHistoryReplayStrategy,
    NodePaneHistoryRestoreEvidence, NodePaneHistoryRestorePlan, NodePaneHistoryScreenSnapshot,
    NodePaneHistorySegment, NodePaneSplit, NodePaneTreeNode, NodeProjectionSource,
    NodeProtocolCompatibility, NodeProtocolCompatibilityStatus, NodeProtocolVersion,
    NodePruneSavedSessionsResult, NodeRenameTabCommand, NodeResizePaneCommand, NodeRestoredSession,
    NodeRouteAuthority, NodeSavedSessionCompatibility, NodeSavedSessionCompatibilityStatus,
    NodeSavedSessionManifest, NodeSavedSessionRecord, NodeSavedSessionRestoreSemantics,
    NodeSavedSessionSummary, NodeScreenCursor, NodeScreenDelta, NodeScreenLine,
    NodeScreenLinePatch, NodeScreenPatch, NodeScreenSnapshot, NodeScreenSurface,
    NodeSendInputCommand, NodeSendPasteCommand, NodeSessionHealthPhase, NodeSessionHealthReason,
    NodeSessionHealthSnapshot, NodeSessionRoute, NodeSessionSummary, NodeShellLaunchSpec,
    NodeSplitDirection, NodeSplitPaneCommand, NodeSubscriptionEvent, NodeSubscriptionMeta,
    NodeSubscriptionSpec, NodeTabSnapshot, NodeTopologySnapshot,
};

#[cfg(test)]
mod tests;
