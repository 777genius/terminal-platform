use super::{prelude::*, support::*};

#[test]
fn exposes_binding_version_from_protocol_contract() {
    let client = NodeHostClient::from_runtime_slug("terminal-node-binding-version");
    let version = client.binding_version();

    assert_eq!(version.binding_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(version.protocol.major, 0);
    assert_eq!(version.protocol.minor, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_node_host_flow_against_daemon_fixture() {
    let fixture = daemon_fixture("terminal-node-host").expect("fixture should start");
    let node = NodeHostClient::new(fixture.client.address().clone());

    let handshake = node.handshake_info().await.expect("handshake_info should succeed");
    let native_capabilities = node
        .backend_capabilities(NodeBackendKind::Native)
        .await
        .expect("native capabilities should succeed");
    let tmux_capabilities = node
        .backend_capabilities(NodeBackendKind::Tmux)
        .await
        .expect("tmux capabilities should succeed");
    let zellij_capabilities = node
        .backend_capabilities(NodeBackendKind::Zellij)
        .await
        .expect("zellij capabilities should succeed");
    let created = node
        .create_native_session(&cat_launch_request("shell"))
        .await
        .expect("create_native_session should succeed");
    let listed = node.list_sessions().await.expect("list_sessions should succeed");
    let attached =
        node.attach_session(&created.session_id).await.expect("attach_session should succeed");
    let topology = node
        .topology_snapshot(&created.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_pane_id =
        attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
    let ready_screen = wait_for_interactive_screen(
        &node,
        &created.session_id,
        &focused_pane_id,
        "node-host-roundtrip",
    )
    .await;
    let save = node
        .dispatch_mux_command(&created.session_id, &NodeMuxCommand::SaveSession)
        .await
        .expect("save session should succeed");
    let saved = node.list_saved_sessions().await.expect("list_saved_sessions should succeed");
    let loaded =
        node.saved_session(&created.session_id).await.expect("saved_session should succeed");
    let _input = node
        .dispatch_mux_command(
            &created.session_id,
            &NodeMuxCommand::SendInput(NodeSendInputCommand {
                pane_id: focused_pane_id.clone(),
                data: submitted_input("node host input"),
                client_event_id: None,
            }),
        )
        .await
        .expect("send input should succeed");
    let after_input =
        wait_for_screen_line(&node, &created.session_id, &focused_pane_id, "node host input").await;
    let delta = node
        .screen_delta(&created.session_id, &focused_pane_id, ready_screen.sequence)
        .await
        .expect("screen_delta should succeed");
    let new_tab = node
        .dispatch_mux_command(
            &created.session_id,
            &NodeMuxCommand::NewTab(NodeNewTabCommand { title: Some("logs".to_string()) }),
        )
        .await
        .expect("new tab should succeed");
    let topology_after_dispatch = node
        .topology_snapshot(&created.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let restored = node
        .restore_saved_session(&created.session_id)
        .await
        .expect("restore_saved_session should succeed");
    let deleted = node
        .delete_saved_session(&created.session_id)
        .await
        .expect("delete_saved_session should succeed");
    let saved_after_delete =
        node.list_saved_sessions().await.expect("list_saved_sessions should succeed");

    assert!(handshake.assessment.can_use);
    assert!(handshake.handshake.available_backends.contains(&NodeBackendKind::Native));
    assert_eq!(native_capabilities.backend, NodeBackendKind::Native);
    assert!(native_capabilities.capabilities.explicit_session_save);
    assert_eq!(tmux_capabilities.backend, NodeBackendKind::Tmux);
    assert!(tmux_capabilities.capabilities.read_only_client_mode);
    assert_eq!(zellij_capabilities.backend, NodeBackendKind::Zellij);
    if zellij_capabilities.capabilities.rendered_viewport_snapshot {
        assert!(zellij_capabilities.capabilities.tab_create);
        assert!(zellij_capabilities.capabilities.tab_close);
        assert_eq!(zellij_capabilities.capabilities.tab_focus, !cfg!(windows));
        assert!(zellij_capabilities.capabilities.tab_rename);
        assert!(zellij_capabilities.capabilities.rendered_viewport_stream);
        assert!(zellij_capabilities.capabilities.session_scoped_tab_refs);
        assert!(zellij_capabilities.capabilities.session_scoped_pane_refs);
        assert!(zellij_capabilities.capabilities.pane_close);
        assert_eq!(zellij_capabilities.capabilities.pane_focus, !cfg!(windows));
        assert!(zellij_capabilities.capabilities.pane_input_write);
        assert!(zellij_capabilities.capabilities.pane_paste_write);
        assert!(zellij_capabilities.capabilities.plugin_panes);
        assert!(zellij_capabilities.capabilities.advisory_metadata_subscriptions);
        assert!(zellij_capabilities.capabilities.read_only_client_mode);
    } else {
        assert!(!zellij_capabilities.capabilities.tab_create);
        assert!(!zellij_capabilities.capabilities.tab_close);
        assert!(!zellij_capabilities.capabilities.tab_focus);
        assert!(!zellij_capabilities.capabilities.tab_rename);
        assert!(!zellij_capabilities.capabilities.pane_close);
        assert!(!zellij_capabilities.capabilities.pane_focus);
        assert!(!zellij_capabilities.capabilities.pane_input_write);
        assert!(!zellij_capabilities.capabilities.pane_paste_write);
        assert!(!zellij_capabilities.capabilities.rendered_viewport_stream);
    }
    assert!(listed.iter().any(|session| session.session_id == created.session_id));
    assert_eq!(attached.session.session_id, created.session_id);
    assert_eq!(attached.topology.session_id, created.session_id);
    assert_eq!(topology.session_id, created.session_id);
    assert!(!topology.tabs.is_empty());
    assert_eq!(ready_screen.pane_id, focused_pane_id);
    assert!(!save.changed);
    assert!(saved.iter().any(|session| session.session_id == created.session_id));
    assert_eq!(loaded.session_id, created.session_id);
    assert!(loaded.compatibility.can_restore);
    let expected_launch = cat_launch_request("shell").launch;
    assert_eq!(loaded.launch, expected_launch);
    assert_eq!(loaded.restore_semantics.uses_saved_launch_spec, loaded.launch.is_some());
    assert!(after_input.sequence >= ready_screen.sequence);
    assert!(after_input.surface.lines.iter().any(|line| line.text.contains("node host input")));
    assert_eq!(delta.pane_id, focused_pane_id);
    assert!(delta.to_sequence >= delta.from_sequence);
    assert!(delta.patch.is_some() || delta.full_replace.is_some());
    assert!(new_tab.changed);
    assert_eq!(topology_after_dispatch.tabs.len(), 2);
    assert_eq!(restored.saved_session_id, created.session_id);
    assert_ne!(restored.session.session_id, created.session_id);
    assert_eq!(deleted.session_id, created.session_id);
    assert!(!saved_after_delete.iter().any(|session| session.session_id == created.session_id));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
