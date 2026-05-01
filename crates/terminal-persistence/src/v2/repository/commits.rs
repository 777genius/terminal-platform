use super::super::*;

pub(in crate::v2) fn allocate_commit(
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

pub(in crate::v2) fn ensure_active_writer(
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

pub(in crate::v2) fn insert_clock_anchor(
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

pub(in crate::v2) fn process_monotonic_ms() -> i64 {
    static PROCESS_START: OnceLock<Instant> = OnceLock::new();
    let elapsed_ms = PROCESS_START.get_or_init(Instant::now).elapsed().as_millis();
    elapsed_ms.min(i64::MAX as u128) as i64
}
