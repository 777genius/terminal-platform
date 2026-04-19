use std::sync::Arc;

use terminal_application::SessionService;
use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, CreateSessionSpec,
};
use terminal_backend_native::NativeBackend;
use terminal_domain::BackendKind;
use terminal_protocol::{DaemonPhase, Handshake, ProtocolVersion};

pub struct TerminalDaemonState {
    sessions: SessionService,
}

impl Default for TerminalDaemonState {
    fn default() -> Self {
        Self { sessions: SessionService::new(Arc::new(NativeBackend::default())) }
    }
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
        self.sessions.session_count()
    }

    #[must_use]
    pub fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.sessions.list_sessions()
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.create_session(backend, spec).await
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::CreateSessionSpec;
    use terminal_domain::BackendKind;

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

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_native_session_summary() {
        let state = TerminalDaemonState::default();
        let created = state
            .create_session(
                BackendKind::Native,
                CreateSessionSpec { title: Some("shell".to_string()) },
            )
            .await
            .expect("native session create should succeed");

        assert_eq!(created.route.backend, BackendKind::Native);
        assert_eq!(created.title.as_deref(), Some("shell"));
        assert_eq!(state.session_count(), 1);
    }
}
