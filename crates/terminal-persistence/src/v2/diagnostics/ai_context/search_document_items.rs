use super::super::super::*;
use super::super::*;

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

    let rows = load_search_documents_for_context(connection, session_id, pane_id, limit)?;
    let mut inserted = Vec::new();
    for document in rows {
        let context_item = search_document_context_item(package_id, document)?;
        insert_into(terminal_ai_context_items::table).values(&context_item).execute(connection)?;
        inserted.push(InsertedAiContextItem {
            id: context_item.id,
            content_preview: context_item.content_preview,
        });
    }
    Ok(inserted)
}

fn load_search_documents_for_context(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    limit: i64,
) -> Result<Vec<SearchDocumentRow>, TerminalPersistenceV2Error> {
    let mut query = terminal_search_documents::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query.filter(terminal_search_documents::session_id.eq(session_id.to_string()));
    }
    if let Some(pane_id) = pane_id {
        query = query.filter(terminal_search_documents::pane_id.eq(Some(pane_id.to_string())));
    }

    query
        .order(terminal_search_documents::updated_at_ms.desc())
        .limit(limit)
        .select(SearchDocumentRow::as_select())
        .load::<SearchDocumentRow>(connection)
        .map_err(Into::into)
}

fn search_document_context_item(
    package_id: &str,
    document: SearchDocumentRow,
) -> Result<NewAiContextItemRow, TerminalPersistenceV2Error> {
    Ok(NewAiContextItemRow {
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
        content_preview: limit_text_preview(&document.text_preview, 512),
        metadata_json: Some(serde_json::to_string(&serde_json::json!({
            "source": "search_document",
            "document_kind": document.document_kind,
            "raw_terminal_output_included": false,
            "source_hash_exported": false
        }))?),
    })
}
