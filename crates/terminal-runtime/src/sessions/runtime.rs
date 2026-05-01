mod capture;
mod helpers;
#[cfg(test)]
mod tests;

use terminal_backend_api::{
    BackendError, BackendSessionPort, BackendSessionSummary, CreateSessionSpec, MuxBackendPort,
};
use terminal_domain::{BackendKind, SessionId, SessionRoute};
use terminal_persistence::{SessionRouteRecord, SqliteSessionStore};
use terminal_projection::SessionHealthSnapshot;
use tokio::sync::oneshot;

use crate::{
    backend_catalog::BackendCatalog,
    registry::{SessionDescriptor, SessionRegistry},
};

use capture::run_v2_history_capture;
use helpers::{session_health_from_attach_error, session_route_fingerprint};

pub(super) use helpers::{
    collect_pane_ids_from_node, collect_pane_ids_from_topology, command_updates_summary_title,
    saved_session_title, tab_snapshot_by_id,
};

const V2_RAW_CAPTURE_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
const V2_RAW_CAPTURE_MAX_BATCH_BYTES: usize = 64 * 1024;
const V2_RENDERED_CAPTURE_FLUSH_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(250);
const V2_CAPTURE_READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone)]
pub(super) struct SessionRuntime<'a> {
    backends: &'a BackendCatalog,
    registry: std::sync::Arc<dyn SessionRegistry>,
    persistence: &'a SqliteSessionStore,
}

impl<'a> SessionRuntime<'a> {
    pub(super) fn new(
        backends: &'a BackendCatalog,
        registry: std::sync::Arc<dyn SessionRegistry>,
        persistence: &'a SqliteSessionStore,
    ) -> Self {
        Self { backends, registry, persistence }
    }

    pub(super) fn available_backends(&self) -> Vec<BackendKind> {
        self.backends.kinds()
    }

    pub(super) fn session_count(&self) -> usize {
        self.registry.list().len()
    }

    pub(super) fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.registry.list().into_iter().map(Self::to_summary).collect()
    }

    pub(super) fn registry(&self) -> &dyn SessionRegistry {
        self.registry.as_ref()
    }

    pub(super) fn registry_handle(&self) -> std::sync::Arc<dyn SessionRegistry> {
        self.registry.clone()
    }

    pub(super) fn persistence(&self) -> &'a SqliteSessionStore {
        self.persistence
    }

    pub(super) fn backend(
        &self,
        kind: BackendKind,
    ) -> Result<std::sync::Arc<dyn MuxBackendPort>, BackendError> {
        self.backends.backend(kind)
    }

    pub(super) async fn create_native_session(
        &self,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        let binding = self.backend(BackendKind::Native)?.create_session(spec.clone()).await?;
        let descriptor = SessionDescriptor {
            session_id: binding.session_id,
            route: binding.route,
            title: spec.title.clone(),
            launch: spec.launch.clone(),
            health: SessionHealthSnapshot::ready(binding.session_id),
        };
        let summary = Self::to_summary(descriptor.clone());
        self.upsert_session_route(descriptor.session_id, &descriptor.route)?;
        self.registry.insert(descriptor);
        if let Ok(session) = self
            .backend(BackendKind::Native)?
            .attach_session(summary.session_id, summary.route.clone())
            .await
        {
            self.start_v2_history_capture(
                SessionDescriptor {
                    session_id: summary.session_id,
                    route: summary.route.clone(),
                    title: summary.title.clone(),
                    launch: spec.launch,
                    health: SessionHealthSnapshot::ready(summary.session_id),
                },
                session,
            )
            .await;
        }

        Ok(summary)
    }

    pub(super) async fn attach_session(
        &self,
        session_id: SessionId,
    ) -> Result<Box<dyn BackendSessionPort>, BackendError> {
        let descriptor = self
            .registry
            .get(session_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown session {session_id:?}")))?;

        match self
            .backend(descriptor.route.backend)?
            .attach_session(descriptor.session_id, descriptor.route)
            .await
        {
            Ok(session) => {
                self.mark_session_ready(session_id);
                Ok(session)
            }
            Err(error) => {
                if let Some(health) = session_health_from_attach_error(session_id, &error) {
                    self.record_session_health(session_id, health);
                }
                Err(error)
            }
        }
    }

    pub(super) async fn refresh_session_summary_title(
        &self,
        session_id: SessionId,
        session: &dyn BackendSessionPort,
    ) {
        let Some(descriptor) = self.registry.get(session_id) else {
            return;
        };
        let Ok(topology) = session.topology_snapshot().await else {
            return;
        };
        self.registry.update_title(session_id, saved_session_title(descriptor.title, &topology));
    }

    pub(super) fn to_summary(session: SessionDescriptor) -> BackendSessionSummary {
        BackendSessionSummary {
            session_id: session.session_id,
            route: session.route,
            title: session.title,
        }
    }

    pub(super) fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.registry
            .get(session_id)
            .map(|session| session.health)
            .ok_or_else(|| BackendError::not_found(format!("unknown session {session_id:?}")))
    }

    pub(super) fn record_session_health(
        &self,
        session_id: SessionId,
        health: SessionHealthSnapshot,
    ) {
        self.registry.update_health(session_id, health);
    }

    pub(super) fn mark_session_ready(&self, session_id: SessionId) {
        self.registry.update_health(session_id, SessionHealthSnapshot::ready(session_id));
    }

    pub(super) fn resolve_session_id_for_route(
        &self,
        route: &SessionRoute,
    ) -> Result<SessionId, BackendError> {
        let route_fingerprint = session_route_fingerprint(route);
        if let Some(record) = self
            .persistence
            .load_session_route_by_fingerprint(&route_fingerprint)
            .map_err(|error| {
                BackendError::internal(format!(
                    "failed to load session route by fingerprint - {error}"
                ))
            })?
        {
            return Ok(record.session_id);
        }

        let session_id = SessionId::new();
        self.upsert_session_route(session_id, route)?;
        Ok(session_id)
    }

    pub(super) fn upsert_session_route(
        &self,
        session_id: SessionId,
        route: &SessionRoute,
    ) -> Result<(), BackendError> {
        self.persistence
            .upsert_session_route(&SessionRouteRecord {
                session_id,
                route: route.clone(),
                route_fingerprint: session_route_fingerprint(route),
            })
            .map_err(|error| {
                BackendError::internal(format!("failed to persist session route - {error}"))
            })
    }

    pub(super) async fn start_v2_history_capture(
        &self,
        descriptor: SessionDescriptor,
        session: Box<dyn BackendSessionPort>,
    ) {
        let persistence = self.persistence.clone();
        let (ready_tx, ready_rx) = oneshot::channel();
        tokio::spawn(async move {
            run_v2_history_capture(persistence, descriptor, session, ready_tx).await;
        });
        let _ = tokio::time::timeout(V2_CAPTURE_READY_TIMEOUT, ready_rx).await;
    }
}
