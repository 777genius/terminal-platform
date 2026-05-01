use std::collections::HashSet;

use terminal_backend_api::{
    BackendRawOutputEvent, BackendRawOutputSubscription, BackendSessionPort, BackendSubscription,
    BackendSubscriptionEvent, SubscriptionSpec,
};
use terminal_domain::{PaneId, RouteAuthority, SessionRoute};
use terminal_persistence::{
    BackendCapabilityReportInput, HistoryGapEventInput, ScreenSnapshotEventInput,
    SqliteSessionStore, TerminalOutputEventInput, TopologySnapshotEventInput,
};
use terminal_projection::{
    ScreenDelta, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
};
use tokio::{sync::oneshot, time::MissedTickBehavior};

use crate::registry::SessionDescriptor;

use super::{
    V2_RAW_CAPTURE_FLUSH_INTERVAL, V2_RAW_CAPTURE_MAX_BATCH_BYTES,
    V2_RENDERED_CAPTURE_FLUSH_INTERVAL,
    helpers::{collect_pane_ids_from_topology, tab_id_for_pane},
};

pub(super) async fn run_v2_history_capture(
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
            Ok(subscription) => {
                persist_backend_capability_report(
                    persistence,
                    descriptor,
                    "raw_stream",
                    "raw_vt_stream",
                    "medium",
                    "native raw output subscription opened",
                );
                spawn_v2_raw_capture_loop(
                    persistence.clone(),
                    descriptor.clone(),
                    tab_id.clone(),
                    rows,
                    cols,
                    subscription,
                );
            }
            Err(_) => {
                if let Ok(subscription) =
                    session.subscribe(SubscriptionSpec::PaneSurface { pane_id }).await
                {
                    persist_backend_capability_report(
                        persistence,
                        descriptor,
                        "rendered_stream",
                        "rendered_plaintext_snapshot",
                        "low",
                        "rendered pane surface subscription opened after raw output fallback",
                    );
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

fn persist_backend_capability_report(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    capture_strategy: &str,
    capture_semantics: &str,
    command_boundary_confidence: &str,
    evidence_reason: &str,
) {
    let input = BackendCapabilityReportInput {
        id: None,
        session_id: Some(descriptor.session_id.0.to_string()),
        backend_kind: format!("{:?}", descriptor.route.backend).to_lowercase(),
        backend_version: None,
        backend_binary_path_hash: None,
        route_kind: persistence_route_kind(&descriptor.route),
        probe_status: "passed".to_string(),
        capture_strategy: capture_strategy.to_string(),
        capture_semantics: capture_semantics.to_string(),
        can_preserve_process_when_live: false,
        can_capture_scrollback: false,
        command_boundary_confidence: command_boundary_confidence.to_string(),
        evidence: Some(serde_json::json!({
            "source": "runtime_v2_history_capture",
            "reason": evidence_reason,
            "session_id": descriptor.session_id.0.to_string(),
            "backend": format!("{:?}", descriptor.route.backend).to_lowercase(),
        })),
        expires_at_ms: None,
    };
    let store = persistence.clone();
    let _ = tokio::task::spawn_blocking(move || store.record_v2_backend_capability_report(input));
}

fn persistence_route_kind(route: &SessionRoute) -> String {
    match route.authority {
        RouteAuthority::LocalDaemon => "local_daemon".to_string(),
        RouteAuthority::ImportedForeign => "imported_foreign".to_string(),
    }
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
