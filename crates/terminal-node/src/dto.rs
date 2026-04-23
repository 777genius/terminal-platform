use serde::{Deserialize, Serialize};
use terminal_backend_api::{
    BackendCapabilities, BackendSessionSummary, CreateSessionSpec, DiscoveredSession, MuxCommand,
    MuxCommandResult, NewTabSpec, OverrideLayoutSpec, ResizePaneSpec, SendInputSpec, SendPasteSpec,
    ShellLaunchSpec, SplitPaneSpec, SubscriptionSpec,
};
use terminal_daemon_client::{HandshakeAssessment, HandshakeAssessmentStatus};
use terminal_domain::{
    BackendKind, PaneId, ProtocolCompatibility, ProtocolCompatibilityStatus, RouteAuthority,
    SavedSessionCompatibility, SavedSessionCompatibilityStatus, SavedSessionManifest, SessionRoute,
    SubscriptionId, TabId,
};
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};
use terminal_projection::{
    ProjectionSource, ScreenCursor, ScreenDelta, ScreenLine, ScreenLinePatch, ScreenPatch,
    ScreenSnapshot, ScreenSurface, SessionHealthPhase, SessionHealthReason, SessionHealthSnapshot,
    TopologySnapshot,
};
use terminal_protocol::{
    BackendCapabilitiesResponse, DaemonCapabilities, DaemonPhase, DeleteSavedSessionResponse,
    Handshake, ProtocolError, ProtocolVersion, PruneSavedSessionsResponse,
    RestoreSavedSessionResponse, SavedSessionRecord, SavedSessionRestoreSemantics,
    SavedSessionSummary, SubscriptionEvent,
};
use ts_rs::TS;
use uuid::Uuid;

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
    pub session_health: bool,
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
#[ts(export)]
pub struct NodeDiscoveredSession {
    pub route: NodeSessionRoute,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeBackendCapabilities {
    pub tiled_panes: bool,
    pub floating_panes: bool,
    pub split_resize: bool,
    pub tab_create: bool,
    pub tab_close: bool,
    pub tab_focus: bool,
    pub tab_rename: bool,
    pub session_scoped_tab_refs: bool,
    pub session_scoped_pane_refs: bool,
    pub pane_split: bool,
    pub pane_close: bool,
    pub pane_focus: bool,
    pub pane_input_write: bool,
    pub pane_paste_write: bool,
    pub raw_output_stream: bool,
    pub rendered_viewport_stream: bool,
    pub rendered_viewport_snapshot: bool,
    pub rendered_scrollback_snapshot: bool,
    pub layout_dump: bool,
    pub layout_override: bool,
    pub read_only_client_mode: bool,
    pub explicit_session_save: bool,
    pub explicit_session_restore: bool,
    pub plugin_panes: bool,
    pub advisory_metadata_subscriptions: bool,
    pub independent_resize_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeBackendCapabilitiesInfo {
    pub backend: NodeBackendKind,
    pub capabilities: NodeBackendCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSavedSessionCompatibilityStatus {
    Compatible,
    BinarySkew,
    FormatVersionUnsupported,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionManifest {
    pub format_version: u32,
    pub binary_version: String,
    pub protocol_major: u16,
    pub protocol_minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionCompatibility {
    pub can_restore: bool,
    pub status: NodeSavedSessionCompatibilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionRestoreSemantics {
    pub restores_topology: bool,
    pub restores_focus_state: bool,
    pub restores_tab_titles: bool,
    pub uses_saved_launch_spec: bool,
    pub replays_saved_screen_buffers: bool,
    pub preserves_process_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionSummary {
    pub session_id: String,
    pub route: NodeSessionRoute,
    pub title: Option<String>,
    pub saved_at_ms: i64,
    pub manifest: NodeSavedSessionManifest,
    pub compatibility: NodeSavedSessionCompatibility,
    pub has_launch: bool,
    pub tab_count: usize,
    pub pane_count: usize,
    pub restore_semantics: NodeSavedSessionRestoreSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionRecord {
    pub session_id: String,
    pub route: NodeSessionRoute,
    pub title: Option<String>,
    pub launch: Option<NodeShellLaunchSpec>,
    pub manifest: NodeSavedSessionManifest,
    pub compatibility: NodeSavedSessionCompatibility,
    pub topology: NodeTopologySnapshot,
    pub screens: Vec<NodeScreenSnapshot>,
    pub saved_at_ms: i64,
    pub restore_semantics: NodeSavedSessionRestoreSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeRestoredSession {
    pub saved_session_id: String,
    pub manifest: NodeSavedSessionManifest,
    pub compatibility: NodeSavedSessionCompatibility,
    pub session: NodeSessionSummary,
    pub restore_semantics: NodeSavedSessionRestoreSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeDeleteSavedSessionResult {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePruneSavedSessionsResult {
    pub deleted_count: usize,
    pub kept_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSubscriptionSpec {
    SessionTopology,
    PaneSurface { pane_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSessionHealthPhase {
    Ready,
    Degraded,
    Stale,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSessionHealthReason {
    BackendDegraded,
    SubscriptionSourceClosed,
    SessionNotFound,
    BackendTransportLost,
    BackendInternalFault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSessionHealthSnapshot {
    pub session_id: String,
    pub phase: NodeSessionHealthPhase,
    pub can_attach: bool,
    pub invalidated: bool,
    pub reason: Option<NodeSessionHealthReason>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSubscriptionEvent {
    TopologySnapshot(NodeTopologySnapshot),
    ScreenDelta(NodeScreenDelta),
    SessionHealthSnapshot(NodeSessionHealthSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSubscriptionMeta {
    pub subscription_id: String,
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
#[ts(export)]
pub struct NodeScreenLinePatch {
    pub row: u16,
    pub line: NodeScreenLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenPatch {
    pub title_changed: bool,
    pub title: Option<String>,
    pub cursor_changed: bool,
    pub cursor: Option<NodeScreenCursor>,
    pub line_updates: Vec<NodeScreenLinePatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenDelta {
    pub pane_id: String,
    pub from_sequence: u64,
    pub to_sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: NodeProjectionSource,
    pub patch: Option<NodeScreenPatch>,
    pub full_replace: Option<NodeScreenSurface>,
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
pub struct NodeSplitPaneCommand {
    pub pane_id: String,
    pub direction: NodeSplitDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeResizePaneCommand {
    pub pane_id: String,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeNewTabCommand {
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeRenameTabCommand {
    pub tab_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSendInputCommand {
    pub pane_id: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSendPasteCommand {
    pub pane_id: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeOverrideLayoutCommand {
    pub tab_id: String,
    pub root: NodePaneTreeNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodeMuxCommand {
    SplitPane(NodeSplitPaneCommand),
    ClosePane { pane_id: String },
    FocusPane { pane_id: String },
    ResizePane(NodeResizePaneCommand),
    NewTab(NodeNewTabCommand),
    CloseTab { tab_id: String },
    FocusTab { tab_id: String },
    RenameTab(NodeRenameTabCommand),
    SendInput(NodeSendInputCommand),
    SendPaste(NodeSendPasteCommand),
    Detach,
    SaveSession,
    OverrideLayout(NodeOverrideLayoutCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeMuxCommandResult {
    pub changed: bool,
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
    pub health: NodeSessionHealthSnapshot,
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
            session_health: value.session_health,
        }
    }
}

impl From<&SessionHealthPhase> for NodeSessionHealthPhase {
    fn from(value: &SessionHealthPhase) -> Self {
        match value {
            SessionHealthPhase::Ready => Self::Ready,
            SessionHealthPhase::Degraded => Self::Degraded,
            SessionHealthPhase::Stale => Self::Stale,
            SessionHealthPhase::Terminated => Self::Terminated,
        }
    }
}

impl From<&SessionHealthReason> for NodeSessionHealthReason {
    fn from(value: &SessionHealthReason) -> Self {
        match value {
            SessionHealthReason::BackendDegraded => Self::BackendDegraded,
            SessionHealthReason::SubscriptionSourceClosed => Self::SubscriptionSourceClosed,
            SessionHealthReason::SessionNotFound => Self::SessionNotFound,
            SessionHealthReason::BackendTransportLost => Self::BackendTransportLost,
            SessionHealthReason::BackendInternalFault => Self::BackendInternalFault,
        }
    }
}

impl From<&SessionHealthSnapshot> for NodeSessionHealthSnapshot {
    fn from(value: &SessionHealthSnapshot) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            phase: (&value.phase).into(),
            can_attach: value.can_attach,
            invalidated: value.invalidated,
            reason: value.reason.as_ref().map(Into::into),
            detail: value.detail.clone(),
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

impl From<&DiscoveredSession> for NodeDiscoveredSession {
    fn from(value: &DiscoveredSession) -> Self {
        Self { route: (&value.route).into(), title: value.title.clone() }
    }
}

impl From<&BackendCapabilities> for NodeBackendCapabilities {
    fn from(value: &BackendCapabilities) -> Self {
        Self {
            tiled_panes: value.tiled_panes,
            floating_panes: value.floating_panes,
            split_resize: value.split_resize,
            tab_create: value.tab_create,
            tab_close: value.tab_close,
            tab_focus: value.tab_focus,
            tab_rename: value.tab_rename,
            session_scoped_tab_refs: value.session_scoped_tab_refs,
            session_scoped_pane_refs: value.session_scoped_pane_refs,
            pane_split: value.pane_split,
            pane_close: value.pane_close,
            pane_focus: value.pane_focus,
            pane_input_write: value.pane_input_write,
            pane_paste_write: value.pane_paste_write,
            raw_output_stream: value.raw_output_stream,
            rendered_viewport_stream: value.rendered_viewport_stream,
            rendered_viewport_snapshot: value.rendered_viewport_snapshot,
            rendered_scrollback_snapshot: value.rendered_scrollback_snapshot,
            layout_dump: value.layout_dump,
            layout_override: value.layout_override,
            read_only_client_mode: value.read_only_client_mode,
            explicit_session_save: value.explicit_session_save,
            explicit_session_restore: value.explicit_session_restore,
            plugin_panes: value.plugin_panes,
            advisory_metadata_subscriptions: value.advisory_metadata_subscriptions,
            independent_resize_authority: value.independent_resize_authority,
        }
    }
}

impl From<&BackendCapabilitiesResponse> for NodeBackendCapabilitiesInfo {
    fn from(value: &BackendCapabilitiesResponse) -> Self {
        Self { backend: (&value.backend).into(), capabilities: (&value.capabilities).into() }
    }
}

impl From<&SavedSessionManifest> for NodeSavedSessionManifest {
    fn from(value: &SavedSessionManifest) -> Self {
        Self {
            format_version: value.format_version,
            binary_version: value.binary_version.clone(),
            protocol_major: value.protocol_major,
            protocol_minor: value.protocol_minor,
        }
    }
}

impl From<&SavedSessionCompatibilityStatus> for NodeSavedSessionCompatibilityStatus {
    fn from(value: &SavedSessionCompatibilityStatus) -> Self {
        match value {
            SavedSessionCompatibilityStatus::Compatible => Self::Compatible,
            SavedSessionCompatibilityStatus::BinarySkew => Self::BinarySkew,
            SavedSessionCompatibilityStatus::FormatVersionUnsupported => {
                Self::FormatVersionUnsupported
            }
            SavedSessionCompatibilityStatus::ProtocolMajorUnsupported => {
                Self::ProtocolMajorUnsupported
            }
            SavedSessionCompatibilityStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&SavedSessionCompatibility> for NodeSavedSessionCompatibility {
    fn from(value: &SavedSessionCompatibility) -> Self {
        Self { can_restore: value.can_restore, status: (&value.status).into() }
    }
}

impl From<&SavedSessionRestoreSemantics> for NodeSavedSessionRestoreSemantics {
    fn from(value: &SavedSessionRestoreSemantics) -> Self {
        Self {
            restores_topology: value.restores_topology,
            restores_focus_state: value.restores_focus_state,
            restores_tab_titles: value.restores_tab_titles,
            uses_saved_launch_spec: value.uses_saved_launch_spec,
            replays_saved_screen_buffers: value.replays_saved_screen_buffers,
            preserves_process_state: value.preserves_process_state,
        }
    }
}

impl From<&SavedSessionSummary> for NodeSavedSessionSummary {
    fn from(value: &SavedSessionSummary) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            route: (&value.route).into(),
            title: value.title.clone(),
            saved_at_ms: value.saved_at_ms,
            manifest: (&value.manifest).into(),
            compatibility: (&value.compatibility).into(),
            has_launch: value.has_launch,
            tab_count: value.tab_count,
            pane_count: value.pane_count,
            restore_semantics: (&value.restore_semantics).into(),
        }
    }
}

impl From<&SavedSessionRecord> for NodeSavedSessionRecord {
    fn from(value: &SavedSessionRecord) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            route: (&value.route).into(),
            title: value.title.clone(),
            launch: value.launch.as_ref().map(Into::into),
            manifest: (&value.manifest).into(),
            compatibility: (&value.compatibility).into(),
            topology: (&value.topology).into(),
            screens: value.screens.iter().map(Into::into).collect(),
            saved_at_ms: value.saved_at_ms,
            restore_semantics: (&value.restore_semantics).into(),
        }
    }
}

impl From<&RestoreSavedSessionResponse> for NodeRestoredSession {
    fn from(value: &RestoreSavedSessionResponse) -> Self {
        Self {
            saved_session_id: value.saved_session_id.0.to_string(),
            manifest: (&value.manifest).into(),
            compatibility: (&value.compatibility).into(),
            session: (&value.session).into(),
            restore_semantics: (&value.restore_semantics).into(),
        }
    }
}

impl From<&DeleteSavedSessionResponse> for NodeDeleteSavedSessionResult {
    fn from(value: &DeleteSavedSessionResponse) -> Self {
        Self { session_id: value.session_id.0.to_string() }
    }
}

impl From<&PruneSavedSessionsResponse> for NodePruneSavedSessionsResult {
    fn from(value: &PruneSavedSessionsResponse) -> Self {
        Self { deleted_count: value.deleted_count, kept_count: value.kept_count }
    }
}

impl From<&SubscriptionId> for NodeSubscriptionMeta {
    fn from(value: &SubscriptionId) -> Self {
        Self { subscription_id: value.0.to_string() }
    }
}

impl From<&SubscriptionEvent> for NodeSubscriptionEvent {
    fn from(value: &SubscriptionEvent) -> Self {
        match value {
            SubscriptionEvent::TopologySnapshot(snapshot) => {
                Self::TopologySnapshot(snapshot.into())
            }
            SubscriptionEvent::ScreenDelta(delta) => Self::ScreenDelta(delta.into()),
            SubscriptionEvent::SessionHealthSnapshot(snapshot) => {
                Self::SessionHealthSnapshot(snapshot.into())
            }
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

impl From<&ScreenLinePatch> for NodeScreenLinePatch {
    fn from(value: &ScreenLinePatch) -> Self {
        Self { row: value.row, line: (&value.line).into() }
    }
}

impl From<&ScreenPatch> for NodeScreenPatch {
    fn from(value: &ScreenPatch) -> Self {
        Self {
            title_changed: value.title_changed,
            title: value.title.clone(),
            cursor_changed: value.cursor_changed,
            cursor: value.cursor.as_ref().map(Into::into),
            line_updates: value.line_updates.iter().map(Into::into).collect(),
        }
    }
}

impl From<&ScreenDelta> for NodeScreenDelta {
    fn from(value: &ScreenDelta) -> Self {
        Self {
            pane_id: value.pane_id.0.to_string(),
            from_sequence: value.from_sequence,
            to_sequence: value.to_sequence,
            rows: value.rows,
            cols: value.cols,
            source: (&value.source).into(),
            patch: value.patch.as_ref().map(Into::into),
            full_replace: value.full_replace.as_ref().map(Into::into),
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

impl From<&NodeSplitDirection> for SplitDirection {
    fn from(value: &NodeSplitDirection) -> Self {
        match value {
            NodeSplitDirection::Horizontal => Self::Horizontal,
            NodeSplitDirection::Vertical => Self::Vertical,
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

impl From<&MuxCommandResult> for NodeMuxCommandResult {
    fn from(value: &MuxCommandResult) -> Self {
        Self { changed: value.changed }
    }
}

impl TryFrom<&NodeSessionRoute> for SessionRoute {
    type Error = ProtocolError;

    fn try_from(value: &NodeSessionRoute) -> Result<Self, Self::Error> {
        Ok(Self {
            backend: (&value.backend).into(),
            authority: (&value.authority).into(),
            external: value.external.as_ref().map(|external| terminal_domain::ExternalSessionRef {
                namespace: external.namespace.clone(),
                value: external.value.clone(),
            }),
        })
    }
}

impl From<&NodeBackendKind> for BackendKind {
    fn from(value: &NodeBackendKind) -> Self {
        match value {
            NodeBackendKind::Native => Self::Native,
            NodeBackendKind::Tmux => Self::Tmux,
            NodeBackendKind::Zellij => Self::Zellij,
        }
    }
}

impl From<&NodeRouteAuthority> for RouteAuthority {
    fn from(value: &NodeRouteAuthority) -> Self {
        match value {
            NodeRouteAuthority::LocalDaemon => Self::LocalDaemon,
            NodeRouteAuthority::ImportedForeign => Self::ImportedForeign,
        }
    }
}

impl TryFrom<&NodePaneTreeNode> for PaneTreeNode {
    type Error = ProtocolError;

    fn try_from(value: &NodePaneTreeNode) -> Result<Self, Self::Error> {
        match value {
            NodePaneTreeNode::Leaf { pane_id } => {
                Ok(Self::Leaf { pane_id: parse_pane_id(pane_id)? })
            }
            NodePaneTreeNode::Split(split) => Ok(Self::Split(PaneSplit {
                direction: (&split.direction).into(),
                first: Box::new((&*split.first).try_into()?),
                second: Box::new((&*split.second).try_into()?),
            })),
        }
    }
}

impl TryFrom<&NodeMuxCommand> for MuxCommand {
    type Error = ProtocolError;

    fn try_from(value: &NodeMuxCommand) -> Result<Self, Self::Error> {
        Ok(match value {
            NodeMuxCommand::SplitPane(command) => Self::SplitPane(SplitPaneSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                direction: (&command.direction).into(),
            }),
            NodeMuxCommand::ClosePane { pane_id } => {
                Self::ClosePane { pane_id: parse_pane_id(pane_id)? }
            }
            NodeMuxCommand::FocusPane { pane_id } => {
                Self::FocusPane { pane_id: parse_pane_id(pane_id)? }
            }
            NodeMuxCommand::ResizePane(command) => Self::ResizePane(ResizePaneSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                rows: command.rows,
                cols: command.cols,
            }),
            NodeMuxCommand::NewTab(command) => {
                Self::NewTab(NewTabSpec { title: command.title.clone() })
            }
            NodeMuxCommand::CloseTab { tab_id } => Self::CloseTab { tab_id: parse_tab_id(tab_id)? },
            NodeMuxCommand::FocusTab { tab_id } => Self::FocusTab { tab_id: parse_tab_id(tab_id)? },
            NodeMuxCommand::RenameTab(command) => Self::RenameTab {
                tab_id: parse_tab_id(&command.tab_id)?,
                title: command.title.clone(),
            },
            NodeMuxCommand::SendInput(command) => Self::SendInput(SendInputSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                data: command.data.clone(),
            }),
            NodeMuxCommand::SendPaste(command) => Self::SendPaste(SendPasteSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                data: command.data.clone(),
            }),
            NodeMuxCommand::Detach => Self::Detach,
            NodeMuxCommand::SaveSession => Self::SaveSession,
            NodeMuxCommand::OverrideLayout(command) => Self::OverrideLayout(OverrideLayoutSpec {
                tab_id: parse_tab_id(&command.tab_id)?,
                root: (&command.root).try_into()?,
            }),
        })
    }
}

impl TryFrom<&NodeSubscriptionSpec> for SubscriptionSpec {
    type Error = ProtocolError;

    fn try_from(value: &NodeSubscriptionSpec) -> Result<Self, Self::Error> {
        Ok(match value {
            NodeSubscriptionSpec::SessionTopology => Self::SessionTopology,
            NodeSubscriptionSpec::PaneSurface { pane_id } => {
                Self::PaneSurface { pane_id: parse_pane_id(pane_id)? }
            }
        })
    }
}

fn parse_pane_id(value: &str) -> Result<PaneId, ProtocolError> {
    parse_uuid(value, "invalid_pane_id", "pane").map(PaneId::from)
}

fn parse_tab_id(value: &str) -> Result<TabId, ProtocolError> {
    parse_uuid(value, "invalid_tab_id", "tab").map(TabId::from)
}

fn parse_uuid(value: &str, code: &str, label: &str) -> Result<Uuid, ProtocolError> {
    Uuid::parse_str(value).map_err(|error| {
        ProtocolError::new(code, format!("failed to parse {label} id '{value}' - {error}"))
    })
}
