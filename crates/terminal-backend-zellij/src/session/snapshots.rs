use std::thread;

use terminal_backend_api::BackendError;
use terminal_domain::PaneId;
use terminal_projection::{ProjectionSource, ScreenLine, ScreenSnapshot};

use crate::{
    cli::is_transient_zellij_backend_error,
    constants::{ZELLIJ_POLL_INTERVAL, ZELLIJ_TRANSIENT_RETRY_ATTEMPTS},
    rows::{parse_panes_json, parse_tabs_json},
    screen::{screen_lines_from_output, screen_snapshot_from_lines},
    snapshot::{ZellijPaneTarget, ZellijSessionSnapshot, build_session_snapshot},
};

use super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(crate) fn snapshot(&self) -> Result<ZellijSessionSnapshot, BackendError> {
        let mut last_error = None;
        for attempt in 0..ZELLIJ_TRANSIENT_RETRY_ATTEMPTS {
            let snapshot_outputs: Result<(String, String), BackendError> = (|| {
                let _io_permit =
                    self.io_lane.lock().expect("zellij io lane should not be poisoned");
                let tabs_output =
                    self.backend.run(Some(&self.target), &["action", "list-tabs", "--json"])?;
                let panes_output =
                    self.backend.run(Some(&self.target), &["action", "list-panes", "--json"])?;
                Ok((tabs_output, panes_output))
            })();
            let (tabs_output, panes_output) = match snapshot_outputs {
                Ok(outputs) => outputs,
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
                Err(error) => return Err(error),
            };

            if tabs_output.trim().is_empty() || panes_output.trim().is_empty() {
                last_error = Some(BackendError::internal(
                    "zellij snapshot commands returned empty output while the session was still settling",
                ));
                if attempt + 1 < ZELLIJ_TRANSIENT_RETRY_ATTEMPTS {
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
            }

            let tabs = match parse_tabs_json(&tabs_output) {
                Ok(tabs) => tabs,
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
                Err(error) => return Err(error),
            };
            let panes = match parse_panes_json(&panes_output) {
                Ok(panes) => panes,
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
                Err(error) => return Err(error),
            };

            return build_session_snapshot(self.session_id, &self.target, &tabs, &panes);
        }

        Err(last_error.unwrap_or_else(|| {
            BackendError::transport("zellij snapshot never stabilized after retries")
        }))
    }

    pub(crate) fn pane_target(&self, pane_id: PaneId) -> Result<ZellijPaneTarget, BackendError> {
        self.snapshot()?
            .pane_targets
            .get(&pane_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij pane {pane_id:?}")))
    }

    pub(crate) fn screen_snapshot_inner(
        &self,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        let pane_target = self.pane_target(pane_id)?;
        let _io_permit = self.io_lane.lock().expect("zellij io lane should not be poisoned");
        let output = self
            .backend
            .run_owned(Some(&self.target), &dump_screen_scrollback_args(&pane_target))?;

        Ok(screen_snapshot_from_lines(
            pane_id,
            &pane_target,
            screen_lines_from_output(&output),
            ProjectionSource::ZellijDumpSnapshot,
        ))
    }

    pub(crate) fn screen_snapshot_from_viewport(
        &self,
        pane_id: PaneId,
        viewport: Vec<String>,
        source: ProjectionSource,
    ) -> Result<ScreenSnapshot, BackendError> {
        let pane_target = self.pane_target(pane_id)?;
        let lines = viewport.into_iter().map(|text| ScreenLine { text }).collect();

        Ok(screen_snapshot_from_lines(pane_id, &pane_target, lines, source))
    }
}

pub(crate) fn dump_screen_scrollback_args(pane_target: &ZellijPaneTarget) -> Vec<String> {
    vec![
        "action".to_string(),
        "dump-screen".to_string(),
        "--pane-id".to_string(),
        pane_target.backend_ref.clone(),
        "--full".to_string(),
    ]
}
