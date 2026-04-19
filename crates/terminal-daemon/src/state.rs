use terminal_application::{InMemorySessionRegistry, SessionRegistry};
use terminal_backend_api::BackendCapabilities;
use terminal_domain::BackendKind;
use terminal_protocol::{DaemonPhase, Handshake, ProtocolVersion};

#[derive(Debug, Default)]
pub struct TerminalDaemonState {
    registry: InMemorySessionRegistry,
}

impl TerminalDaemonState {
    #[must_use]
    pub fn handshake(&self) -> Handshake {
        Handshake {
            protocol_version: ProtocolVersion { major: 0, minor: 1 },
            binary_version: "0.1.0-dev".to_string(),
            daemon_phase: DaemonPhase::Starting,
            capabilities: BackendCapabilities::default(),
            available_backends: vec![BackendKind::Native, BackendKind::Tmux, BackendKind::Zellij],
            session_scope: "current_user".to_string(),
        }
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.registry.list().len()
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalDaemonState;

    #[test]
    fn exposes_starting_handshake_with_known_backends() {
        let state = TerminalDaemonState::default();
        let handshake = state.handshake();

        assert_eq!(handshake.protocol_version.major, 0);
        assert_eq!(handshake.protocol_version.minor, 1);
        assert_eq!(handshake.available_backends.len(), 3);
        assert_eq!(state.session_count(), 0);
    }
}
