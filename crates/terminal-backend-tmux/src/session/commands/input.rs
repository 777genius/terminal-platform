use super::super::TmuxAttachedSession;
use crate::prelude::*;

impl TmuxAttachedSession {
    pub(super) fn send_input(&self, spec: SendInputSpec) -> Result<MuxCommandResult, BackendError> {
        self.send_text_to_pane(spec.pane_id, &spec.data)
    }

    pub(super) fn send_paste(&self, spec: SendPasteSpec) -> Result<MuxCommandResult, BackendError> {
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
