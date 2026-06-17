use super::super::*;
pub(super) use terminal_domain::{
    BackendKind, PaneId, RouteAuthority, SavedSessionManifest, SessionId, SessionRoute, TabId,
};
pub(super) use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
pub(super) use terminal_projection::{
    ProjectionSource, ScreenBufferKind, ScreenLine, ScreenSurface,
};

pub(super) fn test_store(label: &str) -> TerminalPersistenceV2 {
    let path = std::env::temp_dir()
        .join(format!("terminal-persistence-v2-{label}-{}.sqlite3", Uuid::new_v4()));
    TerminalPersistenceV2::open_with_config(path, TerminalPersistenceV2Config::test())
        .expect("v2 store should open")
}

pub(super) fn route() -> SessionRoute {
    SessionRoute {
        backend: BackendKind::Native,
        authority: RouteAuthority::LocalDaemon,
        external: None,
    }
}

pub(super) fn session_and_pane(
    store: &TerminalPersistenceV2,
) -> (String, String, WriterGenerationLease) {
    let session_id = store.create_session(SessionInput::new(route())).expect("session should save");
    let pane_id =
        store.create_pane(PaneInput::new(session_id.clone(), 24, 80)).expect("pane should save");
    let writer =
        store.acquire_writer_generation("test-process", 60_000).expect("writer should acquire");
    (session_id, pane_id, writer)
}
