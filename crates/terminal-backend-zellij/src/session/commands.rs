mod input;
mod panes;
mod settle;
mod tabs;

use terminal_backend_api::{BackendError, MuxCommand, MuxCommandResult};
use terminal_domain::DegradedModeReason;

use crate::{
    action::ZellijAction,
    cli::{zellij_focus_actions_supported, zellij_focus_unsupported_error},
    snapshot::ZellijSessionSnapshot,
};

use super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(super) async fn dispatch_inner(
        &self,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let actions = self.dispatch_actions(&snapshot, command)?;
        if actions.is_empty() {
            return Ok(MuxCommandResult { changed: false });
        }

        let _permit = self.command_lane.lock().await;
        let mut settled_snapshot = snapshot.clone();
        for action in actions {
            let _io_permit = self.io_lane.lock().expect("zellij io lane should not be poisoned");
            self.backend.run_owned(Some(&self.target), &action.args())?;
            drop(_io_permit);
            if action.requires_settle() {
                settled_snapshot = self.wait_for_action_settle(&settled_snapshot, &action).await?;
            }
        }

        Ok(MuxCommandResult { changed: true })
    }

    pub(crate) fn dispatch_actions(
        &self,
        snapshot: &ZellijSessionSnapshot,
        command: MuxCommand,
    ) -> Result<Vec<ZellijAction>, BackendError> {
        match command {
            MuxCommand::NewTab(spec) => Ok(self.new_tab_actions(spec)),
            MuxCommand::SendInput(spec) => self.send_input_actions(snapshot, spec),
            MuxCommand::SendPaste(spec) => self.send_paste_actions(snapshot, spec),
            MuxCommand::FocusPane { .. } if !zellij_focus_actions_supported() => {
                Err(zellij_focus_unsupported_error())
            }
            MuxCommand::FocusPane { pane_id } => {
                Ok(vec![self.focus_pane_action(snapshot, pane_id)?])
            }
            MuxCommand::ClosePane { pane_id } => {
                Ok(vec![self.close_pane_action(snapshot, pane_id)?])
            }
            MuxCommand::FocusTab { .. } if !zellij_focus_actions_supported() => {
                Err(zellij_focus_unsupported_error())
            }
            MuxCommand::FocusTab { tab_id } => Ok(vec![self.focus_tab_action(snapshot, tab_id)?]),
            MuxCommand::CloseTab { tab_id } => Ok(vec![self.close_tab_action(snapshot, tab_id)?]),
            MuxCommand::RenameTab { tab_id, title } => {
                Ok(vec![self.rename_tab_action(snapshot, tab_id, &title)?])
            }
            MuxCommand::SplitPane(_)
            | MuxCommand::ResizePane(_)
            | MuxCommand::Detach
            | MuxCommand::SaveSession
            | MuxCommand::OverrideLayout(_) => Err(BackendError::unsupported(
                "zellij imported routes do not support this command in the current rollout phase",
                DegradedModeReason::UnsupportedByBackend,
            )),
        }
    }
}
