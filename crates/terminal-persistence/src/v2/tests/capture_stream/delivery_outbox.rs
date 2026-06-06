use super::super::super::*;
use super::super::support::*;

#[test]
fn records_delivery_offsets_and_builds_replay_window() {
    let store = test_store("delivery-offset");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"first\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"second\r\n".to_vec(),
        ))
        .expect("second segment should persist");
    let client = store
        .upsert_delivery_client(DeliveryClientInput {
            id: Some("browser-a".to_string()),
            client_kind: "browser".to_string(),
            install_ref_hash: None,
            browser_profile_ref_hash: None,
            user_agent_hash: None,
            trust_state: None,
        })
        .expect("client should persist");

    let sent = store
        .record_delivery_progress(DeliveryProgressInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
            last_sent_event_seq: Some(2),
            last_acked_event_seq: None,
        })
        .expect("sent offset should persist");
    let acked = store
        .record_delivery_progress(DeliveryProgressInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
            last_sent_event_seq: None,
            last_acked_event_seq: Some(1),
        })
        .expect("acked offset should persist");
    let window = store
        .delivery_replay_window(DeliveryOffsetInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
        })
        .expect("replay window should load");
    let replay = store
        .hydrate_pane_history(&session_id, &pane_id, window.from_event_seq, Some(10), Some(1024))
        .expect("replay history should hydrate");

    assert_eq!(sent.last_sent_event_seq, 2);
    assert_eq!(acked.last_acked_event_seq, 1);
    assert_eq!(acked.replay_from_event_seq, Some(2));
    assert_eq!(window.from_event_seq, Some(2));
    assert_eq!(window.to_event_seq, 2);
    assert_eq!(window.gap_state, "none");
    assert_eq!(replay.segments.len(), 1);
    assert_eq!(replay.segments[0].payload, b"second\r\n");

    let fully_acked = store
        .record_delivery_progress(DeliveryProgressInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
            last_sent_event_seq: None,
            last_acked_event_seq: Some(2),
        })
        .expect("fully acked offset should persist");
    let empty_window = store
        .delivery_replay_window(DeliveryOffsetInput {
            client_id: client.id,
            session_id,
            pane_id,
            stream_id: None,
        })
        .expect("empty replay window should load");

    assert_eq!(fully_acked.replay_from_event_seq, None);
    assert_eq!(empty_window.from_event_seq, None);
    assert_eq!(empty_window.to_event_seq, 2);
}

#[test]
fn delivery_replay_window_surfaces_gap_state() {
    let store = test_store("delivery-gap");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store.release_writer_generation(&writer.id).expect("writer should release");
    store
        .record_history_gap_event(HistoryGapEventInput {
            session_id: session_id.clone(),
            route: route(),
            title: Some("shell".to_string()),
            launch: None,
            pane_id: pane_id.clone(),
            tab_id: None,
            rows: Some(24),
            cols: Some(80),
            skipped_events: 2,
            estimated_dropped_bytes: Some(64),
            reason: "test_delivery_gap".to_string(),
            occurred_at_ms: None,
        })
        .expect("history gap should persist");
    let client = store
        .upsert_delivery_client(DeliveryClientInput {
            id: Some("browser-gap".to_string()),
            client_kind: "browser".to_string(),
            install_ref_hash: None,
            browser_profile_ref_hash: None,
            user_agent_hash: None,
            trust_state: None,
        })
        .expect("client should persist");

    let window = store
        .delivery_replay_window(DeliveryOffsetInput {
            client_id: client.id,
            session_id,
            pane_id,
            stream_id: None,
        })
        .expect("replay window should load");

    assert_eq!(window.from_event_seq, Some(1));
    assert_eq!(window.to_event_seq, 2);
    assert_eq!(window.gap_state, "gap");
}

#[test]
fn stream_segment_enqueue_projection_outbox_message() {
    let store = test_store("outbox-stream");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let receipt = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"outbox\r\n".to_vec(),
        ))
        .expect("stream segment should persist");

    let message = store
        .claim_next_outbox_message("projection-worker", 60_000)
        .expect("claim should load")
        .expect("projection outbox message should exist");

    assert_eq!(message.message_kind, "pane_history_projection");
    assert_eq!(message.state, "claimed");
    assert_eq!(message.attempts, 1);
    assert_eq!(message.payload_json["session_id"], session_id);
    assert_eq!(message.payload_json["pane_id"], pane_id);
    assert_eq!(message.payload_json["commit_id"], receipt.commit_id);
}

#[test]
fn outbox_dedupes_claims_and_completes_by_lease_token() {
    let store = test_store("outbox-dedupe");
    let first = store
        .enqueue_outbox_message(OutboxMessageInput {
            message_kind: "restore_drill".to_string(),
            payload: serde_json::json!({ "session_id": "session-a" }),
            dedupe_key: Some("restore-drill:session-a".to_string()),
            max_attempts: None,
            next_run_at_ms: None,
        })
        .expect("first outbox message should enqueue");
    let second = store
        .enqueue_outbox_message(OutboxMessageInput {
            message_kind: "restore_drill".to_string(),
            payload: serde_json::json!({ "session_id": "session-a" }),
            dedupe_key: Some("restore-drill:session-a".to_string()),
            max_attempts: None,
            next_run_at_ms: None,
        })
        .expect("deduped outbox message should load");

    let claim = store
        .claim_next_outbox_message("worker-a", 60_000)
        .expect("claim should succeed")
        .expect("message should be claimable");
    let second_claim =
        store.claim_next_outbox_message("worker-b", 60_000).expect("second claim should not fail");
    let wrong_token_done = store
        .mark_outbox_message_done(&claim.id, "wrong-token")
        .expect("wrong token completion should be safe");
    let done = store
        .mark_outbox_message_done(
            &claim.id,
            claim.lease_token.as_deref().expect("claim should have a lease token"),
        )
        .expect("completion should succeed");
    let no_more = store
        .claim_next_outbox_message("worker-a", 60_000)
        .expect("done message should not be claimable");

    assert_eq!(first.id, second.id);
    assert_eq!(claim.id, first.id);
    assert!(second_claim.is_none());
    assert!(!wrong_token_done);
    assert!(done);
    assert!(no_more.is_none());
}

#[test]
fn outbox_quarantines_poison_message_after_max_attempts() {
    let store = test_store("outbox-quarantine");
    let message = store
        .enqueue_outbox_message(OutboxMessageInput {
            message_kind: "integrity_check".to_string(),
            payload: serde_json::json!({ "scope": "test" }),
            dedupe_key: None,
            max_attempts: Some(1),
            next_run_at_ms: None,
        })
        .expect("message should enqueue");
    let claim = store
        .claim_next_outbox_message("worker-a", 60_000)
        .expect("claim should succeed")
        .expect("message should be claimable");

    let failed = store
        .fail_outbox_message(
            &message.id,
            claim.lease_token.as_deref().expect("claim should have a lease token"),
            "synthetic failure",
        )
        .expect("failure should persist");
    let no_more = store
        .claim_next_outbox_message("worker-b", 60_000)
        .expect("quarantined message should not be claimable");

    assert_eq!(failed.state, "quarantined");
    assert_eq!(failed.last_error.as_deref(), Some("synthetic failure"));
    assert!(no_more.is_none());
}
