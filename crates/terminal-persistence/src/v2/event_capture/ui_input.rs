use super::super::*;

impl TerminalPersistenceV2 {
    pub fn record_ui_input_event(
        &self,
        input: UiInputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        self.upsert_runtime_session(SessionInput {
            id: Some(input.session_id.clone()),
            route: input.route.clone(),
            title: input.title.clone(),
            launch: input.launch.clone(),
            source: Some("runtime_ui_input".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: Some(serde_json::json!({
                "capture_source": "ui_input",
                "trusted_command_source": true
            })),
        })?;
        self.upsert_runtime_pane(PaneInput {
            id: Some(input.pane_id.clone()),
            session_id: input.session_id.clone(),
            tab_id: None,
            stream_id: None,
            title: None,
            rows: input.rows.unwrap_or(24),
            cols: input.cols.unwrap_or(80),
            metadata: Some(serde_json::json!({
                "capture_source": "ui_input",
                "dimensions": if input.rows.is_some() && input.cols.is_some() {
                    "observed"
                } else {
                    "provisional"
                }
            })),
        })?;
        if self.is_session_private(&input.session_id)? {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "private mode suppresses durable ui input history".to_string(),
            ));
        }

        let lease = self.acquire_writer_generation_with_retry("runtime-ui-input", 60_000)?;
        let event_result = self.append_ui_input_event_and_command(&input, &lease.id);
        let release_result = self.release_writer_generation(&lease.id);

        match (event_result, release_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    pub(in crate::v2) fn append_ui_input_event_and_command(
        &self,
        input: &UiInputEventInput,
        writer_generation: &str,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let stream_id = DEFAULT_STREAM_ID.to_string();
        let event_type = if input.is_paste { "terminal_paste_input" } else { "terminal_input" };
        let payload_json = serde_json::json!({
            "data": input.data.clone(),
            "is_paste": input.is_paste
        });
        let payload_json = serde_json::to_string(&payload_json)?;
        let payload_hash = blake3_hash_text(&payload_json);
        let source_event_id_hash = input.source_event_id.as_ref().map(|source_event_id| {
            blake3_hash_text(&format!("ui-input-client-event:{source_event_id}"))
        });
        let capture_source_kind =
            source_event_id_hash.as_ref().map(|_| ui_input_capture_source_kind(&input.pane_id));
        let command_text = command_text_from_ui_input(&input.data);
        let shell_profile =
            shell_metadata_profile(input.launch.as_ref(), input.shell_kind.as_deref());
        let command_metadata_json = Some(serde_json::to_string(&serde_json::json!({
            "capture_source": "ui_input",
            "rerun_policy": "confirm",
            "shell_profile": shell_profile
        }))?);

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            if let (Some(source_kind), Some(source_event_id_hash)) =
                (capture_source_kind.as_deref(), source_event_id_hash.as_deref())
            {
                if let Some(receipt) = load_capture_receipt(
                    connection,
                    &input.session_id,
                    source_kind,
                    source_event_id_hash,
                )? {
                    if receipt.source_payload_hash != payload_hash {
                        return Err(TerminalPersistenceV2Error::InvalidData(format!(
                            "ui input receipt payload hash mismatch for source_kind={source_kind}"
                        )));
                    }
                    return Ok(());
                }
            }

            ensure_active_writer(connection, writer_generation, now)?;
            let commit = allocate_commit(
                connection,
                &input.session_id,
                "ui_input",
                writer_generation,
                now,
                now,
                None,
            )?;
            let cursor =
                load_stream_cursor(connection, &input.session_id, &input.pane_id, &stream_id)?;
            let event_seq = cursor.next_event_seq;
            let scope = event_scope(&input.session_id, Some(&input.pane_id));
            let event_id = new_id();
            let event = NewJournalEventRow {
                id: event_id,
                session_id: input.session_id.clone(),
                pane_id: Some(input.pane_id.clone()),
                commit_id: commit.id.clone(),
                stream_id: stream_id.clone(),
                event_scope_kind: scope.kind,
                event_scope_id: scope.id,
                event_seq,
                event_type: event_type.to_string(),
                byte_low: None,
                byte_high: None,
                payload_json: Some(payload_json.clone()),
                payload_schema_id: Some(PAYLOAD_SCHEMA_UI_INPUT_V1.to_string()),
                source_event_id_hash: source_event_id_hash.clone(),
                occurred_at_ms: now,
                created_at_ms: now,
                capture_semantics: "ui_input".to_string(),
                trust_level: "verified".to_string(),
                metadata_json: None,
            };
            insert_into(terminal_journal_events::table).values(&event).execute(connection)?;

            advance_stream_cursor(
                connection,
                &cursor.id,
                cursor.next_event_seq + 1,
                cursor.next_byte_seq,
                now,
            )?;
            diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(&input.pane_id)))
                .set(terminal_panes::last_event_seq.eq(event_seq))
                .execute(connection)?;

