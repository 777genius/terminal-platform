use terminal_backend_api::{BackendError, BackendSessionSummary, CreateSessionSpec};
use terminal_domain::{SessionId, SessionRoute};
use tokio::sync::watch;

use super::{
    DEFAULT_COLS, DEFAULT_ROWS, NativeSessionEngine, NativeSessionState,
    process::{resolve_launch_spec, spawn_tab},
};

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
            state: std::sync::Mutex::new(NativeSessionState {
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
}
