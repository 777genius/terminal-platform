use super::super::super::super::*;
use super::context::UiInputTransaction;

pub(super) fn upsert_verified_command_history(
    connection: &mut SqliteConnection,
    tx: &UiInputTransaction<'_>,
    commit_id: &str,
    event_seq: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let Some(command_text) = tx.command_text.as_ref() else {
        return Ok(());
    };
    let command_block_id = tx
        .source_event_id_hash
        .as_ref()
        .map(|hash| stable_ui_command_block_id(&tx.input.session_id, &tx.input.pane_id, hash))
        .unwrap_or_else(new_id);
    insert_command_block(connection, tx, commit_id, event_seq, command_text, &command_block_id)?;
    upsert_command_history_entry(connection, tx, command_text, &command_block_id)
}

fn insert_command_block(
    connection: &mut SqliteConnection,
    tx: &UiInputTransaction<'_>,
    commit_id: &str,
    event_seq: i64,
    command_text: &str,
    command_block_id: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let block = NewCommandBlockRow {
        id: command_block_id.to_string(),
        session_id: tx.input.session_id.clone(),
        pane_id: tx.input.pane_id.clone(),
        commit_id: Some(commit_id.to_string()),
        command_text: Some(command_text.to_string()),
        display_text: Some(command_text.to_string()),
        redacted_text: None,
        command_text_source: "ui_submit".to_string(),
        trust_level: "verified".to_string(),
        state: "submitted".to_string(),
        cwd: None,
        cwd_source: None,
        exit_code: None,
        started_event_seq: Some(event_seq),
        submitted_event_seq: Some(event_seq),
        finished_event_seq: None,
        output_event_seq_low: None,
        output_event_seq_high: None,
        output_byte_low: None,
        output_byte_high: None,
        sensitivity_class: "unknown".to_string(),
        created_at_ms: tx.now,
        updated_at_ms: tx.now,
        metadata_json: tx.command_metadata_json.clone(),
    };
    insert_into(terminal_command_blocks::table)
        .values(&block)
        .on_conflict(terminal_command_blocks::id)
        .do_nothing()
        .execute(connection)?;
    Ok(())
}

fn upsert_command_history_entry(
    connection: &mut SqliteConnection,
    tx: &UiInputTransaction<'_>,
    command_text: &str,
    command_block_id: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let command_hash = local_keyed_command_hash(connection, command_text)?;
    let history = NewCommandHistoryEntryRow {
        id: stable_history_id(
            "session",
            Some(&tx.input.session_id),
            Some(&tx.input.pane_id),
            &command_hash,
        ),
        session_id: Some(tx.input.session_id.clone()),
        pane_id: Some(tx.input.pane_id.clone()),
        command_block_id: Some(command_block_id.to_string()),
        scope_kind: "session".to_string(),
        command_text: Some(command_text.to_string()),
        display_text: command_text.to_string(),
        redacted_text: None,
        command_hash_algorithm: COMMAND_HASH_ALGORITHM.to_string(),
        command_hash_scope: COMMAND_HASH_SCOPE.to_string(),
        command_hash,
        cwd: None,
        shell_kind: tx.shell_profile.shell_kind.clone(),
        trust_level: "verified".to_string(),
        source: "ui_submit".to_string(),
        sensitivity_class: "unknown".to_string(),
        redaction_state: "unscanned".to_string(),
        rerun_policy: "confirm".to_string(),
        first_used_at_ms: tx.now,
        last_used_at_ms: tx.now,
        use_count: 1,
        metadata_json: None,
    };
    insert_into(terminal_command_history_entries::table)
        .values(&history)
        .on_conflict(terminal_command_history_entries::id)
        .do_update()
        .set((
            terminal_command_history_entries::last_used_at_ms.eq(history.last_used_at_ms),
            terminal_command_history_entries::use_count
                .eq(terminal_command_history_entries::use_count + 1),
            terminal_command_history_entries::command_block_id.eq(history.command_block_id.clone()),
            terminal_command_history_entries::cwd.eq(history.cwd.clone()),
            terminal_command_history_entries::metadata_json.eq(history.metadata_json.clone()),
        ))
        .execute(connection)?;
    Ok(())
}
