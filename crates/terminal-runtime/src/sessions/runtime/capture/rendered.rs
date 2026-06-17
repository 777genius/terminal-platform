use terminal_backend_api::{BackendSubscription, BackendSubscriptionEvent};
use terminal_persistence::SqliteSessionStore;
use terminal_projection::{ScreenDelta, ScreenLine, ScreenSnapshot, ScreenSurface};
use tokio::time::MissedTickBehavior;

use crate::registry::SessionDescriptor;

use super::super::V2_RENDERED_CAPTURE_FLUSH_INTERVAL;
use super::events::{CapturePersistenceDiagnostics, persist_screen_snapshot};

pub(super) fn spawn_v2_rendered_capture_loop(
    persistence: SqliteSessionStore,
    diagnostics: CapturePersistenceDiagnostics,
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
                            if let Some(snapshot) = snapshot_from_delta(&mut current, *delta) {
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
                            &diagnostics,
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
            persist_screen_snapshot(&persistence, &diagnostics, &descriptor, tab_id, snapshot)
                .await;
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
            buffer_kind: delta.buffer_kind,
            surface,
        };
        *current = Some(snapshot.clone());
        return Some(snapshot);
    }

    let patch = delta.patch?;
    let mut surface =
        current.as_ref().map(|snapshot| snapshot.surface.clone()).unwrap_or_else(|| {
            ScreenSurface {
                title: None,
                working_directory_uri: None,
                user_variables: Default::default(),
                cursor: None,
                palette: Default::default(),
                bell_count: 0,
                progress: Default::default(),
                lines: Vec::new(),
            }
        });
    if patch.title_changed {
        surface.title = patch.title;
    }
    if patch.cursor_changed {
        surface.cursor = patch.cursor;
    }
    let target_rows = usize::from(delta.rows);
    if surface.lines.len() < target_rows {
        surface.lines.resize(target_rows, ScreenLine::plain(String::new()));
    }
    for line in patch.line_updates {
        let row = usize::from(line.row);
        if row >= surface.lines.len() {
            surface.lines.resize(row + 1, ScreenLine::plain(String::new()));
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
        buffer_kind: delta.buffer_kind,
        surface,
    };
    *current = Some(snapshot.clone());
    Some(snapshot)
}
