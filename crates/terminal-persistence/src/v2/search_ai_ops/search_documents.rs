use super::super::*;

impl TerminalPersistenceV2 {
    pub fn upsert_redacted_search_document(
        &self,
        input: SearchDocumentInput,
    ) -> Result<SearchDocumentRecord, TerminalPersistenceV2Error> {
        validate_optional_range(input.event_seq_low, input.event_seq_high, "search event")?;
        validate_optional_half_open_range(input.byte_low, input.byte_high, "search byte")?;

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let redacted = redact_terminal_text(&input.raw_text);
        let redaction_state =
            if redacted == input.raw_text { "clean".to_string() } else { "redacted".to_string() };
        let source_hash = blake3_hash_text(&input.raw_text);
        let document_id = input.document_id.unwrap_or_else(|| {
            stable_search_document_id(
                &input.session_id,
                input.pane_id.as_deref(),
                input.command_block_id.as_deref(),
                &source_hash,
            )
        });
        let row = NewSearchDocumentRow {
            document_id: document_id.clone(),
            session_id: input.session_id,
            pane_id: input.pane_id,
            command_block_id: input.command_block_id,
            document_kind: input.document_kind.unwrap_or_else(|| "redacted_snippet".to_string()),
            event_seq_low: input.event_seq_low,
            event_seq_high: input.event_seq_high,
            byte_low: input.byte_low,
            byte_high: input.byte_high,
            redaction_profile_id: input
                .redaction_profile_id
                .or_else(|| Some("default".to_string())),
            redaction_state,
            source_hash_algorithm: "blake3".to_string(),
            source_hash,
            text_preview: limit_text_preview(&redacted, 2_048),
            updated_at_ms: now,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_search_documents::table)
            .values(&row)
            .on_conflict(terminal_search_documents::document_id)
            .do_update()
            .set((
                terminal_search_documents::session_id.eq(row.session_id.clone()),
                terminal_search_documents::pane_id.eq(row.pane_id.clone()),
                terminal_search_documents::command_block_id.eq(row.command_block_id.clone()),
                terminal_search_documents::document_kind.eq(row.document_kind.clone()),
                terminal_search_documents::event_seq_low.eq(row.event_seq_low),
                terminal_search_documents::event_seq_high.eq(row.event_seq_high),
                terminal_search_documents::byte_low.eq(row.byte_low),
                terminal_search_documents::byte_high.eq(row.byte_high),
                terminal_search_documents::redaction_profile_id
                    .eq(row.redaction_profile_id.clone()),
                terminal_search_documents::redaction_state.eq(row.redaction_state.clone()),
                terminal_search_documents::source_hash_algorithm
                    .eq(row.source_hash_algorithm.clone()),
                terminal_search_documents::source_hash.eq(row.source_hash.clone()),
                terminal_search_documents::text_preview.eq(row.text_preview.clone()),
                terminal_search_documents::updated_at_ms.eq(row.updated_at_ms),
                terminal_search_documents::metadata_json.eq(row.metadata_json.clone()),
            ))
            .execute(&mut connection)?;

        terminal_search_documents::table
            .filter(terminal_search_documents::document_id.eq(document_id))
            .select(SearchDocumentRow::as_select())
            .first::<SearchDocumentRow>(&mut connection)?
            .try_into()
    }

    pub fn list_search_documents(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<SearchDocumentRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_search_documents::table
            .filter(terminal_search_documents::session_id.eq(session_id))
            .order(terminal_search_documents::updated_at_ms.desc())
            .limit(limit.max(1))
            .select(SearchDocumentRow::as_select())
            .load::<SearchDocumentRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
}
