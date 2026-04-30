use std::collections::HashSet;

use terminal_backend_api::{
    BackendError, BackendErrorKind, BackendRawOutputEvent, BackendRawOutputSubscription,
    BackendSessionPort, BackendSessionSummary, BackendSubscription, BackendSubscriptionEvent,
    CreateSessionSpec, MuxBackendPort, MuxCommand, SubscriptionSpec,
};
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute, TabId};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
use terminal_persistence::{
    HistoryGapEventInput, ScreenSnapshotEventInput, SessionRouteRecord, SqliteSessionStore,
    TerminalOutputEventInput, TopologySnapshotEventInput,
};
use terminal_projection::{
    ScreenDelta, ScreenLine, ScreenSnapshot, ScreenSurface, SessionHealthReason,
    SessionHealthSnapshot, TopologySnapshot,
};
use tokio::{
    sync::oneshot,
    time::{Duration, MissedTickBehavior},
};

use crate::{
    backend_catalog::BackendCatalog,
    registry::{SessionDescriptor, SessionRegistry},
};

const V2_RAW_CAPTURE_FLUSH_INTERVAL: Duration = Duration::from_millis(100);
const V2_RAW_CAPTURE_MAX_BATCH_BYTES: usize = 64 * 1024;
const V2_RENDERED_CAPTURE_FLUSH_INTERVAL: Duration = Duration::from_millis(250);
const V2_CAPTURE_READY_TIMEOUT: Duration = Duration::from_secs(2);

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

async fn run_v2_history_capture(
    persistence: SqliteSessionStore,
    descriptor: SessionDescriptor,
    session: Box<dyn BackendSessionPort>,
    ready_tx: oneshot::Sender<()>,
) {
    let mut ready_tx = Some(ready_tx);
    let mut captured_panes = HashSet::new();
    let Ok(initial_topology) = session.topology_snapshot().await else {
        if let Some(ready_tx) = ready_tx.take() {
            let _ = ready_tx.send(());
        }
        return;
    };

    persist_topology_snapshot(&persistence, &descriptor, initial_topology.clone()).await;
    start_capture_for_topology(
        &persistence,
        &descriptor,
        &*session,
        &initial_topology,
        &mut captured_panes,
    )
    .await;
    if let Some(ready_tx) = ready_tx.take() {
        let _ = ready_tx.send(());
    }

    let Ok(mut topology_subscription) = session.subscribe(SubscriptionSpec::SessionTopology).await
    else {
        return;
    };

    while let Some(event) = topology_subscription.events.recv().await {
        match event {
            BackendSubscriptionEvent::TopologySnapshot(topology) => {
                persist_topology_snapshot(&persistence, &descriptor, topology.clone()).await;
                start_capture_for_topology(
                    &persistence,
                    &descriptor,
                    &*session,
                    &topology,
                    &mut captured_panes,
                )
                .await;
            }
            BackendSubscriptionEvent::SessionHealthSnapshot(health) if health.invalidated => break,
            BackendSubscriptionEvent::ScreenDelta(_)
            | BackendSubscriptionEvent::SessionHealthSnapshot(_) => {}
        }
    }

    topology_subscription.cancel();
}

async fn start_capture_for_topology(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    session: &dyn BackendSessionPort,
    topology: &TopologySnapshot,
    captured_panes: &mut HashSet<PaneId>,
) {
    for pane_id in collect_pane_ids_from_topology(topology) {
        if !captured_panes.insert(pane_id) {
            continue;
        }

        let tab_id = tab_id_for_pane(topology, pane_id).map(|tab_id| tab_id.0.to_string());
        let initial_screen = session.screen_snapshot(pane_id).await.ok();
        let rows = initial_screen.as_ref().map(|screen| i32::from(screen.rows));
        let cols = initial_screen.as_ref().map(|screen| i32::from(screen.cols));

        match session.subscribe_raw_output(pane_id).await {
            Ok(subscription) => spawn_v2_raw_capture_loop(
                persistence.clone(),
                descriptor.clone(),
                tab_id.clone(),
                rows,
                cols,
                subscription,
            ),
            Err(_) => {
                if let Ok(subscription) =
                    session.subscribe(SubscriptionSpec::PaneSurface { pane_id }).await
                {
                    spawn_v2_rendered_capture_loop(
                        persistence.clone(),
                        descriptor.clone(),
                        tab_id.clone(),
                        subscription,
                    );
                }
            }
        }

        if let Some(screen) = initial_screen {
            persist_screen_snapshot(persistence, descriptor, tab_id, screen).await;
        }
    }
}

