use super::*;

pub(super) fn allocate_commit(
    connection: &mut SqliteConnection,
    session_id: &str,
    commit_kind: &str,
    writer_generation: &str,
    occurred_at_ms: i64,
    created_at_ms: i64,
    metadata_json: Option<String>,
) -> Result<CommitAllocation, TerminalPersistenceV2Error> {
    let cursor = terminal_session_cursors::table
        .filter(terminal_session_cursors::session_id.eq(session_id))
        .select(SessionCursorRow::as_select())
        .first::<SessionCursorRow>(connection)?;
    let commit = CommitAllocation { id: new_id(), commit_seq: cursor.next_commit_seq };
    let row = NewCommitLogRow {
        id: commit.id.clone(),
        session_id: session_id.to_string(),
        commit_seq: commit.commit_seq,
        commit_kind: commit_kind.to_string(),
        writer_generation: writer_generation.to_string(),
        occurred_at_ms,
        created_at_ms,
        metadata_json,
    };

    insert_into(terminal_commit_log::table).values(&row).execute(connection)?;
    diesel::update(
        terminal_session_cursors::table.filter(terminal_session_cursors::session_id.eq(session_id)),
    )
    .set((
        terminal_session_cursors::next_commit_seq.eq(commit.commit_seq + 1),
        terminal_session_cursors::writer_generation.eq(Some(writer_generation.to_string())),
        terminal_session_cursors::updated_at_ms.eq(created_at_ms),
    ))
    .execute(connection)?;

    Ok(commit)
}

pub(super) fn load_stream_cursor(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<StreamCursorRow, TerminalPersistenceV2Error> {
    terminal_stream_cursors::table
        .filter(terminal_stream_cursors::session_id.eq(session_id))
        .filter(terminal_stream_cursors::pane_id.eq(pane_id))
        .filter(terminal_stream_cursors::stream_id.eq(stream_id))
        .select(StreamCursorRow::as_select())
        .first::<StreamCursorRow>(connection)
        .map_err(Into::into)
}

pub(super) fn load_capture_receipt(
    connection: &mut SqliteConnection,
    session_id: &str,
    source_kind: &str,
    source_event_id_hash: &str,
) -> Result<Option<CaptureReceiptRow>, TerminalPersistenceV2Error> {
    terminal_capture_receipts::table
        .filter(terminal_capture_receipts::session_id.eq(session_id))
        .filter(terminal_capture_receipts::source_kind.eq(source_kind))
        .filter(terminal_capture_receipts::source_event_id_hash.eq(source_event_id_hash))
        .select(CaptureReceiptRow::as_select())
        .first::<CaptureReceiptRow>(connection)
        .optional()
        .map_err(Into::into)
}

pub(super) fn stream_segment_receipt_from_capture_receipt(
    connection: &mut SqliteConnection,
    receipt: &CaptureReceiptRow,
) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
    let commit_id = receipt.commit_id.as_deref().ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData(format!(
            "stream capture receipt {} does not point to a commit",
            receipt.id
        ))
    })?;
    stream_segment_receipt_from_commit(connection, commit_id)
}

pub(super) fn stream_segment_receipt_from_commit(
    connection: &mut SqliteConnection,
    commit_ref: &str,
) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
    let segment = terminal_stream_segments::table
        .filter(terminal_stream_segments::commit_id.eq(commit_ref))
        .select(StreamSegmentRow::as_select())
        .first::<StreamSegmentRow>(connection)?;
    let event_id = terminal_journal_events::table
        .filter(terminal_journal_events::commit_id.eq(commit_ref))
        .select(terminal_journal_events::id)
        .first::<String>(connection)?;
    let commit_seq = terminal_commit_log::table
        .filter(terminal_commit_log::id.eq(commit_ref))
        .select(terminal_commit_log::commit_seq)
        .first::<i64>(connection)?;

    Ok(StreamSegmentReceipt {
        commit_id: commit_ref.to_string(),
        commit_seq,
        segment_id: segment.id,
        event_id,
        event_seq_low: segment.event_seq_low,
        event_seq_high: segment.event_seq_high,
        byte_low: segment.byte_low,
        byte_high: segment.byte_high,
        checksum: segment.checksum,
    })
}

