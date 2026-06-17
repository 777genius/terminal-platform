use crate::prelude::BackendCapabilities;

pub(super) fn tmux_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        tiled_panes: true,
        split_resize: true,
        tab_create: true,
        tab_close: true,
        tab_focus: true,
        tab_rename: true,
        session_scoped_tab_refs: true,
        session_scoped_pane_refs: true,
        pane_split: true,
        pane_close: true,
        pane_focus: true,
        pane_input_write: true,
        pane_paste_write: true,
        rendered_viewport_stream: true,
        rendered_viewport_snapshot: true,
        rich_screen_surface: true,
        advisory_metadata_subscriptions: true,
        read_only_client_mode: true,
        ..BackendCapabilities::default()
    }
}
