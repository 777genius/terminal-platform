use terminal_backend_api::{BackendRawOutputEvent, BackendRawOutputSubscription};
use terminal_domain::PaneId;
use terminal_persistence::{SqliteSessionStore, TerminalOutputEventInput};
use tokio::time::MissedTickBehavior;

use crate::registry::SessionDescriptor;

use super::super::{V2_RAW_CAPTURE_FLUSH_INTERVAL, V2_RAW_CAPTURE_MAX_BATCH_BYTES};
use super::events::persist_history_gap;

pub(super) fn spawn_v2_raw_capture_loop(
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
