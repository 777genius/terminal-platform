use std::{collections::HashSet, sync::Arc};

use terminal_backend_api::{BackendSessionPort, BackendSubscriptionEvent, SubscriptionSpec};
use terminal_domain::PaneId;
use terminal_persistence::SqliteSessionStore;
use terminal_projection::TopologySnapshot;
use tokio::sync::oneshot;

use crate::registry::{SessionDescriptor, SessionRegistry};

use super::helpers::{collect_pane_ids_from_topology, tab_id_for_pane};

mod events;
mod raw;
mod rendered;

use events::{
    CapturePersistenceDiagnostics, persist_backend_capability_report, persist_screen_snapshot,
    persist_topology_snapshot,
};
use raw::spawn_v2_raw_capture_loop;
use rendered::spawn_v2_rendered_capture_loop;

pub(super) async fn run_v2_history_capture(
    persistence: SqliteSessionStore,
    registry: Arc<dyn SessionRegistry>,
    descriptor: SessionDescriptor,
    session: Box<dyn BackendSessionPort>,
    ready_tx: oneshot::Sender<()>,
) {
    let diagnostics =
        CapturePersistenceDiagnostics::new(persistence.clone(), registry, descriptor.session_id);
    let mut ready_tx = Some(ready_tx);
    let mut captured_panes = HashSet::new();
    let Ok(initial_topology) = session.topology_snapshot().await else {
        if let Some(ready_tx) = ready_tx.take() {
            let _ = ready_tx.send(());
        }
        return;
    };

    persist_topology_snapshot(&persistence, &diagnostics, &descriptor, initial_topology.clone())
        .await;
    start_capture_for_topology(
        &persistence,
        &diagnostics,
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
                persist_topology_snapshot(
                    &persistence,
                    &diagnostics,
                    &descriptor,
                    topology.clone(),
                )
                .await;
                start_capture_for_topology(
                    &persistence,
                    &diagnostics,
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
    diagnostics: &CapturePersistenceDiagnostics,
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
                    diagnostics,
                    descriptor,
                    "raw_stream",
                    "raw_vt_stream",
                    "medium",
                    "native raw output subscription opened",
                );
                spawn_v2_raw_capture_loop(
                    persistence.clone(),
                    diagnostics.clone(),
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
                        diagnostics,
                        descriptor,
                        "rendered_stream",
                        "rendered_plaintext_snapshot",
                        "low",
                        "rendered pane surface subscription opened after raw output fallback",
                    );
                    spawn_v2_rendered_capture_loop(
                        persistence.clone(),
                        diagnostics.clone(),
                        descriptor.clone(),
                        tab_id.clone(),
                        subscription,
                    );
                }
            }
        }

        if let Some(screen) = initial_screen {
            persist_screen_snapshot(persistence, diagnostics, descriptor, tab_id, screen).await;
        }
    }
}
