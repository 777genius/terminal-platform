use super::super::*;
use super::*;

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
    let mut query = terminal_command_history_entries::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query
            .filter(terminal_command_history_entries::session_id.eq(Some(session_id.to_string())));
    }
    if let Some(pane_id) = pane_id {
        query =
            query.filter(terminal_command_history_entries::pane_id.eq(Some(pane_id.to_string())));
    }
    let rows = query
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
        .load::<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
            String,
        )>(connection)?;

    let mut inserted = Vec::new();
    for (
        source_id,
        session_id,
        pane_id,
        command_block_id,
        display_text,
        redacted_text,
        redaction_state,
        trust_level,
        rerun_policy,
    ) in rows
    {
        let preview_source = redacted_text.as_deref().unwrap_or(&display_text);
        let content_preview = limit_text_preview(&redact_terminal_text(preview_source), 512);
        let row = NewAiContextItemRow {
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
        };
        insert_into(terminal_ai_context_items::table).values(&row).execute(connection)?;
        inserted.push(InsertedAiContextItem { id: row.id, content_preview: row.content_preview });
    }
    Ok(inserted)
}

pub(in crate::v2) fn insert_ai_context_items_from_search_documents(
    connection: &mut SqliteConnection,
    package_id: &str,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    limit: i64,
) -> Result<Vec<InsertedAiContextItem>, TerminalPersistenceV2Error> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    let mut query = terminal_search_documents::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query.filter(terminal_search_documents::session_id.eq(session_id.to_string()));
    }
    if let Some(pane_id) = pane_id {
        query = query.filter(terminal_search_documents::pane_id.eq(Some(pane_id.to_string())));
    }
    let rows = query
        .order(terminal_search_documents::updated_at_ms.desc())
        .limit(limit)
        .select(SearchDocumentRow::as_select())
        .load::<SearchDocumentRow>(connection)?;

    let mut inserted = Vec::new();
    for document in rows {
        let content_preview = limit_text_preview(&document.text_preview, 512);
        let row = NewAiContextItemRow {
            id: new_id(),
            package_id: package_id.to_string(),
            source_kind: "search_document".to_string(),
            source_ref: Some(document.document_id),
            session_id: Some(document.session_id),
            pane_id: document.pane_id,
            command_block_id: document.command_block_id,
            event_seq_low: document.event_seq_low,
            event_seq_high: document.event_seq_high,
            byte_low: document.byte_low,
            byte_high: document.byte_high,
            redaction_state: document.redaction_state,
            data_only: 1,
            content_preview,
            metadata_json: Some(serde_json::to_string(&serde_json::json!({
                "source": "search_document",
                "document_kind": document.document_kind,
                "raw_terminal_output_included": false,
                "source_hash_exported": false
            }))?),
        };
        insert_into(terminal_ai_context_items::table).values(&row).execute(connection)?;
        inserted.push(InsertedAiContextItem { id: row.id, content_preview: row.content_preview });
    }
    Ok(inserted)
}

pub(in crate::v2) fn insert_prompt_injection_findings_for_items(
    connection: &mut SqliteConnection,
    package_id: &str,
    items: &[InsertedAiContextItem],
    now: i64,
) -> Result<i64, TerminalPersistenceV2Error> {
    let mut count = 0_i64;
    for item in items {
        if let Some(pattern_kind) = detect_prompt_injection_pattern(&item.content_preview) {
            let finding = NewPromptInjectionFindingRow {
                id: new_id(),
                package_id: Some(package_id.to_string()),
                item_id: Some(item.id.clone()),
                severity: "warning".to_string(),
                pattern_kind: pattern_kind.to_string(),
                action_state: "detected".to_string(),
                detected_at_ms: now,
                evidence_preview: limit_text_preview(&item.content_preview, 160),
                metadata_json: Some(serde_json::to_string(&serde_json::json!({
                    "terminal_output_is_data_only": true,
                    "auto_action_allowed": false
                }))?),
            };
            insert_into(terminal_prompt_injection_findings::table)
                .values(&finding)
                .execute(connection)?;
            count += 1;
        }
    }
    Ok(count)
}
