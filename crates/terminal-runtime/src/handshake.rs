use terminal_domain::BackendKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePhase {
    Starting,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilities {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandshake {
    pub protocol_version: RuntimeProtocolVersion,
    pub binary_version: String,
    pub daemon_phase: RuntimePhase,
    pub capabilities: RuntimeCapabilities,
    pub available_backends: Vec<BackendKind>,
    pub session_scope: String,
}
