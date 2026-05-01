use super::TmuxAttachedSession;
use crate::{prelude::*, util::*};

impl TmuxAttachedSession {
    pub(super) fn dispatch_inner(
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
    fn rename_tab(&self, tab_id: TabId, title: &str) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend
            .run(Some(&self.target), &["rename-window", "-t", &tab_target.target, title])?;

        Ok(MuxCommandResult { changed: true })
    }

    fn focus_tab(&self, tab_id: TabId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend.run(Some(&self.target), &["select-window", "-t", &tab_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    fn focus_pane(&self, pane_id: PaneId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        self.backend.run(Some(&self.target), &["select-pane", "-t", &pane_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    fn new_tab(
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

    fn close_tab(&self, tab_id: TabId) -> Result<MuxCommandResult, BackendError> {
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

    fn split_pane(&self, spec: SplitPaneSpec) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot.pane_targets.get(&spec.pane_id).ok_or_else(|| {
            BackendError::not_found(format!("unknown tmux pane {:?}", spec.pane_id))
        })?;
        self.backend.run(
            Some(&self.target),
            &["split-window", tmux_split_flag(spec.direction), "-t", &pane_target.target],
        )?;

        Ok(MuxCommandResult { changed: true })
    }

    fn close_pane(&self, pane_id: PaneId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        let tab =
            snapshot.topology.tabs.iter().find(|tab| tab_contains_pane(tab, pane_id)).ok_or_else(
                || BackendError::not_found(format!("tmux pane {pane_id:?} is not bound to a tab")),
            )?;
        if collect_pane_ids(&tab.root).len() <= 1 {
            return Err(BackendError::unsupported(
                "tmux imported routes refuse to close the last pane in a tab because it would collapse tab lifecycle into tab closure semantics",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        self.backend.run(Some(&self.target), &["kill-pane", "-t", &pane_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    fn resize_pane(&self, spec: ResizePaneSpec) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot.pane_targets.get(&spec.pane_id).ok_or_else(|| {
            BackendError::not_found(format!("unknown tmux pane {:?}", spec.pane_id))
        })?;
        if pane_target.rows == spec.rows && pane_target.cols == spec.cols {
            return Ok(MuxCommandResult { changed: false });
        }
        let rows = spec.rows.to_string();
        let cols = spec.cols.to_string();
        self.backend.run(
            Some(&self.target),
            &["resize-pane", "-t", &pane_target.target, "-y", &rows, "-x", &cols],
        )?;

        Ok(MuxCommandResult { changed: true })
    }

    fn send_input(&self, spec: SendInputSpec) -> Result<MuxCommandResult, BackendError> {
        self.send_text_to_pane(spec.pane_id, &spec.data)
    }

    fn send_paste(&self, spec: SendPasteSpec) -> Result<MuxCommandResult, BackendError> {
        self.send_text_to_pane(spec.pane_id, &spec.data)
    }

    fn send_text_to_pane(
        &self,
        pane_id: PaneId,
        data: &str,
    ) -> Result<MuxCommandResult, BackendError> {
        if data.is_empty() {
            return Ok(MuxCommandResult { changed: false });
        }

        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        self.send_tmux_text(&pane_target.target, data)?;

        Ok(MuxCommandResult { changed: true })
    }

    fn send_tmux_text(&self, pane_target: &str, data: &str) -> Result<(), BackendError> {
        let mut literal = String::new();
        for ch in data.chars() {
            match ch {
                '\r' | '\n' => {
                    self.flush_tmux_literal(pane_target, &mut literal)?;
                    self.backend
                        .run(Some(&self.target), &["send-keys", "-t", pane_target, "Enter"])?;
                }
                '\t' => {
                    self.flush_tmux_literal(pane_target, &mut literal)?;
                    self.backend
                        .run(Some(&self.target), &["send-keys", "-t", pane_target, "Tab"])?;
                }
                c if c.is_control() => {
                    return Err(BackendError::unsupported(
                        format!("tmux input path does not support control character {:?}", c),
                        DegradedModeReason::UnsupportedByBackend,
                    ));
                }
                c => literal.push(c),
            }
        }

        self.flush_tmux_literal(pane_target, &mut literal)
    }

    fn flush_tmux_literal(
        &self,
        pane_target: &str,
        literal: &mut String,
    ) -> Result<(), BackendError> {
        if literal.is_empty() {
            return Ok(());
        }

        let args = vec![
            "send-keys".to_string(),
            "-t".to_string(),
            pane_target.to_string(),
            "-l".to_string(),
            literal.clone(),
        ];
        self.backend.run_owned(Some(&self.target), &args)?;
        literal.clear();

        Ok(())
    }
}
