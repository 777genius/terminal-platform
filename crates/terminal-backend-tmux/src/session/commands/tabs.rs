use super::super::TmuxAttachedSession;
use crate::prelude::*;

impl TmuxAttachedSession {
    pub(super) fn rename_tab(
        &self,
        tab_id: TabId,
        title: &str,
    ) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend
            .run(Some(&self.target), &["rename-window", "-t", &tab_target.target, title])?;

        Ok(MuxCommandResult { changed: true })
    }

    pub(super) fn focus_tab(&self, tab_id: TabId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend.run(Some(&self.target), &["select-window", "-t", &tab_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    pub(super) fn new_tab(
        &self,
        spec: terminal_backend_api::NewTabSpec,
    ) -> Result<MuxCommandResult, BackendError> {
        let mut args = vec![
            "new-window".to_string(),
            "-P".to_string(),
            "-F".to_string(),
            "#{window_id}".to_string(),
        ];
        args.push("-t".to_string());
        args.push(self.target.session_name.clone());
        if let Some(title) = spec.title {
            args.push("-n".to_string());
            args.push(title);
        }
        self.backend.run_owned(Some(&self.target), &args)?;

        Ok(MuxCommandResult { changed: true })
    }

    pub(super) fn close_tab(&self, tab_id: TabId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        if snapshot.topology.tabs.len() <= 1 {
            return Err(BackendError::unsupported(
                "tmux imported routes refuse to close the last tab because it would terminate the foreign session",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend.run(Some(&self.target), &["kill-window", "-t", &tab_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }
}
