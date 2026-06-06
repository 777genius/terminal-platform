use terminal_backend_api::{BackendError, BackendSubscription, BackendSubscriptionEvent};
use terminal_domain::PaneId;
use terminal_projection::{ProjectionSource, ScreenDelta, ScreenSnapshot};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::ChildStdout,
    sync::{mpsc, oneshot},
};

use crate::rows::ZellijSubscribeEvent;

use super::super::ZellijAttachedSession;

pub(super) fn open_pane_surface_subscription(
    session: &ZellijAttachedSession,
    pane_id: PaneId,
) -> Result<BackendSubscription, BackendError> {
    let subscription_id = terminal_domain::SubscriptionId::new();
    let session = session.clone();
    let pane_target = session.pane_target(pane_id)?;
    let initial = session.screen_snapshot_inner(pane_id)?;
    let (events_tx, events_rx) = mpsc::channel(32);
    let (cancel_tx, cancel_rx) = oneshot::channel();

    tokio::spawn(stream_pane_surface(
        session,
        pane_id,
        pane_target.backend_ref,
        initial,
        events_tx,
        cancel_rx,
    ));

    Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
}

async fn stream_pane_surface(
    session: ZellijAttachedSession,
    pane_id: PaneId,
    backend_ref: String,
    initial: ScreenSnapshot,
    events_tx: mpsc::Sender<BackendSubscriptionEvent>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    if send_screen_delta(&events_tx, ScreenDelta::full_replace(0, &initial)).await.is_err() {
        return;
    }

    let mut child = match session.backend.spawn_subscribe(&session.target, &backend_ref) {
        Ok(child) => child,
        Err(_) => return,
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.start_kill();
        let _ = child.wait().await;
        return;
    };

    let mut lines = BufReader::new(stdout).lines();
    let mut last = initial;
    stream_subscribe_lines(
        &session,
        pane_id,
        &backend_ref,
        &mut last,
        &events_tx,
        &mut cancel_rx,
        &mut lines,
    )
    .await;

    let _ = child.start_kill();
    let _ = child.wait().await;
}

async fn stream_subscribe_lines(
    session: &ZellijAttachedSession,
    pane_id: PaneId,
    backend_ref: &str,
    last: &mut ScreenSnapshot,
    events_tx: &mpsc::Sender<BackendSubscriptionEvent>,
    cancel_rx: &mut oneshot::Receiver<()>,
    lines: &mut Lines<BufReader<ChildStdout>>,
) {
    loop {
        tokio::select! {
            _ = &mut *cancel_rx => break,
            next_line = lines.next_line() => {
                let should_continue = match next_line {
                    Ok(Some(line)) => handle_subscribe_line(
                        session,
                        pane_id,
                        backend_ref,
                        last,
                        events_tx,
                        line,
                    )
                    .await,
                    Ok(None) | Err(_) => false,
                };
                if !should_continue {
                    break;
                }
            }
        }
    }
}

async fn handle_subscribe_line(
    session: &ZellijAttachedSession,
    pane_id: PaneId,
    backend_ref: &str,
    last: &mut ScreenSnapshot,
    events_tx: &mpsc::Sender<BackendSubscriptionEvent>,
    line: String,
) -> bool {
    if line.trim().is_empty() {
        return true;
    }

    let event = match serde_json::from_str::<ZellijSubscribeEvent>(&line) {
        Ok(event) => event,
        Err(_) => return false,
    };

    match event {
        ZellijSubscribeEvent::PaneUpdate {
            pane_id: updated_pane_ref,
            viewport,
            is_initial,
            ..
        } => {
            if updated_pane_ref != backend_ref || is_initial {
                return true;
            }

            let Ok(mut current) = session.screen_snapshot_from_viewport(
                pane_id,
                viewport,
                ProjectionSource::ZellijViewportSubscribe,
            ) else {
                return false;
            };
            if equivalent_surface(last, &current) {
                return true;
            }
            if current.sequence <= last.sequence {
                current.sequence = last.sequence.saturating_add(1);
            }

            let delta = ScreenDelta::between(last, &current);
            *last = current;
            send_screen_delta(events_tx, delta).await.is_ok()
        }
        ZellijSubscribeEvent::PaneClosed { pane_id: closed_pane_ref } => {
            closed_pane_ref != backend_ref
        }
    }
}

fn equivalent_surface(previous: &ScreenSnapshot, current: &ScreenSnapshot) -> bool {
    previous.pane_id == current.pane_id
        && previous.rows == current.rows
        && previous.cols == current.cols
        && previous.surface == current.surface
}

async fn send_screen_delta(
    events_tx: &mpsc::Sender<BackendSubscriptionEvent>,
    delta: ScreenDelta,
) -> Result<(), mpsc::error::SendError<BackendSubscriptionEvent>> {
    events_tx.send(BackendSubscriptionEvent::ScreenDelta(delta)).await
}
