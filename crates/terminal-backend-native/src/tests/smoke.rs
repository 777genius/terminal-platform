use terminal_backend_api::{
    BackendScope, CreateSessionSpec, MuxBackendPort, MuxCommand, NewTabSpec,
};
use terminal_domain::BackendKind;
use terminal_projection::ProjectionSource;

use crate::NativeBackend;

#[cfg(any(unix, windows))]
use super::support::quiet_launch_spec;

#[tokio::test]
async fn creates_and_lists_empty_sessions() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            ..CreateSessionSpec::default()
        })
        .await
        .expect("native session should be created");
    let sessions = backend
        .list_sessions(BackendScope::CurrentUser)
        .await
        .expect("list_sessions should succeed");

    assert_eq!(backend.kind(), BackendKind::Native);
    assert_eq!(binding.route.backend, BackendKind::Native);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, binding.session_id);
    assert_eq!(sessions[0].title.as_deref(), Some("shell"));
}

#[tokio::test]
async fn attaches_and_exposes_topology_and_screen() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            ..CreateSessionSpec::default()
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route.clone())
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("tab should expose a focused pane");
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let delta =
        session.screen_delta(pane_id, screen.sequence).await.expect("screen delta should succeed");

    assert_eq!(topology.session_id, binding.session_id);
    assert_eq!(topology.tabs.len(), 1);
    assert_eq!(screen.pane_id, pane_id);
    assert_eq!(screen.source, ProjectionSource::NativeEmulator);
    assert!(!screen.surface.lines.is_empty());
    assert_eq!(delta.pane_id, pane_id);
    assert_eq!(delta.from_sequence, screen.sequence);
    assert_eq!(delta.to_sequence, screen.sequence);
    assert_eq!(delta.source, ProjectionSource::NativeEmulator);
    assert_eq!(delta.rows, screen.rows);
    assert_eq!(delta.cols, screen.cols);
    assert!(delta.patch.is_none());
    assert!(delta.full_replace.is_none());
}

#[tokio::test]
async fn mutates_topology_through_dispatch() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            ..CreateSessionSpec::default()
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let before = session.topology_snapshot().await.expect("topology should succeed");
    let focused_pane = before.tabs[0].focused_pane.expect("focused pane should exist");

    let new_tab = session
        .dispatch(MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }))
        .await
        .expect("new tab should succeed");
    let focus_same_pane = session
        .dispatch(MuxCommand::FocusPane { pane_id: focused_pane })
        .await
        .expect("focus pane should succeed");
    let after = session.topology_snapshot().await.expect("topology should succeed");

    assert!(new_tab.changed);
    assert!(focus_same_pane.changed);
    assert_eq!(after.tabs.len(), 2);
    assert_eq!(after.focused_tab, Some(before.tabs[0].tab_id));
}

#[tokio::test]
async fn emits_screen_delta_for_tab_title_changes() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(quiet_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let tab_id = topology.tabs[0].tab_id;
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let before = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");

    let result = session
        .dispatch(MuxCommand::RenameTab { tab_id, title: "renamed".to_string() })
        .await
        .expect("rename tab should succeed");
    let delta =
        session.screen_delta(pane_id, before.sequence).await.expect("screen delta should succeed");
    let patch = delta.patch.expect("delta patch should exist");

    assert!(result.changed);
    assert!(delta.to_sequence > before.sequence);
    assert!(patch.title_changed);
    assert_eq!(patch.title.as_deref(), Some("renamed"));
    assert!(delta.full_replace.is_none());
}
