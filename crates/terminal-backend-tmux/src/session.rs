mod commands;
mod screen;
mod snapshot;
mod subscriptions;

use crate::{backend::TmuxBackend, prelude::*, target::TmuxTarget};

pub(super) struct TmuxSessionSnapshot {
    pub(super) topology: TopologySnapshot,
    pub(super) pane_targets: HashMap<PaneId, crate::rows::TmuxPaneTarget>,
    pub(super) tab_targets: HashMap<TabId, crate::rows::TmuxTabTarget>,
}

#[derive(Clone)]
pub(crate) struct TmuxAttachedSession {
    pub(crate) backend: Arc<TmuxBackend>,
    pub(crate) session_id: SessionId,
    pub(crate) target: TmuxTarget,
}

impl BackendSessionPort for TmuxAttachedSession {
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
        Box::pin(async move { self.dispatch_inner(command) })
    }

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let session = self.clone();
        Box::pin(async move { session.open_subscription(spec) })
    }
}
