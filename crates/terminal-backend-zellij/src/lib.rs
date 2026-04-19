use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BoxFuture, CreateSessionSpec, DiscoveredSession, MuxBackendPort,
};
use terminal_domain::{BackendKind, DegradedModeReason, SessionRoute};

#[derive(Debug, Default)]
pub struct ZellijBackend;

impl ZellijBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }
}

impl MuxBackendPort for ZellijBackend {
    fn kind(&self) -> BackendKind {
        self.kind()
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async { Ok(BackendCapabilities::default()) })
    }

    fn discover_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij adapter is not implemented yet",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }

    fn create_session(
        &self,
        _spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij sessions are not creatable in the current rollout phase",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }

    fn attach_session(
        &self,
        _route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij routes are not attachable in the current rollout phase",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij backend does not expose canonical sessions directly",
                DegradedModeReason::ImportedForeignSession,
            ))
        })
    }
}
