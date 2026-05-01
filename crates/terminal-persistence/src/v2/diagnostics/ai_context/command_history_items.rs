use super::super::super::*;
use super::super::*;

pub(in crate::v2) fn insert_ai_context_items_from_command_history(
    connection: &mut SqliteConnection,
    package_id: &str,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    limit: i64,
) -> Result<Vec<InsertedAiContextItem>, TerminalPersistenceV2Error> {
    if limit <= 0 {
        return Ok(Vec::new());
    }

    let rows = load_command_history_context_rows(connection, session_id, pane_id, limit)?;
    let mut inserted = Vec::new();
    for row in rows {
        let context_item = command_history_context_item(package_id, row)?;
        insert_into(terminal_ai_context_items::table).values(&context_item).execute(connection)?;
        inserted.push(InsertedAiContextItem {
            id: context_item.id,
            content_preview: context_item.content_preview,
        });
    }
    Ok(inserted)
}

type CommandHistoryContextRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    String,
    String,
    String,
);

fn load_command_history_context_rows(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    limit: i64,
) -> Result<Vec<CommandHistoryContextRow>, TerminalPersistenceV2Error> {
    let mut query = terminal_command_history_entries::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query
            .filter(terminal_command_history_entries::session_id.eq(Some(session_id.to_string())));
    }
    if let Some(pane_id) = pane_id {
        query =
            query.filter(terminal_command_history_entries::pane_id.eq(Some(pane_id.to_string())));
    }

    query
        .order(terminal_command_history_entries::last_used_at_ms.desc())
        .limit(limit)
        .select((
            terminal_command_history_entries::id,
            terminal_command_history_entries::session_id,
            terminal_command_history_entries::pane_id,
            terminal_command_history_entries::command_block_id,
            terminal_command_history_entries::display_text,
            terminal_command_history_entries::redacted_text,
            terminal_command_history_entries::redaction_state,
            terminal_command_history_entries::trust_level,
            terminal_command_history_entries::rerun_policy,
        ))
        .load::<CommandHistoryContextRow>(connection)
        .map_err(Into::into)
}

fn command_history_context_item(
    package_id: &str,
    row: CommandHistoryContextRow,
) -> Result<NewAiContextItemRow, TerminalPersistenceV2Error> {
    let (
        source_id,
        session_id,
        pane_id,
        command_block_id,
        display_text,
        redacted_text,
        redaction_state,
        trust_level,
        rerun_policy,
    ) = row;
    let preview_source = redacted_text.as_deref().unwrap_or(&display_text);
    let content_preview = limit_text_preview(&redact_terminal_text(preview_source), 512);

    Ok(NewAiContextItemRow {
        id: new_id(),
        package_id: package_id.to_string(),
        source_kind: "command_history".to_string(),
        source_ref: Some(source_id),
        session_id,
        pane_id,
        command_block_id,
        event_seq_low: None,
        event_seq_high: None,
        byte_low: None,
        byte_high: None,
        redaction_state,
        data_only: 1,
        content_preview,
        metadata_json: Some(serde_json::to_string(&serde_json::json!({
            "source": "command_history",
            "trust_level": trust_level,
            "rerun_policy": rerun_policy,
            "raw_command_text_included": false,
            "command_hash_exported": false
        }))?),
    })
}
