use std::time::Instant;

use terminal_backend_api::BackendError;
use tokio::time;

use crate::{
    action::ZellijAction,
    cli::is_transient_zellij_backend_error,
    constants::{
        ZELLIJ_ACTION_SETTLE_ATTEMPTS, ZELLIJ_ACTION_SETTLE_TIMEOUT, ZELLIJ_POLL_INTERVAL,
    },
    snapshot::ZellijSessionSnapshot,
};

use super::super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(super) async fn wait_for_action_settle(
        &self,
        previous: &ZellijSessionSnapshot,
        action: &ZellijAction,
    ) -> Result<ZellijSessionSnapshot, BackendError> {
        let mut last_error = None;
        let mut last_snapshot_summary = None;
        let started = Instant::now();
        for _ in 0..ZELLIJ_ACTION_SETTLE_ATTEMPTS {
            match self.snapshot() {
                Ok(snapshot) if action.settled(previous, &snapshot) => return Ok(snapshot),
                Ok(snapshot) => {
                    last_snapshot_summary = Some(snapshot.settle_summary());
                }
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            }
            if started.elapsed() >= ZELLIJ_ACTION_SETTLE_TIMEOUT {
                break;
            }
            time::sleep(ZELLIJ_POLL_INTERVAL).await;
        }

        Err(last_error.unwrap_or_else(|| {
            BackendError::transport(format!(
                "zellij action did not settle within {} ms: action={action:?}; last_snapshot={}",
                ZELLIJ_ACTION_SETTLE_TIMEOUT.as_millis(),
                last_snapshot_summary.unwrap_or_else(|| "<none>".to_string())
            ))
        }))
    }
}
