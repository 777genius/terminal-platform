use super::super::TmuxAttachedSession;
use crate::prelude::*;

impl TmuxAttachedSession {
    pub(in crate::session) fn dispatch_inner(
        &self,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        match command {
            MuxCommand::NewTab(spec) => self.new_tab(spec),
            MuxCommand::SplitPane(spec) => self.split_pane(spec),
            MuxCommand::SendInput(spec) => self.send_input(spec),
            MuxCommand::SendPaste(spec) => self.send_paste(spec),
            MuxCommand::ClosePane { pane_id } => self.close_pane(pane_id),
            MuxCommand::CloseTab { tab_id } => self.close_tab(tab_id),
            MuxCommand::FocusTab { tab_id } => self.focus_tab(tab_id),
            MuxCommand::RenameTab { tab_id, title } => self.rename_tab(tab_id, &title),
            MuxCommand::FocusPane { pane_id } => self.focus_pane(pane_id),
            MuxCommand::ResizePane(spec) => self.resize_pane(spec),
            MuxCommand::Detach | MuxCommand::SaveSession | MuxCommand::OverrideLayout(_) => {
                Err(BackendError::unsupported(
                    "tmux imported routes do not support this command in the current rollout phase",
                    DegradedModeReason::UnsupportedByBackend,
                ))
            }
        }
    }
}
