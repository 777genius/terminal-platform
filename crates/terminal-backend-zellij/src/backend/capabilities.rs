use terminal_backend_api::{BackendCapabilities, BackendError, BoxFuture};

use crate::{cli::zellij_focus_actions_supported, probe::ZellijSurface};

use super::ZellijBackend;

impl ZellijBackend {
    pub(super) fn capabilities_inner(
        &self,
    ) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async move {
            let probe = self.probe()?;
            Ok(capabilities_for_surface(probe.surface))
        })
    }
}

pub(crate) fn capabilities_for_surface(surface: ZellijSurface) -> BackendCapabilities {
    match surface {
        ZellijSurface::RichCli044Plus => BackendCapabilities {
            tiled_panes: true,
            tab_create: true,
            tab_close: true,
            tab_focus: zellij_focus_actions_supported(),
            tab_rename: true,
            session_scoped_tab_refs: true,
            session_scoped_pane_refs: true,
            pane_close: true,
            pane_focus: zellij_focus_actions_supported(),
            pane_input_write: true,
            pane_paste_write: true,
            rendered_viewport_stream: true,
            rendered_viewport_snapshot: true,
            rendered_scrollback_snapshot: true,
            plugin_panes: true,
            advisory_metadata_subscriptions: true,
            read_only_client_mode: true,
            ..BackendCapabilities::default()
        },
        ZellijSurface::LegacyCli043 => {
            BackendCapabilities { read_only_client_mode: true, ..BackendCapabilities::default() }
        }
        ZellijSurface::Unknown => BackendCapabilities::default(),
    }
}