fn spawn_v2_raw_capture_loop(
    persistence: SqliteSessionStore,
    descriptor: SessionDescriptor,
    tab_id: Option<String>,
    rows: Option<i32>,
    cols: Option<i32>,
    mut subscription: BackendRawOutputSubscription,
) {
    tokio::spawn(async move {
        let mut flush_interval = tokio::time::interval(V2_RAW_CAPTURE_FLUSH_INTERVAL);
        flush_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut pending_payload = Vec::new();
        let mut pending_pane_id = None;
        let mut pending_sequence = None;

        loop {
            tokio::select! {
                event = subscription.events.recv() => {
                    match event {
                        Some(BackendRawOutputEvent::Bytes(bytes)) => {
                            pending_pane_id = Some(bytes.pane_id);
                            pending_sequence = Some(bytes.sequence);
                            pending_payload.extend(bytes.payload);
                            if pending_payload.len() >= V2_RAW_CAPTURE_MAX_BATCH_BYTES {
                                flush_raw_capture_batch(
                                    &persistence,
                                    &descriptor,
                                    tab_id.clone(),
                                    pending_pane_id,
                                    rows,
                                    cols,
                                    &mut pending_payload,
                                    pending_sequence,
                                )
                                .await;
                            }
                        }
                        Some(BackendRawOutputEvent::Gap(gap)) => {
                            flush_raw_capture_batch(
                                &persistence,
                                &descriptor,
                                tab_id.clone(),
                                pending_pane_id,
                                rows,
                                cols,
                                &mut pending_payload,
                                pending_sequence,
                            )
                            .await;
                            pending_pane_id = None;
                            pending_sequence = None;
                            persist_history_gap(
                                &persistence,
                                &descriptor,
                                tab_id.clone(),
                                rows,
                                cols,
                                gap.pane_id,
                                gap.skipped_events,
                            )
                            .await;
                        }
                        None => break,
                    }
                }
                _ = flush_interval.tick() => {
                    flush_raw_capture_batch(
                        &persistence,
                        &descriptor,
                        tab_id.clone(),
                        pending_pane_id,
                        rows,
                        cols,
                        &mut pending_payload,
                        pending_sequence,
                    )
                    .await;
                }
            }
        }

        flush_raw_capture_batch(
            &persistence,
            &descriptor,
            tab_id,
            pending_pane_id,
            rows,
            cols,
            &mut pending_payload,
            pending_sequence,
        )
        .await;
        subscription.cancel();
    });
}

async fn flush_raw_capture_batch(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    tab_id: Option<String>,
    pane_id: Option<PaneId>,
    rows: Option<i32>,
    cols: Option<i32>,
    pending_payload: &mut Vec<u8>,
    source_sequence: Option<u64>,
) {
    if pending_payload.is_empty() {
        return;
    }
    let Some(pane_id) = pane_id else {
        pending_payload.clear();
        return;
    };

    let input = TerminalOutputEventInput {
        session_id: descriptor.session_id.0.to_string(),
        route: descriptor.route.clone(),
        title: descriptor.title.clone(),
        launch: descriptor.launch.clone(),
        pane_id: pane_id.0.to_string(),
        tab_id,
        payload: std::mem::take(pending_payload),
        rows,
        cols,
        source_sequence,
        occurred_at_ms: None,
        capture_semantics: Some("raw_vt_stream".to_string()),
    };
    let store = persistence.clone();
    let _ = tokio::task::spawn_blocking(move || store.record_v2_terminal_output(input)).await;
}

async fn persist_history_gap(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    tab_id: Option<String>,
    rows: Option<i32>,
    cols: Option<i32>,
    pane_id: PaneId,
    skipped_events: u64,
) {
    let input = HistoryGapEventInput {
        session_id: descriptor.session_id.0.to_string(),
        route: descriptor.route.clone(),
        title: descriptor.title.clone(),
        launch: descriptor.launch.clone(),
        pane_id: pane_id.0.to_string(),
        tab_id,
        rows,
        cols,
        skipped_events,
        estimated_dropped_bytes: None,
        reason: "raw_output_receiver_lagged".to_string(),
        occurred_at_ms: None,
    };
    let store = persistence.clone();
    let _ = tokio::task::spawn_blocking(move || store.record_v2_history_gap(input)).await;
}

