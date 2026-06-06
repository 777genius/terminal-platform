use terminal_backend_api::{BackendError, SendInputSpec, SendPasteSpec};
use terminal_domain::DegradedModeReason;

use crate::{
    action::ZellijAction,
    input::{flush_zellij_literal, zellij_control_key, zellij_named_key_sequence},
    snapshot::{ZellijPaneKind, ZellijSessionSnapshot},
};

use super::super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(super) fn send_input_actions(
        &self,
        snapshot: &ZellijSessionSnapshot,
        spec: SendInputSpec,
    ) -> Result<Vec<ZellijAction>, BackendError> {
        if spec.data.is_empty() {
            return Ok(Vec::new());
        }

        let pane_target = snapshot.pane_targets.get(&spec.pane_id).cloned().ok_or_else(|| {
            BackendError::not_found(format!("unknown zellij pane {:?}", spec.pane_id))
        })?;
        if pane_target.kind != ZellijPaneKind::Terminal {
            return Err(BackendError::unsupported(
                "zellij input writes target terminal panes only",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        let mut actions = Vec::new();
        let mut literal = String::new();
        let mut remaining = spec.data.as_str();
        while !remaining.is_empty() {
            if let Some((sequence, key)) = zellij_named_key_sequence(remaining) {
                flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);
                actions.push(ZellijAction::SendKeys {
                    pane_ref: pane_target.backend_ref.clone(),
                    keys: vec![key.to_string()],
                });
                remaining = &remaining[sequence.len()..];
                continue;
            }

            let Some(ch) = remaining.chars().next() else {
                break;
            };
            remaining = &remaining[ch.len_utf8()..];
            match ch {
                '\r' | '\n' => {
                    if ch == '\r' && remaining.starts_with('\n') {
                        remaining = &remaining['\n'.len_utf8()..];
                    }
                    flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);
                    actions.push(ZellijAction::SendKeys {
                        pane_ref: pane_target.backend_ref.clone(),
                        keys: vec!["Enter".to_string()],
                    });
                }
                '\t' => {
                    flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);
                    actions.push(ZellijAction::SendKeys {
                        pane_ref: pane_target.backend_ref.clone(),
                        keys: vec!["Tab".to_string()],
                    });
                }
                c if c.is_control() => {
                    if let Some(key) = zellij_control_key(c) {
                        flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);
                        actions.push(ZellijAction::SendKeys {
                            pane_ref: pane_target.backend_ref.clone(),
                            keys: vec![key.to_string()],
                        });
                    } else {
                        return Err(BackendError::unsupported(
                            format!("zellij input path does not support control character {:?}", c),
                            DegradedModeReason::UnsupportedByBackend,
                        ));
                    }
                }
                c => literal.push(c),
            }
        }
        flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);

        Ok(actions)
    }

    pub(super) fn send_paste_actions(
        &self,
        snapshot: &ZellijSessionSnapshot,
        spec: SendPasteSpec,
    ) -> Result<Vec<ZellijAction>, BackendError> {
        if cfg!(windows) {
            return self.send_input_actions(
                snapshot,
                SendInputSpec {
                    pane_id: spec.pane_id,
                    data: spec.data,
                    client_event_id: spec.client_event_id,
                },
            );
        }

        if spec.data.is_empty() {
            return Ok(Vec::new());
        }

        let pane_target = snapshot.pane_targets.get(&spec.pane_id).cloned().ok_or_else(|| {
            BackendError::not_found(format!("unknown zellij pane {:?}", spec.pane_id))
        })?;
        if pane_target.kind != ZellijPaneKind::Terminal {
            return Err(BackendError::unsupported(
                "zellij paste writes target terminal panes only",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        Ok(vec![ZellijAction::Paste { pane_ref: pane_target.backend_ref, text: spec.data }])
    }
}
