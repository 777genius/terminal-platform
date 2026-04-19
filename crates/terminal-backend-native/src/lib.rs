use std::{collections::HashMap, sync::RwLock};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BoxFuture, CreateSessionSpec, MuxBackendPort,
};
use terminal_domain::{BackendKind, DegradedModeReason, RouteAuthority, SessionId, SessionRoute};

#[derive(Default)]
pub struct NativeBackend {
    sessions: RwLock<HashMap<SessionId, BackendSessionSummary>>,
}

impl NativeBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Native
    }
}

impl MuxBackendPort for NativeBackend {
    fn kind(&self) -> BackendKind {
        self.kind()
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async {
            Ok(BackendCapabilities {
                tiled_panes: true,
                session_scoped_tab_refs: true,
                session_scoped_pane_refs: true,
                ..BackendCapabilities::default()
            })
        })
    }

    fn create_session(
        &self,
        spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async move {
            let session_id = SessionId::new();
            let route = SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            };
            let summary =
                BackendSessionSummary { session_id, route: route.clone(), title: spec.title };

            let mut sessions =
                self.sessions.write().expect("native backend write lock should not be poisoned");
            sessions.insert(session_id, summary);

            Ok(BackendSessionBinding { session_id, route })
        })
    }

    fn attach_session(
        &self,
        _route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "native attach session is not wired in v1 start phase",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async move {
            let sessions =
                self.sessions.read().expect("native backend read lock should not be poisoned");

            Ok(sessions.values().cloned().collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::{BackendScope, CreateSessionSpec, MuxBackendPort};

    use super::NativeBackend;

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_and_lists_empty_sessions() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec { title: Some("shell".to_string()) })
            .await
            .expect("create_session should succeed");
        let sessions = backend
            .list_sessions(BackendScope::CurrentUser)
            .await
            .expect("list_sessions should succeed");

        assert_eq!(binding.route.backend, terminal_domain::BackendKind::Native);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, binding.session_id);
        assert_eq!(sessions[0].title.as_deref(), Some("shell"));
    }
}
