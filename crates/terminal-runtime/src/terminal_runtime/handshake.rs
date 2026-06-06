use terminal_domain::{CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR};

use super::TerminalRuntime;
use crate::{RuntimeCapabilities, RuntimeHandshake, RuntimePhase, RuntimeProtocolVersion};

impl TerminalRuntime {
    #[must_use]
    pub fn handshake(&self) -> RuntimeHandshake {
        RuntimeHandshake {
            protocol_version: RuntimeProtocolVersion {
                major: CURRENT_PROTOCOL_MAJOR,
                minor: CURRENT_PROTOCOL_MINOR,
            },
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            daemon_phase: RuntimePhase::Ready,
            capabilities: RuntimeCapabilities {
                request_reply: true,
                topology_subscriptions: true,
                pane_subscriptions: true,
                backend_discovery: true,
                backend_capability_queries: true,
                saved_sessions: true,
                session_restore: true,
                degraded_error_reasons: true,
                session_health: true,
            },
            available_backends: self.sessions.available_backends(),
            session_scope: "current_user".to_string(),
        }
    }
}
