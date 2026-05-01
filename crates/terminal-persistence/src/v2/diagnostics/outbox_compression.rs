use super::super::*;

pub(in crate::v2) fn collect_outbox_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<OutboxDiagnosticsRecord, TerminalPersistenceV2Error> {
    let pending_count = count_outbox_state(connection, "pending")?;
    let due_pending_count = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.le(now))
        .count()
        .get_result::<i64>(connection)?;
    let claimed_count = count_outbox_state(connection, "claimed")?;
    let stale_claim_count = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("claimed"))
        .filter(terminal_outbox_messages::claimed_until_ms.le(Some(now)))
        .count()
        .get_result::<i64>(connection)?;
    let done_count = count_outbox_state(connection, "done")?;
    let failed_count = count_outbox_state(connection, "failed")?;
    let quarantined_count = count_outbox_state(connection, "quarantined")?;
    let oldest_due_pending_created_at = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.le(now))
        .select(min(terminal_outbox_messages::created_at_ms))
        .first::<Option<i64>>(connection)?;
    let next_pending_run_at = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.gt(now))
        .select(min(terminal_outbox_messages::next_run_at_ms))
        .first::<Option<i64>>(connection)?;

    Ok(OutboxDiagnosticsRecord {
        generated_at_ms: now,
        pending_count,
        due_pending_count,
        claimed_count,
        stale_claim_count,
        done_count,
        failed_count,
        quarantined_count,
        oldest_due_pending_age_ms: oldest_due_pending_created_at
            .map(|created_at_ms| (now - created_at_ms).max(0)),
        next_pending_due_in_ms: next_pending_run_at.map(|run_at_ms| (run_at_ms - now).max(0)),
    })
}

pub(in crate::v2) fn count_outbox_state(
    connection: &mut SqliteConnection,
    state_name: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq(state_name))
        .count()
        .get_result::<i64>(connection)?)
}

pub(in crate::v2) fn collect_compression_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<CompressionDiagnosticsRecord, TerminalPersistenceV2Error> {
    let feature_gate_state = terminal_feature_gates::table
        .filter(
            terminal_feature_gates::feature_name
                .eq(FeatureGateName::SegmentCompressionZstd.as_str()),
        )
        .select(terminal_feature_gates::state)
        .first::<String>(connection)
        .optional()?
        .unwrap_or_else(|| FeatureGateState::Disabled.as_str().to_string());
    let raw_segment_count = count_stream_segments_by_compression(connection, "none")?;
    let zstd_segment_count = count_stream_segments_by_compression(connection, "zstd")?;
    let unsupported_segment_count = terminal_stream_segments::table
        .filter(terminal_stream_segments::compression.ne("none"))
        .filter(terminal_stream_segments::compression.ne("zstd"))
        .count()
        .get_result::<i64>(connection)?;
    let rewrite_candidate_count = if feature_gate_state == FeatureGateState::Enabled.as_str() {
        raw_segment_count
    } else {
        0
    };
    let action_taken = if feature_gate_state == FeatureGateState::Enabled.as_str() {
        "skipped_restore_drill_guard"
    } else {
        "skipped_feature_disabled"
    }
    .to_string();

    Ok(CompressionDiagnosticsRecord {
        generated_at_ms: now,
        feature_gate_state,
        raw_segment_count,
        zstd_segment_count,
        unsupported_segment_count,
        rewrite_candidate_count,
        segments_rewritten: 0,
        restore_drill_required: true,
        action_taken,
    })
}

pub(in crate::v2) fn count_stream_segments_by_compression(
    connection: &mut SqliteConnection,
    compression: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_stream_segments::table
        .filter(terminal_stream_segments::compression.eq(compression))
        .count()
        .get_result::<i64>(connection)?)
}

#[derive(Debug, Clone)]
pub(in crate::v2) struct InsertedAiContextItem {
    pub(in crate::v2) id: String,
    pub(in crate::v2) content_preview: String,
}
