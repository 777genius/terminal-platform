use serde::{Deserialize, Serialize};

use terminal_backend_api::BackendCapabilities;
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
pub struct Handshake {
    pub protocol_version: ProtocolVersion,
    pub binary_version: String,
    pub daemon_phase: DaemonPhase,
    pub capabilities: BackendCapabilities,
    pub available_backends: Vec<BackendKind>,
    pub session_scope: String,
}
