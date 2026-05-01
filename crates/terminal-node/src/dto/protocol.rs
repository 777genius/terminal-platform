use super::prelude::*;

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
