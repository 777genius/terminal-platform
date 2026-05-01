use std::sync::Mutex;

use terminal_backend_api::{
    BackendError, BackendRawOutputEvent, BackendSessionSummary, CreateSessionSpec, NewTabSpec,
    OverrideLayoutSpec, ResizePaneSpec, SendInputSpec, SendPasteSpec, SplitPaneSpec,
};
use terminal_domain::{PaneId, SessionId, SessionRoute, TabId};
use terminal_mux_domain::TabSnapshot;
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};
use tokio::sync::{broadcast, watch};

use self::{
    dispatch::{
        dispatch_close_pane, dispatch_close_tab, dispatch_focus_pane, dispatch_focus_tab,
        dispatch_new_tab, dispatch_override_layout, dispatch_rename_tab, dispatch_resize_pane,
        dispatch_send_input, dispatch_send_paste, dispatch_split_pane,
    },
    layout::collect_surface_updates,
    model::NativeSessionState,
    process::{resolve_launch_spec, spawn_tab},
    signals::bump_watch,
};

mod dispatch;
mod input;
mod layout;
mod model;
mod process;
mod projection;
mod signals;

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;
const SNAPSHOT_HISTORY_LIMIT: usize = 64;
const SPLIT_RATIO_SCALE: u16 = 10_000;
const DEFAULT_SPLIT_RATIO_BPS: u16 = SPLIT_RATIO_SCALE / 2;

pub(crate) struct NativeSessionEngine {
    session_id: SessionId,
    state: Mutex<NativeSessionState>,
    topology_tick: watch::Sender<u64>,
}

impl NativeSessionEngine {
    pub(crate) fn spawn(
        session_id: SessionId,
        route: SessionRoute,
        spec: CreateSessionSpec,
    ) -> Result<Self, BackendError> {
        let launch = resolve_launch_spec(spec.launch)?;
        let (topology_tick, _) = watch::channel(0_u64);
        let first_tab = spawn_tab(spec.title.clone(), &launch, DEFAULT_ROWS, DEFAULT_COLS)?;
        let summary = BackendSessionSummary { session_id, route, title: spec.title };

        Ok(Self {
            session_id,
            state: Mutex::new(NativeSessionState {
                summary,
                launch,
                focused_tab: first_tab.tab_id,
                tabs: vec![first_tab],
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
            }),
            topology_tick,
        })
    }

    pub(crate) fn summary(&self) -> Result<BackendSessionSummary, BackendError> {
        Ok(self.lock_state()?.summary.clone())
    }

    pub(crate) fn topology_snapshot(&self) -> Result<TopologySnapshot, BackendError> {
        let state = self.lock_state()?;

        Ok(TopologySnapshot {
            session_id: self.session_id,
            backend_kind: terminal_domain::BackendKind::Native,
            focused_tab: Some(state.focused_tab),
            tabs: state
                .tabs
                .iter()
                .map(|tab| TabSnapshot {
                    tab_id: tab.tab_id,
                    title: tab.title.clone(),
                    root: tab.root.snapshot(),
                    focused_pane: Some(tab.focused_pane),
                })
                .collect(),
        })
    }

    pub(crate) fn screen_snapshot(&self, pane_id: PaneId) -> Result<ScreenSnapshot, BackendError> {
        let state = self.lock_state()?;
        let (tab, pane) = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id).map(|pane| (tab, pane)))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        pane.render_snapshot(tab.title.clone().or_else(|| state.summary.title.clone()))
    }

    pub(crate) fn screen_delta(
        &self,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let state = self.lock_state()?;
        let (tab, pane) = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id).map(|pane| (tab, pane)))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        pane.screen_delta(tab.title.clone().or_else(|| state.summary.title.clone()), from_sequence)
    }

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

    pub(crate) fn subscribe_topology(&self) -> watch::Receiver<u64> {
        self.topology_tick.subscribe()
    }

    pub(crate) fn subscribe_pane_surface(
        &self,
        pane_id: PaneId,
    ) -> Result<watch::Receiver<u64>, BackendError> {
        let state = self.lock_state()?;
        let pane = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        Ok(pane.surface_tick.subscribe())
    }

    pub(crate) fn subscribe_pane_raw_output(
        &self,
        pane_id: PaneId,
    ) -> Result<broadcast::Receiver<BackendRawOutputEvent>, BackendError> {
        let state = self.lock_state()?;
        let pane = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        Ok(pane.raw_output_tick.subscribe())
    }

    fn finish_mutation(
        &self,
        state: &NativeSessionState,
        changed: bool,
        surface_updates: Vec<PaneId>,
    ) {
        if changed {
            bump_watch(&self.topology_tick);
        }
        for pane_id in surface_updates {
            if let Some(pane) = state.tabs.iter().find_map(|tab| tab.pane(pane_id)) {
                pane.mark_surface_dirty();
            }
        }
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, NativeSessionState>, BackendError> {
        self.state.lock().map_err(|_| BackendError::internal("native session state lock poisoned"))
    }
}
