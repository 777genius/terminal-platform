use super::{prelude::*, support::*};

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_list_and_get_saved_session_requests() {
    let daemon = isolated_daemon();
    let created = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                backend: terminal_domain::BackendKind::Native,
                spec: CreateSessionSpec {
                    title: Some("persisted-shell".to_string()),
                    launch: Some(cat_launch_spec()),
                },
            }),
        })
        .await
        .expect("create session should succeed");
    let session_id = match created.payload {
        ResponsePayload::CreateSession(created) => created.session.session_id,
        other => panic!("unexpected payload: {other:?}"),
    };

    let saved = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::DispatchMuxCommand(
                terminal_protocol::DispatchMuxCommandRequest {
                    session_id,
                    command: MuxCommand::SaveSession,
                },
            ),
        })
        .await
        .expect("save session should succeed");
    assert!(matches!(saved.payload, ResponsePayload::DispatchMuxCommand(_)));

    let list = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::ListSavedSessions,
        })
        .await
        .expect("list saved sessions should succeed");
    let get = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::GetSavedSession(terminal_protocol::GetSavedSessionRequest {
                session_id,
            }),
        })
        .await
        .expect("get saved session should succeed");

    match list.payload {
        ResponsePayload::ListSavedSessions(listed) => {
            assert_eq!(listed.sessions.len(), 1);
            assert_eq!(listed.sessions[0].session_id, session_id);
            assert_eq!(listed.sessions[0].title.as_deref(), Some("persisted-shell"));
            assert_eq!(
                listed.sessions[0].compatibility.status,
                SavedSessionCompatibilityStatus::Compatible
            );
            assert!(listed.sessions[0].compatibility.can_restore);
            assert!(listed.sessions[0].restore_semantics.restores_topology);
            assert!(listed.sessions[0].restore_semantics.restores_focus_state);
            assert!(listed.sessions[0].restore_semantics.restores_tab_titles);
            assert!(listed.sessions[0].restore_semantics.uses_saved_launch_spec);
            assert!(!listed.sessions[0].restore_semantics.preserves_process_state);
            let v2 = listed.sessions[0]
                .restore_semantics_v2
                .as_ref()
                .expect("v2 restore semantics should be present");
            assert_eq!(
                listed.sessions[0].restore_semantics.replays_saved_screen_buffers,
                v2.replays_saved_screen_buffers
            );
            assert!(
                matches!(
                    v2.restore_guarantee_level,
                    RestoreGuaranteeLevel::RichHistory
                        | RestoreGuaranteeLevel::BasicHistory
                        | RestoreGuaranteeLevel::VisualRestoreOnly
                ),
                "unexpected v2 restore semantics: {v2:?}"
            );
            assert!(matches!(
                v2.history_replay_state,
                HistoryReplayState::ReplayedFromJournal | HistoryReplayState::HydratedFromSnapshot
            ));
            assert_eq!(v2.source_session_id, session_id);
            assert!(!v2.has_known_gaps);
            assert_eq!(v2.latest_restore_drill_status.as_deref(), Some("passed"));
            assert!(
                v2.evidence_refs
                    .iter()
                    .any(|evidence| evidence.starts_with("stream_segment_count:"))
            );
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    match get.payload {
        ResponsePayload::SavedSession(saved) => {
            assert_eq!(saved.session.session_id, session_id);
            assert_eq!(saved.session.title.as_deref(), Some("persisted-shell"));
            assert_eq!(
                saved.session.compatibility.status,
                SavedSessionCompatibilityStatus::Compatible
            );
            assert!(saved.session.compatibility.can_restore);
            assert!(saved.session.restore_semantics.restores_topology);
            assert!(saved.session.restore_semantics.restores_focus_state);
            assert!(saved.session.restore_semantics.restores_tab_titles);
            assert!(saved.session.restore_semantics.uses_saved_launch_spec);
            assert!(!saved.session.restore_semantics.preserves_process_state);
            let v2 = saved
                .session
                .restore_semantics_v2
                .as_ref()
                .expect("v2 restore semantics should be present");
            assert_eq!(
                saved.session.restore_semantics.replays_saved_screen_buffers,
                v2.replays_saved_screen_buffers
            );
            assert!(
                matches!(
                    v2.restore_guarantee_level,
                    RestoreGuaranteeLevel::RichHistory
                        | RestoreGuaranteeLevel::BasicHistory
                        | RestoreGuaranteeLevel::VisualRestoreOnly
                ),
                "unexpected v2 restore semantics: {v2:?}"
            );
            assert!(matches!(
                v2.history_replay_state,
                HistoryReplayState::ReplayedFromJournal | HistoryReplayState::HydratedFromSnapshot
            ));
            assert_eq!(v2.source_session_id, session_id);
            assert_eq!(v2.latest_restore_drill_status.as_deref(), Some("passed"));
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_delete_saved_session_requests() {
    let daemon = isolated_daemon();
    let created = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                backend: terminal_domain::BackendKind::Native,
                spec: CreateSessionSpec {
                    title: Some("delete-shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            }),
        })
        .await
        .expect("create session should succeed");
    let session_id = match created.payload {
        ResponsePayload::CreateSession(created) => created.session.session_id,
        other => panic!("unexpected payload: {other:?}"),
    };

    daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::DispatchMuxCommand(
                terminal_protocol::DispatchMuxCommandRequest {
                    session_id,
                    command: MuxCommand::SaveSession,
                },
            ),
        })
        .await
        .expect("save session should succeed");

    let deleted = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::DeleteSavedSession(
                terminal_protocol::DeleteSavedSessionRequest { session_id },
            ),
        })
        .await
        .expect("delete saved session should succeed");

    match deleted.payload {
        ResponsePayload::DeleteSavedSession(response) => {
            assert_eq!(response.session_id, session_id);
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    let missing = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::GetSavedSession(terminal_protocol::GetSavedSessionRequest {
                session_id,
            }),
        })
        .await;
    assert!(missing.is_err());
}

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_restore_saved_session_requests() {
    let daemon = isolated_daemon();
    let created = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                backend: terminal_domain::BackendKind::Native,
                spec: CreateSessionSpec {
                    title: Some("restore-shell".to_string()),
                    launch: Some(cat_launch_spec()),
                },
            }),
        })
        .await
        .expect("create session should succeed");
    let session_id = match created.payload {
        ResponsePayload::CreateSession(created) => created.session.session_id,
        other => panic!("unexpected payload: {other:?}"),
    };

    daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::DispatchMuxCommand(
                terminal_protocol::DispatchMuxCommandRequest {
                    session_id,
                    command: MuxCommand::SaveSession,
                },
            ),
        })
        .await
        .expect("save session should succeed");

    let restored = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::RestoreSavedSession(
                terminal_protocol::RestoreSavedSessionRequest { session_id },
            ),
        })
        .await
        .expect("restore saved session should succeed");

    match restored.payload {
        ResponsePayload::RestoreSavedSession(response) => {
            assert_eq!(response.saved_session_id, session_id);
            assert_eq!(response.session.route.backend, terminal_domain::BackendKind::Native);
            assert_eq!(response.compatibility.status, SavedSessionCompatibilityStatus::Compatible);
            assert!(response.compatibility.can_restore);
            assert!(response.restore_semantics.restores_topology);
            assert!(response.restore_semantics.restores_focus_state);
            assert!(response.restore_semantics.restores_tab_titles);
            assert!(response.restore_semantics.uses_saved_launch_spec);
            assert!(!response.restore_semantics.preserves_process_state);
            let v2 = response
                .restore_semantics_v2
                .as_ref()
                .expect("v2 restore semantics should be present");
            assert_eq!(
                response.restore_semantics.replays_saved_screen_buffers,
                v2.replays_saved_screen_buffers
            );
            assert!(
                matches!(
                    v2.restore_guarantee_level,
                    RestoreGuaranteeLevel::RichHistory
                        | RestoreGuaranteeLevel::BasicHistory
                        | RestoreGuaranteeLevel::VisualRestoreOnly
                ),
                "unexpected v2 restore semantics: {v2:?}"
            );
            assert!(matches!(
                v2.history_replay_state,
                HistoryReplayState::ReplayedFromJournal | HistoryReplayState::HydratedFromSnapshot
            ));
            assert_eq!(v2.source_session_id, session_id);
            assert_eq!(v2.restored_session_id, Some(response.session.session_id));
            assert_eq!(v2.latest_restore_drill_status.as_deref(), Some("passed"));
            assert!(!v2.preserves_process_state);
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_prune_saved_sessions_requests() {
    let daemon = isolated_daemon();

    for label in ["one", "two", "three"] {
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some(label.to_string()),
                        ..CreateSessionSpec::default()
                    },
                }),
            })
            .await
            .expect("create session should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected payload: {other:?}"),
        };

        daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: MuxCommand::SaveSession,
                    },
                ),
            })
            .await
            .expect("save session should succeed");
    }

    let pruned = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::PruneSavedSessions(
                terminal_protocol::PruneSavedSessionsRequest { keep_latest: 1 },
            ),
        })
        .await
        .expect("prune saved sessions should succeed");
    let list = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::ListSavedSessions,
        })
        .await
        .expect("list saved sessions should succeed");

    match pruned.payload {
        ResponsePayload::PruneSavedSessions(pruned) => {
            assert_eq!(pruned.deleted_count, 2);
            assert_eq!(pruned.kept_count, 1);
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    match list.payload {
        ResponsePayload::ListSavedSessions(listed) => {
            assert_eq!(listed.sessions.len(), 1);
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn exposes_saved_session_degraded_reason_when_manifest_is_incompatible() {
    let manifest = SavedSessionManifest {
        binary_version: CURRENT_BINARY_VERSION.to_string(),
        protocol_major: CURRENT_PROTOCOL_MAJOR,
        protocol_minor: CURRENT_PROTOCOL_MINOR + 1,
        format_version: 1,
    };
    let (daemon, session_id) = save_incompatible_snapshot("protocol-minor-ahead", manifest);

    let listed = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::ListSavedSessions,
        })
        .await
        .expect("list saved sessions should succeed");
    let saved = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::GetSavedSession(terminal_protocol::GetSavedSessionRequest {
                session_id,
            }),
        })
        .await
        .expect("get saved session should succeed");
    let restored = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::RestoreSavedSession(
                terminal_protocol::RestoreSavedSessionRequest { session_id },
            ),
        })
        .await;

    match listed.payload {
        ResponsePayload::ListSavedSessions(listed) => {
            assert_eq!(listed.sessions.len(), 1);
            let session = &listed.sessions[0];
            assert_eq!(session.session_id, session_id);
            assert_eq!(
                session.compatibility.status,
                SavedSessionCompatibilityStatus::ProtocolMinorAhead
            );
            assert!(!session.compatibility.can_restore);
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    match saved.payload {
        ResponsePayload::SavedSession(saved) => {
            assert_eq!(saved.session.session_id, session_id);
            assert_eq!(
                saved.session.compatibility.status,
                SavedSessionCompatibilityStatus::ProtocolMinorAhead
            );
            assert!(!saved.session.compatibility.can_restore);
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    let error = restored.expect_err("restore should fail for incompatible saved session");
    assert_eq!(error.code, "backend_unsupported");
    assert_eq!(
        error.degraded_reason,
        Some(terminal_domain::DegradedModeReason::SavedSessionIncompatible)
    );
}
