use serde::{Deserialize, Serialize};

use terminal_domain::BackendKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonPhase {
    Starting,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonCapabilities {
    pub request_reply: bool,
    pub topology_subscriptions: bool,
    pub pane_subscriptions: bool,
    pub backend_discovery: bool,
    pub backend_capability_queries: bool,
    pub saved_sessions: bool,
    pub session_restore: bool,
    pub degraded_error_reasons: bool,
    #[serde(default)]
    pub session_health: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    pub protocol_version: ProtocolVersion,
    pub binary_version: String,
    pub daemon_phase: DaemonPhase,
    pub capabilities: DaemonCapabilities,
    pub available_backends: Vec<BackendKind>,
    pub session_scope: String,
}
