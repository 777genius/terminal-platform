use terminal_backend_api::{
    BackendError, BackendErrorKind, BackendSessionPort, BackendSessionSummary, CreateSessionSpec,
    MuxBackendPort, MuxCommand,
};
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute, TabId};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
use terminal_persistence::{SessionRouteRecord, SqliteSessionStore};
use terminal_projection::{SessionHealthReason, SessionHealthSnapshot, TopologySnapshot};

use crate::{
    backend_catalog::BackendCatalog,
    registry::{SessionDescriptor, SessionRegistry},
};

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
            title: spec.title,
            launch: spec.launch,
            health: SessionHealthSnapshot::ready(binding.session_id),
        };
        let summary = Self::to_summary(descriptor.clone());
        self.upsert_session_route(descriptor.session_id, &descriptor.route)?;
        self.registry.insert(descriptor);

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
}

pub(super) fn session_health_from_attach_error(
    session_id: SessionId,
    error: &BackendError,
) -> Option<SessionHealthSnapshot> {
    match error.kind {
        BackendErrorKind::Unsupported => error.degraded_reason.as_ref().map(|_| {
            SessionHealthSnapshot::degraded(
                session_id,
                SessionHealthReason::BackendDegraded,
                error.message.clone(),
            )
        }),
        BackendErrorKind::NotFound => Some(SessionHealthSnapshot::terminated(
            session_id,
            SessionHealthReason::SessionNotFound,
            error.message.clone(),
        )),
        BackendErrorKind::Transport => Some(SessionHealthSnapshot::stale(
            session_id,
            SessionHealthReason::BackendTransportLost,
            error.message.clone(),
        )),
        BackendErrorKind::Internal => Some(SessionHealthSnapshot::stale(
            session_id,
            SessionHealthReason::BackendInternalFault,
            error.message.clone(),
        )),
        BackendErrorKind::InvalidInput => None,
    }
}

pub(super) fn session_route_fingerprint(route: &SessionRoute) -> String {
    let external = route
        .external
        .as_ref()
        .map(|external| format!("{}/{}", external.namespace, external.value))
        .unwrap_or_else(|| "-".to_string());

    format!("v1/{:?}/{:?}/{external}", route.backend, route.authority)
}

pub(super) fn collect_pane_ids_from_topology(topology: &TopologySnapshot) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    for tab in &topology.tabs {
        pane_ids.extend(collect_pane_ids_from_node(&tab.root));
    }
    pane_ids
}

pub(super) fn collect_pane_ids_from_node(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_from_node_inner(root, &mut pane_ids);
    pane_ids
}

pub(super) fn saved_session_title(
    descriptor_title: Option<String>,
    topology: &TopologySnapshot,
) -> Option<String> {
    topology
        .focused_tab
        .and_then(|focused_tab| {
            topology
                .tabs
                .iter()
                .find(|tab| tab.tab_id == focused_tab)
                .and_then(|tab| tab.title.clone())
        })
        .or_else(|| topology.tabs.iter().find_map(|tab| tab.title.clone()))
        .or(descriptor_title)
}

pub(super) fn command_updates_summary_title(command: &MuxCommand) -> bool {
    matches!(
        command,
        MuxCommand::NewTab(_)
            | MuxCommand::CloseTab { .. }
            | MuxCommand::FocusTab { .. }
            | MuxCommand::RenameTab { .. }
    )
}

pub(super) fn tab_snapshot_by_id(
    topology: &TopologySnapshot,
    tab_id: TabId,
) -> Result<TabSnapshot, BackendError> {
    topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id == tab_id)
        .cloned()
        .ok_or_else(|| BackendError::internal(format!("missing restored tab {tab_id:?}")))
}

fn collect_pane_ids_from_node_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_from_node_inner(&split.first, pane_ids);
            collect_pane_ids_from_node_inner(&split.second, pane_ids);
        }
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::{BackendError, MuxCommand, NewTabSpec};
    use terminal_domain::{BackendKind, RouteAuthority, SessionId, SessionRoute, TabId};
    use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
    use terminal_projection::{SessionHealthPhase, TopologySnapshot};

    use super::session_health_from_attach_error;
    use super::{command_updates_summary_title, saved_session_title, session_route_fingerprint};

    #[test]
    fn saved_session_title_prefers_focused_tab_title() {
        let tab_id = TabId::new();
        let topology = TopologySnapshot {
            session_id: SessionId::new(),
            backend_kind: BackendKind::Native,
            tabs: vec![TabSnapshot {
                tab_id,
                title: Some("logs".to_string()),
                root: PaneTreeNode::Leaf { pane_id: terminal_domain::PaneId::new() },
                focused_pane: None,
            }],
            focused_tab: Some(tab_id),
        };

        assert_eq!(
            saved_session_title(Some("fallback".to_string()), &topology),
            Some("logs".to_string())
        );
    }

    #[test]
    fn command_title_refresh_tracks_only_tab_mutations() {
        assert!(command_updates_summary_title(&MuxCommand::NewTab(NewTabSpec::default())));
        assert!(!command_updates_summary_title(&MuxCommand::SaveSession));
    }

    #[test]
    fn attach_error_maps_transport_failures_to_stale_health() {
        let session_id = SessionId::new();
        let health = session_health_from_attach_error(
            session_id,
            &BackendError::transport("connection dropped"),
        )
        .expect("transport error should map to health");

        assert_eq!(health.phase, SessionHealthPhase::Stale);
        assert!(health.invalidated);
    }

    #[test]
    fn session_route_fingerprint_distinguishes_foreign_routes() {
        let route_a = SessionRoute {
            backend: BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "tmux_session".to_string(),
                value: "alpha".to_string(),
            }),
        };
        let route_b = SessionRoute {
            backend: BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "tmux_session".to_string(),
                value: "beta".to_string(),
            }),
        };

        assert_ne!(session_route_fingerprint(&route_a), session_route_fingerprint(&route_b));
    }
}
