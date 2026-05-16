use super::super::prelude::*;

#[test]
fn bootstrap_smoke_exposes_empty_daemon() {
    let daemon = daemon();
    let handshake = daemon.handshake();

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 2);
    assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
    assert_eq!(
        handshake.available_backends,
        terminal_daemon::backend_registry::compiled_backend_kinds()
    );
    assert!(handshake.capabilities.request_reply);
    assert!(handshake.capabilities.topology_subscriptions);
    assert!(handshake.capabilities.pane_subscriptions);
    assert!(handshake.capabilities.backend_discovery);
    assert!(handshake.capabilities.backend_capability_queries);
    assert!(handshake.capabilities.saved_sessions);
    assert!(handshake.capabilities.session_restore);
    assert!(handshake.capabilities.degraded_error_reasons);
    assert!(handshake.capabilities.session_health);
    assert_eq!(daemon.session_count(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_reports_dynamic_backend_capabilities() {
    let fixture = daemon_fixture("bootstrap-backend-capabilities").expect("fixture should start");

    let native = fixture
        .client
        .backend_capabilities(BackendKind::Native)
        .await
        .expect("native capabilities should succeed");
    let tmux = fixture
        .client
        .backend_capabilities(BackendKind::Tmux)
        .await
        .expect("tmux capabilities should succeed");
    let zellij = fixture
        .client
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
    assert_eq!(tmux.backend, BackendKind::Tmux);
    assert!(tmux.capabilities.read_only_client_mode);
    assert!(tmux.capabilities.split_resize);
    assert!(tmux.capabilities.tab_create);
    assert!(tmux.capabilities.tab_close);
    assert!(tmux.capabilities.tab_focus);
    assert!(tmux.capabilities.tab_rename);
    assert!(tmux.capabilities.pane_split);
    assert!(tmux.capabilities.pane_close);
    assert!(tmux.capabilities.pane_focus);
    assert!(tmux.capabilities.pane_input_write);
    assert!(tmux.capabilities.rendered_viewport_stream);
    assert_eq!(zellij.backend, BackendKind::Zellij);
    assert!(zellij.capabilities.read_only_client_mode);
    assert!(!zellij.capabilities.explicit_session_save);
    assert!(!zellij.capabilities.explicit_session_restore);
    assert!(!zellij.capabilities.split_resize);
    assert!(!zellij.capabilities.pane_split);
    if zellij.capabilities.rendered_viewport_snapshot {
        assert!(zellij.capabilities.tiled_panes);
        assert!(zellij.capabilities.tab_create);
        assert!(zellij.capabilities.tab_close);
        assert_eq!(zellij.capabilities.tab_focus, !cfg!(windows));
        assert!(zellij.capabilities.tab_rename);
        assert!(zellij.capabilities.session_scoped_tab_refs);
        assert!(zellij.capabilities.session_scoped_pane_refs);
        assert!(zellij.capabilities.pane_close);
        assert_eq!(zellij.capabilities.pane_focus, !cfg!(windows));
        assert!(zellij.capabilities.pane_input_write);
        assert!(zellij.capabilities.pane_paste_write);
        assert!(zellij.capabilities.rendered_viewport_stream);
        assert!(zellij.capabilities.plugin_panes);
        assert!(zellij.capabilities.advisory_metadata_subscriptions);
        assert!(!zellij.capabilities.floating_panes);
        assert!(!zellij.capabilities.rendered_scrollback_snapshot);
    } else {
        assert!(!zellij.capabilities.tab_create);
        assert!(!zellij.capabilities.tab_close);
        assert!(!zellij.capabilities.tab_focus);
        assert!(!zellij.capabilities.tab_rename);
        assert!(!zellij.capabilities.tiled_panes);
        assert!(!zellij.capabilities.session_scoped_tab_refs);
        assert!(!zellij.capabilities.session_scoped_pane_refs);
        assert!(!zellij.capabilities.pane_close);
        assert!(!zellij.capabilities.pane_focus);
        assert!(!zellij.capabilities.pane_input_write);
        assert!(!zellij.capabilities.pane_paste_write);
        assert!(!zellij.capabilities.rendered_viewport_stream);
        assert!(!zellij.capabilities.plugin_panes);
        assert!(!zellij.capabilities.advisory_metadata_subscriptions);
    }

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_roundtrips_request_reply_flow() {
    let fixture = daemon_fixture("bootstrap-smoke").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let handshake = fixture.client.handshake().await.expect("handshake should succeed");
    let handshake_assessment =
        fixture.client.handshake_assessment().await.expect("handshake_assessment should succeed");
    let sessions = fixture.client.list_sessions().await.expect("list_sessions should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let screen = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let delta = fixture
        .client
        .screen_delta(created.session.session_id, pane_id, screen.sequence)
        .await
        .expect("screen_delta should succeed");
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
        )
        .await
        .expect("dispatch should succeed");
    let topology_after_dispatch = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 2);
    assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
    assert!(handshake.capabilities.request_reply);
    assert!(handshake.capabilities.saved_sessions);
    assert!(handshake.capabilities.session_health);
    assert!(handshake_assessment.can_use);
    assert_eq!(created.session.route.backend, BackendKind::Native);
    assert_eq!(created.session.title.as_deref(), Some("shell"));
    assert_eq!(sessions.sessions.len(), 1);
    assert_eq!(sessions.sessions[0].session_id, created.session.session_id);
    assert_eq!(topology.session_id, created.session.session_id);
    assert_eq!(screen.pane_id, pane_id);
    assert!(!screen.surface.lines.is_empty());
    assert_eq!(delta.rows, screen.rows);
    assert_eq!(delta.cols, screen.cols);
    assert!(delta.patch.is_none());
    assert_eq!(delta.from_sequence, screen.sequence);
    assert_eq!(delta.to_sequence, screen.sequence);
    assert!(delta.full_replace.is_none());
    assert!(dispatch.changed);
    assert_eq!(topology_after_dispatch.tabs.len(), 2);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