            if let Some(command_text) = command_text.as_ref() {
                let command_block_id = source_event_id_hash
                    .as_ref()
                    .map(|hash| stable_ui_command_block_id(&input.session_id, &input.pane_id, hash))
                    .unwrap_or_else(new_id);
                let block = NewCommandBlockRow {
                    id: command_block_id.clone(),
                    session_id: input.session_id.clone(),
                    pane_id: input.pane_id.clone(),
                    commit_id: Some(commit.id.clone()),
                    command_text: Some(command_text.clone()),
                    display_text: Some(command_text.clone()),
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
                    created_at_ms: now,
                    updated_at_ms: now,
                    metadata_json: command_metadata_json.clone(),
                };
                insert_into(terminal_command_blocks::table)
                    .values(&block)
                    .on_conflict(terminal_command_blocks::id)
                    .do_nothing()
                    .execute(connection)?;

                let command_hash = local_keyed_command_hash(connection, command_text)?;
                let history_id = stable_history_id(
                    "session",
                    Some(&input.session_id),
                    Some(&input.pane_id),
                    &command_hash,
                );
                let history = NewCommandHistoryEntryRow {
                    id: history_id,
                    session_id: Some(input.session_id.clone()),
                    pane_id: Some(input.pane_id.clone()),
                    command_block_id: Some(command_block_id),
                    scope_kind: "session".to_string(),
                    command_text: Some(command_text.clone()),
                    display_text: command_text.clone(),
                    redacted_text: None,
                    command_hash_algorithm: COMMAND_HASH_ALGORITHM.to_string(),
                    command_hash_scope: COMMAND_HASH_SCOPE.to_string(),
                    command_hash,
                    cwd: None,
                    shell_kind: shell_profile.shell_kind.clone(),
                    trust_level: "verified".to_string(),
                    source: "ui_submit".to_string(),
                    sensitivity_class: "unknown".to_string(),
                    redaction_state: "unscanned".to_string(),
                    rerun_policy: "confirm".to_string(),
                    first_used_at_ms: now,
                    last_used_at_ms: now,
                    use_count: 1,
                    metadata_json: None,
                };
                insert_into(terminal_command_history_entries::table)
                    .values(&history)
                    .on_conflict(terminal_command_history_entries::id)
                    .do_update()
                    .set((
                        terminal_command_history_entries::last_used_at_ms
                            .eq(history.last_used_at_ms),
                        terminal_command_history_entries::use_count
                            .eq(terminal_command_history_entries::use_count + 1),
                        terminal_command_history_entries::command_block_id
                            .eq(history.command_block_id.clone()),
                        terminal_command_history_entries::cwd.eq(history.cwd.clone()),
                        terminal_command_history_entries::metadata_json
                            .eq(history.metadata_json.clone()),
                    ))
                    .execute(connection)?;
            }

            if let (Some(source_kind), Some(source_event_id_hash)) =
                (capture_source_kind.as_deref(), source_event_id_hash.as_deref())
            {
                let receipt = NewCaptureReceiptRow {
                    id: new_id(),
                    session_id: input.session_id.clone(),
                    commit_id: Some(commit.id),
                    source_kind: source_kind.to_string(),
                    source_event_id_hash: source_event_id_hash.to_string(),
                    source_payload_hash: payload_hash.clone(),
                    received_at_ms: now,
                    created_at_ms: now,
                    metadata_json: None,
                };
                insert_into(terminal_capture_receipts::table)
                    .values(&receipt)
                    .execute(connection)?;
            }

            Ok(())
        })
    }
}
