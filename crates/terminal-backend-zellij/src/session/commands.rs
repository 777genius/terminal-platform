use std::time::Instant;

use terminal_backend_api::{
    BackendError, MuxCommand, MuxCommandResult, NewTabSpec, SendInputSpec, SendPasteSpec,
};
use terminal_domain::{DegradedModeReason, PaneId, TabId};
use tokio::time;

use crate::{
    action::ZellijAction,
    cli::{
        is_transient_zellij_backend_error, zellij_focus_actions_supported,
        zellij_focus_unsupported_error,
    },
    constants::{
        ZELLIJ_ACTION_SETTLE_ATTEMPTS, ZELLIJ_ACTION_SETTLE_TIMEOUT, ZELLIJ_POLL_INTERVAL,
    },
    input::{flush_zellij_literal, zellij_control_key, zellij_named_key_sequence},
    snapshot::{ZellijPaneKind, ZellijSessionSnapshot, collect_pane_ids, tab_contains_pane},
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

    fn new_tab_actions(&self, spec: NewTabSpec) -> Vec<ZellijAction> {
        vec![ZellijAction::NewTab { title: spec.title }]
    }

    fn focus_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
    ) -> Result<ZellijAction, BackendError> {
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;
        Ok(ZellijAction::FocusTab {
            backend_tab_id: tab_target.backend_tab_id,
            display_index: tab_target.display_index,
        })
    }

    fn close_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
    ) -> Result<ZellijAction, BackendError> {
        if snapshot.topology.tabs.len() <= 1 {
            return Err(BackendError::unsupported(
                "zellij imported routes refuse to close the last tab because it would terminate the foreign session",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;

        Ok(ZellijAction::CloseTab { backend_tab_id: tab_target.backend_tab_id })
    }

    fn rename_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
        title: &str,
    ) -> Result<ZellijAction, BackendError> {
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;
        Ok(ZellijAction::RenameTab {
            backend_tab_id: tab_target.backend_tab_id,
            title: title.to_string(),
        })
    }

    fn focus_pane_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        pane_id: PaneId,
    ) -> Result<ZellijAction, BackendError> {
        let pane_target =
            snapshot.pane_targets.get(&pane_id).cloned().ok_or_else(|| {
                BackendError::not_found(format!("unknown zellij pane {pane_id:?}"))
            })?;
        Ok(ZellijAction::FocusPane { pane_ref: pane_target.backend_ref })
    }

    fn close_pane_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        pane_id: PaneId,
    ) -> Result<ZellijAction, BackendError> {
        let pane_target =
            snapshot.pane_targets.get(&pane_id).cloned().ok_or_else(|| {
                BackendError::not_found(format!("unknown zellij pane {pane_id:?}"))
            })?;
        let tab = snapshot
            .topology
            .tabs
            .iter()
            .find(|tab| tab_contains_pane(tab, pane_id))
            .ok_or_else(|| {
                BackendError::not_found(format!("zellij pane {pane_id:?} is not bound to a tab"))
            })?;
        if collect_pane_ids(&tab.root).len() <= 1 {
            return Err(BackendError::unsupported(
                "zellij imported routes refuse to close the last pane in a tab because it would collapse tab lifecycle into tab closure semantics",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        Ok(ZellijAction::ClosePane { pane_ref: pane_target.backend_ref })
    }

    fn send_input_actions(
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

    fn send_paste_actions(
        &self,
        snapshot: &ZellijSessionSnapshot,
        spec: SendPasteSpec,
    ) -> Result<Vec<ZellijAction>, BackendError> {
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

    async fn wait_for_action_settle(
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
