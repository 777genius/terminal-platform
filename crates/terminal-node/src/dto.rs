use serde::{Deserialize, Serialize};
use terminal_backend_api::{BackendSessionSummary, CreateSessionSpec, ShellLaunchSpec};
use terminal_daemon_client::{HandshakeAssessment, HandshakeAssessmentStatus};
use terminal_domain::{
    BackendKind, ProtocolCompatibility, ProtocolCompatibilityStatus, RouteAuthority, SessionRoute,
};
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};
use terminal_projection::{
    ProjectionSource, ScreenCursor, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
};
use terminal_protocol::{DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeBindingVersion {
    pub binding_version: String,
    pub protocol: NodeProtocolVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeHandshakeInfo {
    pub handshake: NodeHandshake,
    pub assessment: NodeHandshakeAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeProtocolCompatibilityStatus {
    Compatible,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeProtocolCompatibility {
    pub can_connect: bool,
    pub status: NodeProtocolCompatibilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeHandshakeAssessmentStatus {
    Ready,
    Starting,
    Degraded,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeHandshakeAssessment {
    pub can_use: bool,
    pub protocol: NodeProtocolCompatibility,
    pub status: NodeHandshakeAssessmentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeDaemonPhase {
    Starting,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeDaemonCapabilities {
    pub request_reply: bool,
    pub topology_subscriptions: bool,
    pub pane_subscriptions: bool,
    pub backend_discovery: bool,
    pub backend_capability_queries: bool,
    pub saved_sessions: bool,
    pub session_restore: bool,
    pub degraded_error_reasons: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeHandshake {
    pub protocol_version: NodeProtocolVersion,
    pub binary_version: String,
    pub daemon_phase: NodeDaemonPhase,
    pub capabilities: NodeDaemonCapabilities,
    pub available_backends: Vec<NodeBackendKind>,
    pub session_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeBackendKind {
    Native,
    Tmux,
    Zellij,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeRouteAuthority {
    LocalDaemon,
    ImportedForeign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeExternalSessionRef {
    pub namespace: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSessionRoute {
    pub backend: NodeBackendKind,
    pub authority: NodeRouteAuthority,
    pub external: Option<NodeExternalSessionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeShellLaunchSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeCreateSessionRequest {
    pub title: Option<String>,
    pub launch: Option<NodeShellLaunchSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSessionSummary {
    pub session_id: String,
    pub route: NodeSessionRoute,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeProjectionSource {
    NativeEmulator,
    NativeTranscript,
    TmuxCapturePane,
    TmuxRawOutputImport,
    ZellijViewportSubscribe,
    ZellijDumpSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenCursor {
    pub row: u16,
    pub col: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLine {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenSurface {
    pub title: Option<String>,
    pub cursor: Option<NodeScreenCursor>,
    pub lines: Vec<NodeScreenLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenSnapshot {
    pub pane_id: String,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: NodeProjectionSource,
    pub surface: NodeScreenSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneSplit {
    pub direction: NodeSplitDirection,
    pub first: Box<NodePaneTreeNode>,
    pub second: Box<NodePaneTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodePaneTreeNode {
    Leaf { pane_id: String },
    Split(NodePaneSplit),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeTabSnapshot {
    pub tab_id: String,
    pub title: Option<String>,
    pub root: NodePaneTreeNode,
    pub focused_pane: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeTopologySnapshot {
    pub session_id: String,
    pub backend_kind: NodeBackendKind,
    pub tabs: Vec<NodeTabSnapshot>,
    pub focused_tab: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeAttachedSession {
    pub session: NodeSessionSummary,
    pub topology: NodeTopologySnapshot,
    pub focused_screen: Option<NodeScreenSnapshot>,
}

impl NodeBindingVersion {
    #[must_use]
    pub fn current(protocol: &ProtocolVersion) -> Self {
        Self { binding_version: env!("CARGO_PKG_VERSION").to_string(), protocol: protocol.into() }
    }
}

impl From<&ProtocolVersion> for NodeProtocolVersion {
    fn from(value: &ProtocolVersion) -> Self {
        Self { major: value.major, minor: value.minor }
    }
}

impl From<&ProtocolCompatibility> for NodeProtocolCompatibility {
    fn from(value: &ProtocolCompatibility) -> Self {
        Self { can_connect: value.can_connect, status: (&value.status).into() }
    }
}

impl From<&ProtocolCompatibilityStatus> for NodeProtocolCompatibilityStatus {
    fn from(value: &ProtocolCompatibilityStatus) -> Self {
        match value {
            ProtocolCompatibilityStatus::Compatible => Self::Compatible,
            ProtocolCompatibilityStatus::ProtocolMajorUnsupported => Self::ProtocolMajorUnsupported,
            ProtocolCompatibilityStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&HandshakeAssessment> for NodeHandshakeAssessment {
    fn from(value: &HandshakeAssessment) -> Self {
        Self {
            can_use: value.can_use,
            protocol: (&value.protocol).into(),
            status: (&value.status).into(),
        }
    }
}

impl From<&HandshakeAssessmentStatus> for NodeHandshakeAssessmentStatus {
    fn from(value: &HandshakeAssessmentStatus) -> Self {
        match value {
            HandshakeAssessmentStatus::Ready => Self::Ready,
            HandshakeAssessmentStatus::Starting => Self::Starting,
            HandshakeAssessmentStatus::Degraded => Self::Degraded,
            HandshakeAssessmentStatus::ProtocolMajorUnsupported => Self::ProtocolMajorUnsupported,
            HandshakeAssessmentStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&Handshake> for NodeHandshake {
    fn from(value: &Handshake) -> Self {
        Self {
            protocol_version: (&value.protocol_version).into(),
            binary_version: value.binary_version.clone(),
            daemon_phase: (&value.daemon_phase).into(),
            capabilities: (&value.capabilities).into(),
            available_backends: value.available_backends.iter().map(Into::into).collect(),
            session_scope: value.session_scope.clone(),
        }
    }
}

impl From<&DaemonPhase> for NodeDaemonPhase {
    fn from(value: &DaemonPhase) -> Self {
        match value {
            DaemonPhase::Starting => Self::Starting,
            DaemonPhase::Ready => Self::Ready,
            DaemonPhase::Degraded => Self::Degraded,
        }
    }
}

impl From<&DaemonCapabilities> for NodeDaemonCapabilities {
    fn from(value: &DaemonCapabilities) -> Self {
        Self {
            request_reply: value.request_reply,
            topology_subscriptions: value.topology_subscriptions,
            pane_subscriptions: value.pane_subscriptions,
            backend_discovery: value.backend_discovery,
            backend_capability_queries: value.backend_capability_queries,
            saved_sessions: value.saved_sessions,
            session_restore: value.session_restore,
            degraded_error_reasons: value.degraded_error_reasons,
        }
    }
}

impl From<&BackendKind> for NodeBackendKind {
    fn from(value: &BackendKind) -> Self {
        match value {
            BackendKind::Native => Self::Native,
            BackendKind::Tmux => Self::Tmux,
            BackendKind::Zellij => Self::Zellij,
        }
    }
}

impl From<&RouteAuthority> for NodeRouteAuthority {
    fn from(value: &RouteAuthority) -> Self {
        match value {
            RouteAuthority::LocalDaemon => Self::LocalDaemon,
            RouteAuthority::ImportedForeign => Self::ImportedForeign,
        }
    }
}

impl From<&SessionRoute> for NodeSessionRoute {
    fn from(value: &SessionRoute) -> Self {
        Self {
            backend: (&value.backend).into(),
            authority: (&value.authority).into(),
            external: value.external.as_ref().map(Into::into),
        }
    }
}

impl From<&terminal_domain::ExternalSessionRef> for NodeExternalSessionRef {
    fn from(value: &terminal_domain::ExternalSessionRef) -> Self {
        Self { namespace: value.namespace.clone(), value: value.value.clone() }
    }
}

impl From<&NodeShellLaunchSpec> for ShellLaunchSpec {
    fn from(value: &NodeShellLaunchSpec) -> Self {
        let mut spec = ShellLaunchSpec::new(value.program.clone()).with_args(value.args.clone());
        if let Some(cwd) = &value.cwd {
            spec = spec.with_cwd(cwd);
        }
        spec
    }
}

impl From<&ShellLaunchSpec> for NodeShellLaunchSpec {
    fn from(value: &ShellLaunchSpec) -> Self {
        Self {
            program: value.program.clone(),
            args: value.args.clone(),
            cwd: value.cwd.as_ref().map(|cwd| cwd.display().to_string()),
        }
    }
}

impl From<&NodeCreateSessionRequest> for CreateSessionSpec {
    fn from(value: &NodeCreateSessionRequest) -> Self {
        Self { title: value.title.clone(), launch: value.launch.as_ref().map(Into::into) }
    }
}

impl From<&BackendSessionSummary> for NodeSessionSummary {
    fn from(value: &BackendSessionSummary) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            route: (&value.route).into(),
            title: value.title.clone(),
        }
    }
}

impl From<&ProjectionSource> for NodeProjectionSource {
    fn from(value: &ProjectionSource) -> Self {
        match value {
            ProjectionSource::NativeEmulator => Self::NativeEmulator,
            ProjectionSource::NativeTranscript => Self::NativeTranscript,
            ProjectionSource::TmuxCapturePane => Self::TmuxCapturePane,
            ProjectionSource::TmuxRawOutputImport => Self::TmuxRawOutputImport,
            ProjectionSource::ZellijViewportSubscribe => Self::ZellijViewportSubscribe,
            ProjectionSource::ZellijDumpSnapshot => Self::ZellijDumpSnapshot,
        }
    }
}

impl From<&ScreenCursor> for NodeScreenCursor {
    fn from(value: &ScreenCursor) -> Self {
        Self { row: value.row, col: value.col }
    }
}

impl From<&ScreenLine> for NodeScreenLine {
    fn from(value: &ScreenLine) -> Self {
        Self { text: value.text.clone() }
    }
}

impl From<&ScreenSurface> for NodeScreenSurface {
    fn from(value: &ScreenSurface) -> Self {
        Self {
            title: value.title.clone(),
            cursor: value.cursor.as_ref().map(Into::into),
            lines: value.lines.iter().map(Into::into).collect(),
        }
    }
}

impl From<&ScreenSnapshot> for NodeScreenSnapshot {
    fn from(value: &ScreenSnapshot) -> Self {
        Self {
            pane_id: value.pane_id.0.to_string(),
            sequence: value.sequence,
            rows: value.rows,
            cols: value.cols,
            source: (&value.source).into(),
            surface: (&value.surface).into(),
        }
    }
}

impl From<&SplitDirection> for NodeSplitDirection {
    fn from(value: &SplitDirection) -> Self {
        match value {
            SplitDirection::Horizontal => Self::Horizontal,
            SplitDirection::Vertical => Self::Vertical,
        }
    }
}

impl From<&PaneSplit> for NodePaneSplit {
    fn from(value: &PaneSplit) -> Self {
        Self {
            direction: (&value.direction).into(),
            first: Box::new((&*value.first).into()),
            second: Box::new((&*value.second).into()),
        }
    }
}

impl From<&PaneTreeNode> for NodePaneTreeNode {
    fn from(value: &PaneTreeNode) -> Self {
        match value {
            PaneTreeNode::Leaf { pane_id } => Self::Leaf { pane_id: pane_id.0.to_string() },
            PaneTreeNode::Split(split) => Self::Split(split.into()),
        }
    }
}

impl From<&TabSnapshot> for NodeTabSnapshot {
    fn from(value: &TabSnapshot) -> Self {
        Self {
            tab_id: value.tab_id.0.to_string(),
            title: value.title.clone(),
            root: (&value.root).into(),
            focused_pane: value.focused_pane.map(|pane_id| pane_id.0.to_string()),
        }
    }
}

impl From<&TopologySnapshot> for NodeTopologySnapshot {
    fn from(value: &TopologySnapshot) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            backend_kind: (&value.backend_kind).into(),
            tabs: value.tabs.iter().map(Into::into).collect(),
            focused_tab: value.focused_tab.map(|tab_id| tab_id.0.to_string()),
        }
    }
}
