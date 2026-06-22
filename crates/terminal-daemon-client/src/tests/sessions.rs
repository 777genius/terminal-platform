use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn fetches_session_health_snapshot_for_native_session() {
    let address = unique_address("daemon-client-session-health");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);

    let created = client
        .create_session(BackendKind::Native, CreateSessionSpec::default())
        .await
        .expect("native session should be created");

    let health = client
        .session_health_snapshot(created.session.session_id)
        .await
        .expect("session health should succeed");

    assert_eq!(health.session_id, created.session.session_id);
    assert_eq!(health.phase, terminal_projection::SessionHealthPhase::Ready);
    assert!(health.can_attach);
    assert!(!health.invalidated);
    assert_eq!(health.reason, None);

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn fetches_backend_capabilities() {
    let address = unique_address("daemon-client-capabilities");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);

    let native = client
        .backend_capabilities(BackendKind::Native)
        .await
        .expect("native capabilities should succeed");
    let zellij = client
        .backend_capabilities(BackendKind::Zellij)
        .await
        .expect("zellij capabilities should succeed");

    assert_eq!(native.backend, BackendKind::Native);
    assert!(native.capabilities.tiled_panes);
    assert!(native.capabilities.split_resize);
    assert!(native.capabilities.tab_create);
    assert!(native.capabilities.tab_close);
    assert!(native.capabilities.tab_focus);
    assert!(native.capabilities.tab_rename);
    assert!(native.capabilities.pane_split);
    assert!(native.capabilities.pane_close);
    assert!(native.capabilities.pane_focus);
    assert!(native.capabilities.pane_input_write);
    assert!(native.capabilities.layout_dump);
    assert!(native.capabilities.layout_override);
    assert!(native.capabilities.explicit_session_save);
    assert!(native.capabilities.explicit_session_restore);
    assert!(native.capabilities.rendered_viewport_stream);
    assert_eq!(zellij.backend, BackendKind::Zellij);
    assert!(zellij.capabilities.read_only_client_mode);
    assert!(!zellij.capabilities.explicit_session_save);
    assert!(!zellij.capabilities.explicit_session_restore);

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn creates_native_session_and_lists_it_back() {
    let address = unique_address("daemon-client-create");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);

    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let sessions = client.list_sessions().await.expect("list_sessions should succeed");

    assert_eq!(created.session.route.backend, BackendKind::Native);
    assert_eq!(created.session.title.as_deref(), Some("shell"));
    assert_eq!(sessions.sessions.len(), 1);
    assert_eq!(sessions.sessions[0].session_id, created.session.session_id);

    server.shutdown().await.expect("server shutdown should succeed");
}
