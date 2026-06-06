use terminal_backend_api::{
    BackendError, NewTabSpec, OverrideLayoutSpec, ResizePaneSpec, SendInputSpec, SendPasteSpec,
    SplitPaneSpec,
};
use terminal_domain::{PaneId, TabId};

use super::{
    NativeSessionEngine,
    dispatch::{
        dispatch_close_pane, dispatch_close_tab, dispatch_focus_pane, dispatch_focus_tab,
        dispatch_new_tab, dispatch_override_layout, dispatch_rename_tab, dispatch_resize_pane,
        dispatch_send_input, dispatch_send_paste, dispatch_split_pane,
    },
    layout::collect_surface_updates,
};

impl NativeSessionEngine {
    pub(crate) fn new_tab(&self, spec: NewTabSpec) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let changed = dispatch_new_tab(&mut state, spec)?;
        self.finish_mutation(&state, changed, Vec::new());
        Ok(changed)
    }

    pub(crate) fn split_pane(&self, spec: SplitPaneSpec) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let changed = dispatch_split_pane(&mut state, spec)?;
        self.finish_mutation(&state, changed, Vec::new());
        Ok(changed)
    }

    pub(crate) fn focus_tab(&self, tab_id: TabId) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let changed = dispatch_focus_tab(&mut state, tab_id)?;
        self.finish_mutation(&state, changed, Vec::new());
        Ok(changed)
    }

    pub(crate) fn rename_tab(&self, tab_id: TabId, title: String) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let (changed, surface_updates) = dispatch_rename_tab(&mut state, tab_id, title)?;
        self.finish_mutation(&state, changed, surface_updates);
        Ok(changed)
    }

    pub(crate) fn focus_pane(&self, pane_id: PaneId) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let changed = dispatch_focus_pane(&mut state, pane_id)?;
        self.finish_mutation(&state, changed, Vec::new());
        Ok(changed)
    }

    pub(crate) fn close_pane(&self, pane_id: PaneId) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let changed = dispatch_close_pane(&mut state, pane_id)?;
        self.finish_mutation(&state, changed, Vec::new());
        Ok(changed)
    }

    pub(crate) fn close_tab(&self, tab_id: TabId) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let changed = dispatch_close_tab(&mut state, tab_id)?;
        self.finish_mutation(&state, changed, Vec::new());
        Ok(changed)
    }

    pub(crate) fn resize_pane(&self, spec: ResizePaneSpec) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let pane_id = spec.pane_id;
        let changed = dispatch_resize_pane(&mut state, spec)?;
        let surface_updates =
            if changed { collect_surface_updates(&state, pane_id) } else { Vec::new() };
        self.finish_mutation(&state, changed, surface_updates);
        Ok(changed)
    }

    pub(crate) fn override_layout(&self, spec: OverrideLayoutSpec) -> Result<bool, BackendError> {
        let mut state = self.lock_state()?;
        let (changed, surface_updates) = dispatch_override_layout(&mut state, spec)?;
        self.finish_mutation(&state, changed, surface_updates);
        Ok(changed)
    }

    pub(crate) fn send_input(&self, spec: SendInputSpec) -> Result<bool, BackendError> {
        let state = self.lock_state()?;
        dispatch_send_input(&state, spec)
    }

    pub(crate) fn send_paste(&self, spec: SendPasteSpec) -> Result<bool, BackendError> {
        let state = self.lock_state()?;
        dispatch_send_paste(&state, spec)
    }
}
