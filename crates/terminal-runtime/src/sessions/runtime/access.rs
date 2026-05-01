use terminal_backend_api::{BackendError, BackendSessionSummary, MuxBackendPort};
use terminal_domain::BackendKind;

use super::SessionRuntime;
use crate::registry::SessionDescriptor;

impl SessionRuntime<'_> {
    pub(in crate::sessions) fn available_backends(&self) -> Vec<BackendKind> {
        self.backends.kinds()
    }

    pub(in crate::sessions) fn session_count(&self) -> usize {
        self.registry.list().len()
    }

    pub(in crate::sessions) fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.registry.list().into_iter().map(Self::to_summary).collect()
    }

    pub(in crate::sessions) fn registry(&self) -> &dyn crate::registry::SessionRegistry {
        self.registry.as_ref()
    }

    pub(in crate::sessions) fn registry_handle(
        &self,
    ) -> std::sync::Arc<dyn crate::registry::SessionRegistry> {
        self.registry.clone()
    }

    pub(in crate::sessions) fn persistence(&self) -> &terminal_persistence::SqliteSessionStore {
        self.persistence
    }

    pub(in crate::sessions) fn backend(
        &self,
        kind: BackendKind,
    ) -> Result<std::sync::Arc<dyn MuxBackendPort>, BackendError> {
        self.backends.backend(kind)
    }

    pub(in crate::sessions) fn to_summary(session: SessionDescriptor) -> BackendSessionSummary {
        BackendSessionSummary {
            session_id: session.session_id,
            route: session.route,
            title: session.title,
        }
    }
}
