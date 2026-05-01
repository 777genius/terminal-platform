use std::sync::Arc;

use terminal_backend_api::{
    BackendError, BackendRawOutputSubscription, BackendSessionPort, BackendSubscription, BoxFuture,
    MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{DegradedModeReason, PaneId};

use crate::{
    engine::NativeSessionEngine,
    subscriptions::{open_native_raw_output_subscription, open_native_subscription},
};

pub(super) struct NativeAttachedSession {
    runtime: Arc<NativeSessionEngine>,
}

impl NativeAttachedSession {
    pub(super) fn new(runtime: Arc<NativeSessionEngine>) -> Self {
        Self { runtime }
    }
}

impl BackendSessionPort for NativeAttachedSession {
    fn topology_snapshot(
        &self,
    ) -> BoxFuture<'_, Result<terminal_projection::TopologySnapshot, BackendError>> {
        Box::pin(async move { self.runtime.topology_snapshot() })
    }

    fn screen_snapshot(
        &self,
        pane_id: PaneId,
    ) -> BoxFuture<'_, Result<terminal_projection::ScreenSnapshot, BackendError>> {
        Box::pin(async move { self.runtime.screen_snapshot(pane_id) })
    }

    fn screen_delta(
        &self,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> BoxFuture<'_, Result<terminal_projection::ScreenDelta, BackendError>> {
        Box::pin(async move { self.runtime.screen_delta(pane_id, from_sequence) })
    }

    fn dispatch(
        &self,
        command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        Box::pin(async move {
            let changed = match command {
                MuxCommand::NewTab(spec) => self.runtime.new_tab(spec)?,
                MuxCommand::SplitPane(spec) => self.runtime.split_pane(spec)?,
                MuxCommand::FocusTab { tab_id } => self.runtime.focus_tab(tab_id)?,
                MuxCommand::RenameTab { tab_id, title } => {
                    self.runtime.rename_tab(tab_id, title)?
                }
                MuxCommand::FocusPane { pane_id } => self.runtime.focus_pane(pane_id)?,
                MuxCommand::ClosePane { pane_id } => self.runtime.close_pane(pane_id)?,
                MuxCommand::CloseTab { tab_id } => self.runtime.close_tab(tab_id)?,
                MuxCommand::ResizePane(spec) => self.runtime.resize_pane(spec)?,
                MuxCommand::OverrideLayout(spec) => self.runtime.override_layout(spec)?,
                MuxCommand::SendInput(spec) => self.runtime.send_input(spec)?,
                MuxCommand::SendPaste(spec) => self.runtime.send_paste(spec)?,
                MuxCommand::Detach | MuxCommand::SaveSession => {
                    return Err(BackendError::unsupported(
                        "native mux command is not wired in v1 start phase",
                        DegradedModeReason::NotYetImplemented,
                    ));
                }
            };

            Ok(MuxCommandResult { changed })
        })
    }

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let runtime = Arc::clone(&self.runtime);
        Box::pin(async move { open_native_subscription(runtime, spec) })
    }

    fn subscribe_raw_output(
        &self,
        pane_id: PaneId,
    ) -> BoxFuture<'_, Result<BackendRawOutputSubscription, BackendError>> {
        let runtime = Arc::clone(&self.runtime);
        Box::pin(async move { open_native_raw_output_subscription(runtime, pane_id) })
    }
}