pub(super) fn touch_delivery_client(
    connection: &mut SqliteConnection,
    client_id: &str,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let updated =
        diesel::update(terminal_clients::table.filter(terminal_clients::id.eq(client_id)))
            .set(terminal_clients::last_seen_at_ms.eq(now))
            .execute(connection)?;
    if updated == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "delivery client not found: {client_id}"
        )));
    }
    Ok(())
}

pub(super) fn load_delivery_offset(
    connection: &mut SqliteConnection,
    client_id: &str,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<Option<DeliveryOffsetRow>, TerminalPersistenceV2Error> {
    terminal_delivery_offsets::table
        .filter(terminal_delivery_offsets::client_id.eq(client_id))
        .filter(terminal_delivery_offsets::session_id.eq(session_id))
        .filter(terminal_delivery_offsets::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_delivery_offsets::stream_id.eq(stream_id))
        .select(DeliveryOffsetRow::as_select())
        .first::<DeliveryOffsetRow>(connection)
        .optional()
        .map_err(Into::into)
}

pub(super) fn load_persisted_event_high_water(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    let segment_high = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .filter(terminal_stream_segments::stream_id.eq(stream_id))
        .select(max(terminal_stream_segments::event_seq_high))
        .first::<Option<i64>>(connection)?
        .unwrap_or(0);
    let gap_high = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_history_gaps::stream_id.eq(stream_id))
        .filter(terminal_history_gaps::event_seq_high.is_not_null())
        .select(max(terminal_history_gaps::event_seq_high))
        .first::<Option<i64>>(connection)?
        .unwrap_or(0);

    Ok(segment_high.max(gap_high))
}

pub(super) fn has_history_gap_in_range(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
    from_event_seq: i64,
    to_event_seq: i64,
) -> Result<bool, TerminalPersistenceV2Error> {
    if from_event_seq > to_event_seq {
        return Ok(false);
    }
    let count = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_history_gaps::stream_id.eq(stream_id))
        .filter(terminal_history_gaps::event_seq_low.le(Some(to_event_seq)))
        .filter(terminal_history_gaps::event_seq_high.ge(Some(from_event_seq)))
        .count()
        .get_result::<i64>(connection)?;
    Ok(count > 0)
}

