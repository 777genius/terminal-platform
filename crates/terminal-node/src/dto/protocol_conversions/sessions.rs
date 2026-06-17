use crate::dto::{prelude::*, *};

impl From<&NodeShellLaunchSpec> for ShellLaunchSpec {
    fn from(value: &NodeShellLaunchSpec) -> Self {
        let mut spec = ShellLaunchSpec::new(value.program.clone()).with_args(value.args.clone());
        if let Some(cwd) = &value.cwd {
            spec = spec.with_cwd(cwd);
        }
        spec
    }
}

impl From<&ShellLaunchSpec> for NodeShellLaunchSpec {
    fn from(value: &ShellLaunchSpec) -> Self {
        Self {
            program: value.program.clone(),
            args: value.args.clone(),
            cwd: value.cwd.as_ref().map(|cwd| cwd.display().to_string()),
        }
    }
}

impl From<&NodeCreateSessionRequest> for CreateSessionSpec {
    fn from(value: &NodeCreateSessionRequest) -> Self {
        Self { title: value.title.clone(), launch: value.launch.as_ref().map(Into::into) }
    }
}

impl From<&BackendSessionSummary> for NodeSessionSummary {
    fn from(value: &BackendSessionSummary) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            route: (&value.route).into(),
            title: value.title.clone(),
        }
    }
}

impl From<&DiscoveredSession> for NodeDiscoveredSession {
    fn from(value: &DiscoveredSession) -> Self {
        Self { route: (&value.route).into(), title: value.title.clone() }
    }
}

impl From<&BackendCapabilities> for NodeBackendCapabilities {
    fn from(value: &BackendCapabilities) -> Self {
        Self {
            tiled_panes: value.tiled_panes,
            floating_panes: value.floating_panes,
            split_resize: value.split_resize,
            tab_create: value.tab_create,
            tab_close: value.tab_close,
            tab_focus: value.tab_focus,
            tab_rename: value.tab_rename,
            session_scoped_tab_refs: value.session_scoped_tab_refs,
            session_scoped_pane_refs: value.session_scoped_pane_refs,
            pane_split: value.pane_split,
            pane_close: value.pane_close,
            pane_focus: value.pane_focus,
            pane_input_write: value.pane_input_write,
            pane_paste_write: value.pane_paste_write,
            raw_output_stream: value.raw_output_stream,
            rendered_viewport_stream: value.rendered_viewport_stream,
            rendered_viewport_snapshot: value.rendered_viewport_snapshot,
            rendered_scrollback_snapshot: value.rendered_scrollback_snapshot,
            rich_screen_surface: value.rich_screen_surface,
            layout_dump: value.layout_dump,
            layout_override: value.layout_override,
            read_only_client_mode: value.read_only_client_mode,
            explicit_session_save: value.explicit_session_save,
            explicit_session_restore: value.explicit_session_restore,
            plugin_panes: value.plugin_panes,
            advisory_metadata_subscriptions: value.advisory_metadata_subscriptions,
            independent_resize_authority: value.independent_resize_authority,
        }
    }
}

impl From<&BackendCapabilitiesResponse> for NodeBackendCapabilitiesInfo {
    fn from(value: &BackendCapabilitiesResponse) -> Self {
        Self { backend: (&value.backend).into(), capabilities: (&value.capabilities).into() }
    }
}
