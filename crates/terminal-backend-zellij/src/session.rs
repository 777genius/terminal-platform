use std::sync::{Arc, Mutex as StdMutex};

use terminal_backend_api::{
    BackendError, BackendSessionPort, BackendSubscription, BoxFuture, MuxCommand, MuxCommandResult,
    SubscriptionSpec,
};
use terminal_domain::{PaneId, SessionId};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};
use tokio::sync::Mutex;

use crate::{backend::ZellijBackend, target::ZellijTarget};

mod commands;
mod snapshots;
mod subscriptions;

#[cfg(test)]
pub(crate) use snapshots::dump_screen_scrollback_args;

#[derive(Clone)]
pub(crate) struct ZellijAttachedSession {
    pub(crate) backend: Arc<ZellijBackend>,
    pub(crate) session_id: SessionId,
    pub(crate) target: ZellijTarget,
    pub(crate) io_lane: Arc<StdMutex<()>>,
    pub(crate) command_lane: Arc<Mutex<()>>,
}

impl BackendSessionPort for ZellijAttachedSession {
    fn topology_snapshot(&self) -> BoxFuture<'_, Result<TopologySnapshot, BackendError>> {
        Box::pin(async move { Ok(self.snapshot()?.topology) })
    }

    fn screen_snapshot(
        &self,
        pane_id: PaneId,
    ) -> BoxFuture<'_, Result<ScreenSnapshot, BackendError>> {
        Box::pin(async move { self.screen_snapshot_inner(pane_id) })
    }

    fn screen_delta(
        &self,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> BoxFuture<'_, Result<ScreenDelta, BackendError>> {
        Box::pin(async move {
            let current = self.screen_snapshot_inner(pane_id)?;
            if current.sequence == from_sequence {
                Ok(ScreenDelta::unchanged_from(&current))
            } else {
                Ok(ScreenDelta::full_replace(from_sequence, &current))
            }
        })
    }

    fn dispatch(
        &self,
        command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        let session = self.clone();
        Box::pin(async move { session.dispatch_inner(command).await })
    }

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let session = self.clone();
        Box::pin(async move { session.open_subscription(spec) })
    }
}