pub(super) fn session_private_mode(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<bool, TerminalPersistenceV2Error> {
    terminal_sessions::table
        .filter(terminal_sessions::id.eq(session_id))
        .select(terminal_sessions::private_mode)
        .first::<i32>(connection)
        .optional()
        .map(|value| value.unwrap_or(0) != 0)
        .map_err(Into::into)
}

pub(super) fn latest_backend_capability_report(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<Option<BackendCapabilityReportRow>, TerminalPersistenceV2Error> {
    terminal_backend_capability_reports::table
        .filter(terminal_backend_capability_reports::session_id.eq(Some(session_id.to_string())))
        .order(terminal_backend_capability_reports::created_at_ms.desc())
        .select(BackendCapabilityReportRow::as_select())
        .first::<BackendCapabilityReportRow>(connection)
        .optional()
        .map_err(Into::into)
}

pub(super) fn load_outbox_message(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<OutboxMessageRow, TerminalPersistenceV2Error> {
    terminal_outbox_messages::table
        .filter(terminal_outbox_messages::id.eq(id))
        .select(OutboxMessageRow::as_select())
        .first::<OutboxMessageRow>(connection)
        .map_err(Into::into)
}

pub(super) fn load_outbox_message_by_dedupe(
    connection: &mut SqliteConnection,
    dedupe_key: &str,
) -> Result<Option<OutboxMessageRow>, TerminalPersistenceV2Error> {
    terminal_outbox_messages::table
        .filter(terminal_outbox_messages::dedupe_key.eq(Some(dedupe_key.to_string())))
        .select(OutboxMessageRow::as_select())
        .first::<OutboxMessageRow>(connection)
        .optional()
        .map_err(Into::into)
}

pub(super) fn load_maintenance_run(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<MaintenanceRunRow, TerminalPersistenceV2Error> {
    terminal_maintenance_runs::table
        .filter(terminal_maintenance_runs::id.eq(id))
        .select(MaintenanceRunRow::as_select())
        .first::<MaintenanceRunRow>(connection)
        .map_err(Into::into)
}

pub(super) fn load_export_request(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<ExportRequestRow, TerminalPersistenceV2Error> {
    terminal_export_requests::table
        .filter(terminal_export_requests::id.eq(id))
        .select(ExportRequestRow::as_select())
        .first::<ExportRequestRow>(connection)
        .map_err(Into::into)
}

pub(super) fn load_support_bundle(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<SupportBundleRow, TerminalPersistenceV2Error> {
    terminal_support_bundles::table
        .filter(terminal_support_bundles::id.eq(id))
        .select(SupportBundleRow::as_select())
        .first::<SupportBundleRow>(connection)
        .map_err(Into::into)
}

pub(super) fn load_ai_context_package(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<AiContextPackageRow, TerminalPersistenceV2Error> {
    terminal_ai_context_packages::table
        .filter(terminal_ai_context_packages::id.eq(id))
        .select(AiContextPackageRow::as_select())
        .first::<AiContextPackageRow>(connection)
        .map_err(Into::into)
}

pub(super) fn load_ai_action_approval(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<AiActionApprovalRow, TerminalPersistenceV2Error> {
    terminal_ai_action_approvals::table
        .filter(terminal_ai_action_approvals::id.eq(id))
        .select(AiActionApprovalRow::as_select())
        .first::<AiActionApprovalRow>(connection)
        .map_err(Into::into)
}

pub(super) fn advance_stream_cursor(
    connection: &mut SqliteConnection,
    cursor_id: &str,
    next_event_seq: i64,
    next_byte_seq: i64,
    updated_at_ms: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    diesel::update(
        terminal_stream_cursors::table.filter(terminal_stream_cursors::id.eq(cursor_id)),
    )
    .set((
        terminal_stream_cursors::next_event_seq.eq(next_event_seq),
        terminal_stream_cursors::next_byte_seq.eq(next_byte_seq),
        terminal_stream_cursors::updated_at_ms.eq(updated_at_ms),
    ))
    .execute(connection)?;
    Ok(())
}

pub(super) fn ensure_active_writer(
    connection: &mut SqliteConnection,
    writer_generation: &str,
    now_ms: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let row = terminal_writer_generations::table
        .filter(terminal_writer_generations::id.eq(writer_generation))
        .filter(terminal_writer_generations::state.eq("active"))
        .select(WriterGenerationRow::as_select())
        .first::<WriterGenerationRow>(connection)?;
    if row.lease_expires_at_ms < now_ms {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "writer generation lease expired".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn insert_clock_anchor(
    connection: &mut SqliteConnection,
    writer_generation: &str,
    wall_time_ms: i64,
    source: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let row = NewClockAnchorRow {
        id: new_id(),
        writer_generation: writer_generation.to_string(),
        wall_time_ms,
        monotonic_ms: process_monotonic_ms(),
        source: source.to_string(),
        created_at_ms: wall_time_ms,
    };
    insert_into(terminal_clock_anchors::table).values(&row).execute(connection)?;
    Ok(())
}

pub(super) fn process_monotonic_ms() -> i64 {
    static PROCESS_START: OnceLock<Instant> = OnceLock::new();
    let elapsed_ms = PROCESS_START.get_or_init(Instant::now).elapsed().as_millis();
    elapsed_ms.min(i64::MAX as u128) as i64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MaintenanceRecoverySummary {
    pub(super) stale_outbox_claims_requeued: usize,
    pub(super) stale_outbox_claims_quarantined: usize,
    pub(super) stale_writer_generations_marked: usize,
}