fn spawn_v2_rendered_capture_loop(
    persistence: SqliteSessionStore,
    descriptor: SessionDescriptor,
    tab_id: Option<String>,
    mut subscription: BackendSubscription,
) {
    tokio::spawn(async move {
        let mut current = None;
        let mut pending_snapshot = None;
        let mut flush_interval = tokio::time::interval(V2_RENDERED_CAPTURE_FLUSH_INTERVAL);
        flush_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                event = subscription.events.recv() => {
                    match event {
                        Some(BackendSubscriptionEvent::ScreenDelta(delta)) => {
                            if let Some(snapshot) = snapshot_from_delta(&mut current, delta) {
                                pending_snapshot = Some(snapshot);
                            }
                        }
                        Some(BackendSubscriptionEvent::SessionHealthSnapshot(health)) if health.invalidated => {
                            break;
                        }
                        Some(
                            BackendSubscriptionEvent::TopologySnapshot(_)
                            | BackendSubscriptionEvent::SessionHealthSnapshot(_)
                        ) => {}
                        None => break,
                    }
                }
                _ = flush_interval.tick() => {
                    if let Some(snapshot) = pending_snapshot.take() {
                        persist_screen_snapshot(
                            &persistence,
                            &descriptor,
                            tab_id.clone(),
                            snapshot,
                        )
                        .await;
                    }
                }
            }
        }

        if let Some(snapshot) = pending_snapshot {
            persist_screen_snapshot(&persistence, &descriptor, tab_id, snapshot).await;
        }
        subscription.cancel();
    });
}

fn snapshot_from_delta(
    current: &mut Option<ScreenSnapshot>,
    delta: ScreenDelta,
) -> Option<ScreenSnapshot> {
    if let Some(surface) = delta.full_replace {
        let snapshot = ScreenSnapshot {
            pane_id: delta.pane_id,
            sequence: delta.to_sequence,
            rows: delta.rows,
            cols: delta.cols,
            source: delta.source,
            surface,
        };
        *current = Some(snapshot.clone());
        return Some(snapshot);
    }

    let patch = delta.patch?;
    let mut surface = current
        .as_ref()
        .map(|snapshot| snapshot.surface.clone())
        .unwrap_or_else(|| ScreenSurface { title: None, cursor: None, lines: Vec::new() });
    if patch.title_changed {
        surface.title = patch.title;
    }
    if patch.cursor_changed {
        surface.cursor = patch.cursor;
    }
    let target_rows = usize::from(delta.rows);
    if surface.lines.len() < target_rows {
        surface.lines.resize(target_rows, ScreenLine { text: String::new() });
    }
    for line in patch.line_updates {
        let row = usize::from(line.row);
        if row >= surface.lines.len() {
            surface.lines.resize(row + 1, ScreenLine { text: String::new() });
        }
        surface.lines[row] = line.line;
    }
    if surface.lines.len() > target_rows {
        surface.lines.truncate(target_rows);
    }

    let snapshot = ScreenSnapshot {
        pane_id: delta.pane_id,
        sequence: delta.to_sequence,
        rows: delta.rows,
        cols: delta.cols,
        source: delta.source,
        surface,
    };
    *current = Some(snapshot.clone());
    Some(snapshot)
}

async fn persist_topology_snapshot(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    topology: TopologySnapshot,
) {
    let input = TopologySnapshotEventInput {
        session_id: descriptor.session_id.0.to_string(),
        route: descriptor.route.clone(),
        title: descriptor.title.clone(),
        launch: descriptor.launch.clone(),
        topology,
    };
    let store = persistence.clone();
    let _ = tokio::task::spawn_blocking(move || store.record_v2_topology_snapshot(input)).await;
}

async fn persist_screen_snapshot(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    tab_id: Option<String>,
    screen: ScreenSnapshot,
) {
    let input = ScreenSnapshotEventInput {
        session_id: descriptor.session_id.0.to_string(),
        route: descriptor.route.clone(),
        title: descriptor.title.clone(),
        launch: descriptor.launch.clone(),
        tab_id,
        screen,
        buffer_kind: Some("normal".to_string()),
        capture_semantics: Some("rendered_plaintext_snapshot".to_string()),
    };
    let store = persistence.clone();
    let _ = tokio::task::spawn_blocking(move || store.record_v2_screen_snapshot(input)).await;
}

fn tab_id_for_pane(topology: &TopologySnapshot, pane_id: PaneId) -> Option<TabId> {
    topology
        .tabs
        .iter()
        .find(|tab| collect_pane_ids_from_node(&tab.root).contains(&pane_id))
        .map(|tab| tab.tab_id)
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
