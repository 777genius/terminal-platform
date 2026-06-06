use super::super::super::*;
use super::super::support::*;

#[test]
fn delete_request_writes_tombstone_without_deleting_canonical_history() {
    let store = test_store("delete-workflow");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"history must not disappear silently\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let request = store
        .create_delete_request(DeleteRequestInput {
            id: None,
            session_id: Some(session_id.clone()),
            request_kind: Some("user_delete".to_string()),
            policy_id: None,
            requester_ref: Some("local-user".to_string()),
            reason: Some("test delete request".to_string()),
            metadata: None,
        })
        .expect("delete request should persist");
    let tombstone = store
        .complete_delete_request_with_tombstone(
            &request.id,
            "session",
            Some(serde_json::json!({"canonical_delete_deferred": true})),
            None,
        )
        .expect("tombstone should persist");
    let segments = store
        .list_stream_segments(&session_id, &pane_id, 1, 10)
        .expect("canonical history should remain readable");

    assert_eq!(request.state, "pending");
    assert_eq!(tombstone.session_id.as_deref(), Some(session_id.as_str()));
    assert_eq!(tombstone.deleted_scope, "session");
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].payload, b"history must not disappear silently\r\n");
}

#[test]
fn canonical_history_prevents_parent_delete() {
    let store = test_store("restrict-delete");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id,
            writer.id,
            b"must stay durable\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let mut connection = store.connection().expect("connection should open");
    let delete_result =
        diesel::delete(terminal_sessions::table.filter(terminal_sessions::id.eq(&session_id)))
            .execute(&mut connection);

    assert!(delete_result.is_err());
}
