use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BoxFuture, CreateSessionSpec, DiscoveredSession, MuxBackendPort,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, SessionId, SessionRoute, local_native_route,
    local_native_session_id,
};

use super::{attached_session::NativeAttachedSession, capabilities::native_capabilities};
use crate::engine::NativeSessionEngine;

#[derive(Default)]
pub struct NativeBackend {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<NativeSessionEngine>>>>,
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
        Box::pin(async { Ok(native_capabilities()) })
    }

    fn discover_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "native backend is created through canonical session creation",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }

    fn create_session(
        &self,
        spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async move {
            let session_id = SessionId::new();
            let route = local_native_route(session_id);
            let runtime = Arc::new(NativeSessionEngine::spawn(session_id, route.clone(), spec)?);

            let mut sessions = self
                .sessions
                .write()
                .map_err(|_| BackendError::internal("native backend write lock poisoned"))?;
            sessions.insert(session_id, runtime);

            Ok(BackendSessionBinding { session_id, route })
        })
    }

    fn attach_session(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async move {
            let runtime = self.resolve_session_runtime(session_id, route)?;
            Ok(Box::new(NativeAttachedSession::new(runtime)) as Box<dyn BackendSessionPort>)
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async move {
            let sessions = self
                .sessions
                .read()
                .map_err(|_| BackendError::internal("native backend read lock poisoned"))?;
            let mut summaries = Vec::with_capacity(sessions.len());
            for runtime in sessions.values() {
                summaries.push(runtime.summary()?);
            }
            Ok(summaries)
        })
    }
}

impl NativeBackend {
    fn resolve_session_runtime(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> Result<Arc<NativeSessionEngine>, BackendError> {
        if route.backend != BackendKind::Native {
            return Err(BackendError::invalid_input(
                "native backend can only attach native routes",
            ));
        }

        let sessions = self
            .sessions
            .read()
            .map_err(|_| BackendError::internal("native backend read lock poisoned"))?;

        if let Some(route_session_id) = local_native_session_id(&route) {
            if route_session_id != session_id {
                return Err(BackendError::invalid_input(
                    "native attach session id does not match route identity",
                ));
            }
            return sessions
                .get(&route_session_id)
                .map(Arc::clone)
                .ok_or_else(|| BackendError::not_found("native route is not registered"));
        }

        sessions
            .values()
            .find(|runtime| runtime.summary().is_ok_and(|summary| summary.route == route))
            .map(Arc::clone)
            .ok_or_else(|| BackendError::not_found("native route is not registered"))
    }
}
