use super::*;

impl TerminalPersistenceV2 {
    pub fn import_saved_native_session_snapshot(
        &self,
        saved: &SavedNativeSession,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let lease = self.acquire_writer_generation_with_retry("legacy-save-session", 60_000)?;
        let import_result = self.import_saved_native_session_snapshot_with_writer(saved, &lease.id);
        let release_result = self.release_writer_generation(&lease.id);

        match (import_result, release_result) {
            (Ok(()), Ok(())) => self.restore_plan(&saved.session_id.0.to_string()),
            (Ok(()), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    fn import_saved_native_session_snapshot_with_writer(
        &self,
        saved: &SavedNativeSession,
        writer_generation: &str,
    ) -> Result<(), TerminalPersistenceV2Error> {
        self.upsert_legacy_visual_session(saved)?;
        for screen in &saved.screens {
            self.upsert_legacy_visual_pane(saved, screen)?;
            self.write_screen_snapshot(ScreenSnapshotInput {
                id: None,
                session_id: saved.session_id.0.to_string(),
                pane_id: screen.pane_id.0.to_string(),
                writer_generation: writer_generation.to_string(),
                projection_source: Some(format!("{:?}", screen.source).to_lowercase()),
                buffer_kind: Some("normal".to_string()),
                rows: i32::from(screen.rows),
                cols: i32::from(screen.cols),
                base_event_seq: 0,
                high_water_event_seq: u64_to_i64(screen.sequence, "screen sequence")?,
                high_water_byte_seq: None,
                screen: serde_json::to_value(screen)?,
                parser_version: Some("legacy_saved_screen_snapshot_v1".to_string()),
                projection_version: Some("legacy_visual_snapshot_v1".to_string()),
                metadata: Some(serde_json::json!({
                    "source": "legacy_save_session",
                    "saved_at_ms": saved.saved_at_ms
                })),
            })?;
        }

        self.write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: saved.session_id.0.to_string(),
            writer_generation: writer_generation.to_string(),
            pane_high_water: legacy_pane_high_water(saved),
            topology: serde_json::to_value(&saved.topology)?,
            source: Some("legacy_save_session".to_string()),
            metadata: Some(serde_json::json!({
                "visual_restore_only": true,
                "saved_at_ms": saved.saved_at_ms
            })),
        })?;

        Ok(())
    }

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

    pub(super) fn append_ui_input_event_and_command(
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

    pub fn record_terminal_output_event(
        &self,
        input: TerminalOutputEventInput,
    ) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
        self.upsert_runtime_session(SessionInput {
            id: Some(input.session_id.clone()),
            route: input.route,
            title: input.title,
            launch: input.launch,
            source: Some("runtime_output_capture".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: Some(serde_json::json!({
                "capture_source": "backend_output",
                "capture_semantics": input.capture_semantics
                    .as_deref()
                    .unwrap_or("raw_vt_stream")
            })),
        })?;
        self.upsert_runtime_pane(PaneInput {
            id: Some(input.pane_id.clone()),
            session_id: input.session_id.clone(),
            tab_id: input.tab_id.clone(),
            stream_id: None,
            title: None,
            rows: input.rows.unwrap_or(24),
            cols: input.cols.unwrap_or(80),
            metadata: Some(serde_json::json!({
                "capture_source": "backend_output",
                "dimensions": if input.rows.is_some() && input.cols.is_some() {
                    "observed"
                } else {
                    "provisional"
                }
            })),
        })?;
        if self.is_session_private(&input.session_id)? {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "private mode suppresses durable terminal output capture".to_string(),
            ));
        }

        let lease = self.acquire_writer_generation_with_retry("runtime-output-capture", 60_000)?;
        let append_result = self.append_stream_segment(StreamSegmentInput {
            session_id: input.session_id,
            pane_id: input.pane_id,
            stream_id: None,
            writer_generation: lease.id.clone(),
            payload: input.payload,
            event_type: Some("terminal_output".to_string()),
            event_count: 1,
            occurred_at_ms: input.occurred_at_ms,
            capture_semantics: input.capture_semantics,
            trust_level: Some("captured".to_string()),
            payload_json: None,
            source_event_id_hash: input
                .source_sequence
                .map(|sequence| blake3_hash_text(&format!("raw-output-seq:{sequence}"))),
            metadata: Some(serde_json::json!({
                "backend_source": "runtime_capture",
                "source_sequence": input.source_sequence
            })),
        });
        let release_result = self.release_writer_generation(&lease.id);

        match (append_result, release_result) {
            (Ok(receipt), Ok(())) => Ok(receipt),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    pub fn record_history_gap_event(
        &self,
        input: HistoryGapEventInput,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        let session_id = input.session_id.clone();
        let pane_id = input.pane_id.clone();
        let reason = input.reason.clone();
        let skipped_events = input.skipped_events;
        let estimated_dropped_bytes = input.estimated_dropped_bytes;
        let occurred_at_ms = input.occurred_at_ms;
        self.upsert_runtime_session(SessionInput {
            id: Some(input.session_id.clone()),
            route: input.route,
            title: input.title,
            launch: input.launch,
            source: Some("runtime_output_capture".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: Some(serde_json::json!({ "capture_source": "backend_output_gap" })),
        })?;
        self.upsert_runtime_pane(PaneInput {
            id: Some(input.pane_id.clone()),
            session_id: input.session_id.clone(),
            tab_id: input.tab_id.clone(),
            stream_id: None,
            title: None,
            rows: input.rows.unwrap_or(24),
            cols: input.cols.unwrap_or(80),
            metadata: Some(serde_json::json!({ "capture_source": "backend_output_gap" })),
        })?;

        let lease = self.acquire_writer_generation_with_retry("runtime-output-gap", 60_000)?;
        let append_result = self.append_history_gap_event(
            &session_id,
            &pane_id,
            &lease.id,
            skipped_events,
            estimated_dropped_bytes,
            &reason,
            occurred_at_ms,
        );
        let release_result = self.release_writer_generation(&lease.id);

        match (append_result, release_result) {
            (Ok(receipt), Ok(())) => Ok(receipt),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    pub(super) fn append_history_gap_event(
        &self,
        session_id: &str,
        pane_id: &str,
        writer_generation: &str,
        skipped_events: u64,
        estimated_dropped_bytes: Option<i64>,
        reason: &str,
        occurred_at_ms: Option<i64>,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let occurred_at_ms = occurred_at_ms.unwrap_or(now);
        let stream_id = DEFAULT_STREAM_ID.to_string();
        let gap_width = u64_to_i64(skipped_events.max(1), "history gap skipped events")?;
        let estimated_dropped_bytes = estimated_dropped_bytes.map(|value| value.max(0));
        let payload_json = serde_json::to_string(&serde_json::json!({
            "reason": reason,
            "skipped_events": skipped_events,
            "estimated_dropped_bytes": estimated_dropped_bytes
        }))?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            ensure_active_writer(connection, writer_generation, now)?;
            let commit = allocate_commit(
                connection,
                session_id,
                "history_gap",
                writer_generation,
                occurred_at_ms,
                now,
                None,
            )?;
            let cursor = load_stream_cursor(connection, session_id, pane_id, &stream_id)?;
            let event_seq_low = cursor.next_event_seq;
            let event_seq_high = event_seq_low + gap_width - 1;
            let scope = event_scope(session_id, Some(pane_id));
            let event_id = new_id();
            let event = NewJournalEventRow {
                id: event_id.clone(),
                session_id: session_id.to_string(),
                pane_id: Some(pane_id.to_string()),
                commit_id: commit.id.clone(),
                stream_id: stream_id.clone(),
                event_scope_kind: scope.kind,
                event_scope_id: scope.id,
                event_seq: event_seq_low,
                event_type: "history_gap".to_string(),
                byte_low: None,
                byte_high: None,
                payload_json: Some(payload_json),
                payload_schema_id: Some(PAYLOAD_SCHEMA_HISTORY_GAP_V1.to_string()),
                source_event_id_hash: None,
                occurred_at_ms,
                created_at_ms: now,
                capture_semantics: "raw_vt_stream".to_string(),
                trust_level: "system".to_string(),
                metadata_json: None,
            };
            insert_into(terminal_journal_events::table).values(&event).execute(connection)?;

            let gap = NewHistoryGapRow {
                id: new_id(),
                session_id: session_id.to_string(),
                pane_id: Some(pane_id.to_string()),
                stream_id: stream_id.clone(),
                gap_kind: "capture_gap".to_string(),
                event_seq_low: Some(event_seq_low),
                event_seq_high: Some(event_seq_high),
                byte_low: None,
                byte_high: None,
                estimated_dropped_bytes,
                estimated_dropped_events: Some(gap_width),
                reason: reason.to_string(),
                writer_generation: Some(writer_generation.to_string()),
                opened_at_ms: occurred_at_ms,
                closed_at_ms: Some(occurred_at_ms),
                metadata_json: None,
            };
            insert_into(terminal_history_gaps::table).values(&gap).execute(connection)?;

            advance_stream_cursor(
                connection,
                &cursor.id,
                event_seq_high + 1,
                cursor.next_byte_seq,
                now,
            )?;
            diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(pane_id)))
                .set(terminal_panes::last_event_seq.eq(event_seq_high))
                .execute(connection)?;

            Ok(JournalEventReceipt {
                commit_id: commit.id,
                commit_seq: commit.commit_seq,
                event_id,
                event_seq: event_seq_low,
            })
        })
    }

    pub fn record_screen_snapshot_event(
        &self,
        input: ScreenSnapshotEventInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        self.upsert_runtime_session(SessionInput {
            id: Some(input.session_id.clone()),
            route: input.route,
            title: input.title,
            launch: input.launch,
            source: Some("runtime_screen_snapshot".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: Some(serde_json::json!({ "capture_source": "rendered_screen_snapshot" })),
        })?;
        self.upsert_runtime_pane(PaneInput {
            id: Some(input.screen.pane_id.0.to_string()),
            session_id: input.session_id.clone(),
            tab_id: input.tab_id.clone(),
            stream_id: None,
            title: input.screen.surface.title.clone(),
            rows: i32::from(input.screen.rows),
            cols: i32::from(input.screen.cols),
            metadata: Some(serde_json::json!({
                "capture_source": "rendered_screen_snapshot"
            })),
        })?;

        let lease = self.acquire_writer_generation_with_retry("runtime-screen-snapshot", 60_000)?;
        let high_water_event_seq = u64_to_i64(input.screen.sequence, "screen sequence")?;
        let write_result = self.write_screen_snapshot(ScreenSnapshotInput {
            id: None,
            session_id: input.session_id,
            pane_id: input.screen.pane_id.0.to_string(),
            writer_generation: lease.id.clone(),
            projection_source: Some(format!("{:?}", input.screen.source).to_lowercase()),
            buffer_kind: Some(input.buffer_kind.unwrap_or_else(|| "normal".to_string())),
            rows: i32::from(input.screen.rows),
            cols: i32::from(input.screen.cols),
            base_event_seq: 0,
            high_water_event_seq,
            high_water_byte_seq: None,
            screen: serde_json::to_value(&input.screen)?,
            parser_version: Some("runtime_screen_snapshot_v1".to_string()),
            projection_version: Some("runtime_screen_snapshot_v1".to_string()),
            metadata: Some(serde_json::json!({
                "capture_source": "rendered_screen_snapshot",
                "capture_semantics": input.capture_semantics
                    .unwrap_or_else(|| "rendered_plaintext_snapshot".to_string())
            })),
        });
        let release_result = self.release_writer_generation(&lease.id);

        match (write_result, release_result) {
            (Ok(id), Ok(())) => Ok(id),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    pub fn record_topology_snapshot_event(
        &self,
        input: TopologySnapshotEventInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        self.upsert_runtime_session(SessionInput {
            id: Some(input.session_id.clone()),
            route: input.route,
            title: input.title,
            launch: input.launch,
            source: Some("runtime_topology_snapshot".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: Some(serde_json::json!({ "capture_source": "topology_snapshot" })),
        })?;

        let pane_high_water = {
            let mut connection = self.connection()?;
            topology_pane_high_water_from_store(
                &mut connection,
                &input.session_id,
                &input.topology,
            )?
        };
        let lease =
            self.acquire_writer_generation_with_retry("runtime-topology-snapshot", 60_000)?;
        let write_result = self.write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: input.session_id,
            writer_generation: lease.id.clone(),
            pane_high_water,
            topology: serde_json::to_value(&input.topology)?,
            source: Some("runtime_topology_snapshot".to_string()),
            metadata: Some(serde_json::json!({
                "capture_source": "topology_snapshot"
            })),
        });
        let release_result = self.release_writer_generation(&lease.id);

        match (write_result, release_result) {
            (Ok(id), Ok(())) => Ok(id),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    fn upsert_legacy_visual_session(
        &self,
        saved: &SavedNativeSession,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewTerminalSessionRow {
            id: saved.session_id.0.to_string(),
            route_json: serde_json::to_string(&saved.route)?,
            title: saved.title.clone(),
            launch_json: saved.launch.as_ref().map(serde_json::to_string).transpose()?,
            source: "legacy_save_session".to_string(),
            durability_profile: self.config.durability_profile.as_str().to_string(),
            retention_policy_id: DEFAULT_RETENTION_POLICY_ID.to_string(),
            private_mode: 0,
            created_at_ms: saved.saved_at_ms,
            updated_at_ms: now,
            closed_at_ms: None,
            state: "legacy_visual_only".to_string(),
            metadata_json: Some(serde_json::to_string(&serde_json::json!({
                "manifest": saved.manifest,
                "visual_restore_only": true
            }))?),
        };
        insert_into(terminal_sessions::table)
            .values(&row)
            .on_conflict(terminal_sessions::id)
            .do_update()
            .set((
                terminal_sessions::route_json.eq(row.route_json.clone()),
                terminal_sessions::title.eq(row.title.clone()),
                terminal_sessions::launch_json.eq(row.launch_json.clone()),
                terminal_sessions::source.eq(row.source.clone()),
                terminal_sessions::updated_at_ms.eq(row.updated_at_ms),
                terminal_sessions::state.eq(row.state.clone()),
                terminal_sessions::metadata_json.eq(row.metadata_json.clone()),
            ))
            .execute(&mut connection)?;

        let cursor = NewSessionCursorRow {
            session_id: saved.session_id.0.to_string(),
            next_commit_seq: 1,
            writer_generation: None,
            updated_at_ms: now,
        };
        insert_into(terminal_session_cursors::table)
            .values(&cursor)
            .on_conflict(terminal_session_cursors::session_id)
            .do_nothing()
            .execute(&mut connection)?;

        Ok(())
    }

    fn upsert_legacy_visual_pane(
        &self,
        saved: &SavedNativeSession,
        screen: &terminal_projection::ScreenSnapshot,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let pane_id = screen.pane_id.0.to_string();
        let stream_id = DEFAULT_STREAM_ID.to_string();
        let row = NewTerminalPaneRow {
            id: pane_id.clone(),
            session_id: saved.session_id.0.to_string(),
            tab_id: None,
            stream_id: stream_id.clone(),
            title: screen.surface.title.clone(),
            rows: i32::from(screen.rows),
            cols: i32::from(screen.cols),
            last_event_seq: 0,
            created_at_ms: saved.saved_at_ms,
            closed_at_ms: None,
            metadata_json: Some(serde_json::to_string(&serde_json::json!({
                "source": "legacy_save_session"
            }))?),
        };
        insert_into(terminal_panes::table)
            .values(&row)
            .on_conflict(terminal_panes::id)
            .do_update()
            .set((
                terminal_panes::title.eq(row.title.clone()),
                terminal_panes::rows.eq(row.rows),
                terminal_panes::cols.eq(row.cols),
                terminal_panes::metadata_json.eq(row.metadata_json.clone()),
            ))
            .execute(&mut connection)?;

        let cursor = NewStreamCursorRow {
            id: stream_cursor_id(&pane_id, &stream_id),
            session_id: saved.session_id.0.to_string(),
            pane_id,
            stream_id,
            next_event_seq: 1,
            next_byte_seq: 0,
            updated_at_ms: now,
        };
        insert_into(terminal_stream_cursors::table)
            .values(&cursor)
            .on_conflict(terminal_stream_cursors::id)
            .do_nothing()
            .execute(&mut connection)?;

        Ok(())
    }

    pub fn acquire_writer_generation(
        &self,
        process_id: impl Into<String>,
        lease_ms: i64,
    ) -> Result<WriterGenerationLease, TerminalPersistenceV2Error> {
        if lease_ms <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "writer lease_ms must be positive".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let process_id = process_id.into();
        let id = new_id();
        let lease_token = new_id();

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            diesel::update(
                terminal_writer_generations::table.filter(
                    terminal_writer_generations::state
                        .eq("active")
                        .and(terminal_writer_generations::lease_expires_at_ms.le(now)),
                ),
            )
            .set((
                terminal_writer_generations::state.eq("stale"),
                terminal_writer_generations::released_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;

            let active = terminal_writer_generations::table
                .filter(terminal_writer_generations::state.eq("active"))
                .select(WriterGenerationRow::as_select())
                .first::<WriterGenerationRow>(connection)
                .optional()?;
            if active.is_some() {
                return Err(TerminalPersistenceV2Error::WriterAlreadyActive);
            }

            let row = NewWriterGenerationRow {
                id: id.clone(),
                process_id: process_id.clone(),
                lease_token: lease_token.clone(),
                state: "active".to_string(),
                acquired_at_ms: now,
                heartbeat_at_ms: now,
                lease_expires_at_ms: now + lease_ms,
                released_at_ms: None,
                metadata_json: None,
            };
            insert_into(terminal_writer_generations::table)
                .values(&row)
                .execute(connection)
                .map_err(map_writer_generation_insert_error)?;
            insert_clock_anchor(connection, &id, now, "writer_acquire")?;

            Ok(())
        })?;

        Ok(WriterGenerationLease {
            id,
            process_id,
            lease_token,
            lease_expires_at_ms: now + lease_ms,
        })
    }

    fn acquire_writer_generation_with_retry(
        &self,
        process_id: &str,
        lease_ms: i64,
    ) -> Result<WriterGenerationLease, TerminalPersistenceV2Error> {
        const ATTEMPTS: usize = 40;
        const BACKOFF: Duration = Duration::from_millis(25);

        for attempt in 0..ATTEMPTS {
            match self.acquire_writer_generation(process_id, lease_ms) {
                Ok(lease) => return Ok(lease),
                Err(TerminalPersistenceV2Error::WriterAlreadyActive) if attempt + 1 < ATTEMPTS => {
                    thread::sleep(BACKOFF);
                }
                Err(error) => return Err(error),
            }
        }

        Err(TerminalPersistenceV2Error::WriterAlreadyActive)
    }

    pub fn heartbeat_writer_generation(
        &self,
        writer_generation: &str,
        lease_ms: i64,
    ) -> Result<(), TerminalPersistenceV2Error> {
        if lease_ms <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "writer lease_ms must be positive".to_string(),
            ));
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let updated = diesel::update(
                terminal_writer_generations::table
                    .filter(terminal_writer_generations::id.eq(writer_generation))
                    .filter(terminal_writer_generations::state.eq("active")),
            )
            .set((
                terminal_writer_generations::heartbeat_at_ms.eq(now),
                terminal_writer_generations::lease_expires_at_ms.eq(now + lease_ms),
            ))
            .execute(connection)?;
            if updated == 0 {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "active writer generation not found for heartbeat".to_string(),
                ));
            }
            insert_clock_anchor(connection, writer_generation, now, "writer_heartbeat")?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn release_writer_generation(
        &self,
        writer_generation: &str,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let updated = diesel::update(
                terminal_writer_generations::table
                    .filter(terminal_writer_generations::id.eq(writer_generation))
                    .filter(terminal_writer_generations::state.eq("active")),
            )
            .set((
                terminal_writer_generations::state.eq("released"),
                terminal_writer_generations::released_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;
            if updated == 0 {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "active writer generation not found for release".to_string(),
                ));
            }
            insert_clock_anchor(connection, writer_generation, now, "writer_release")?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn append_stream_segment(
        &self,
        input: StreamSegmentInput,
    ) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
        if input.payload.is_empty() {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "stream segment payload must not be empty".to_string(),
            ));
        }
        if input.event_count != 1 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "stream segment MVP accepts exactly one journal event per segment".to_string(),
            ));
        }
        if self.config.failpoints.stream_segment_before_transaction_storage_full {
            self.record_storage_pressure_write_failure(
                "append_stream_segment",
                "synthetic_sqlite_full",
                None,
            )?;
            return Err(TerminalPersistenceV2Error::InvalidData(
                "failpoint stream_segment_before_transaction_storage_full".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let occurred_at_ms = input.occurred_at_ms.unwrap_or(now);
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let payload_len = checked_len(input.payload.len(), "payload length")?;
        let payload_checksum = blake3_hash_bytes(&input.payload);
        let metadata_json = json_metadata(&input.metadata)?;
        let source_event_id_hash = input.source_event_id_hash.clone();
        let capture_source_kind = source_event_id_hash
            .as_ref()
            .map(|_| stream_capture_source_kind(&input.pane_id, &stream_id));
        let buffer_mode_transitions = detect_buffer_mode_transitions(&input.payload);

        let append_result = connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            if let (Some(source_kind), Some(source_event_id_hash)) =
                (capture_source_kind.as_deref(), source_event_id_hash.as_deref())
            {
                if let Some(receipt) = load_capture_receipt(
                    connection,
                    &input.session_id,
                    source_kind,
                    source_event_id_hash,
                )? {
                    if receipt.source_payload_hash != payload_checksum {
                        return Err(TerminalPersistenceV2Error::InvalidData(format!(
                            "capture receipt payload hash mismatch for source_kind={source_kind}"
                        )));
                    }
                    return stream_segment_receipt_from_capture_receipt(connection, &receipt);
                }
            }

            ensure_active_writer(connection, &input.writer_generation, now)?;
            let commit = allocate_commit(
                connection,
                &input.session_id,
                "stream_segment",
                &input.writer_generation,
                occurred_at_ms,
                now,
                None,
            )?;
            let cursor =
                load_stream_cursor(connection, &input.session_id, &input.pane_id, &stream_id)?;
            let event_seq_low = cursor.next_event_seq;
            let event_seq_high = cursor.next_event_seq;
            let byte_low = cursor.next_byte_seq;
            let byte_high = cursor.next_byte_seq + payload_len;
            let segment_id = new_id();
            let event_id = new_id();
            let capture_semantics =
                input.capture_semantics.unwrap_or_else(|| "raw_vt_stream".to_string());
            validate_capture_semantics_domain(&capture_semantics)?;
            let event_type = input.event_type.unwrap_or_else(|| "terminal_output".to_string());
            let payload_json =
                input.payload_json.as_ref().map(serde_json::to_string).transpose()?;
            let payload_schema_id = payload_json
                .as_ref()
                .map(|_| payload_schema_id_for_journal_event(&event_type).to_string());
            let transition_count =
                checked_len(buffer_mode_transitions.len(), "buffer mode transition count")?;
            let final_event_seq = event_seq_high.checked_add(transition_count).ok_or_else(|| {
                TerminalPersistenceV2Error::InvalidData(
                    "buffer mode transition event sequence overflow".to_string(),
                )
            })?;

            let segment = NewStreamSegmentRow {
                id: segment_id.clone(),
                session_id: input.session_id.clone(),
                pane_id: input.pane_id.clone(),
                commit_id: commit.id.clone(),
                stream_id: stream_id.clone(),
                event_seq_low,
                event_seq_high,
                byte_low,
                byte_high,
                payload: input.payload.clone(),
                payload_len,
                stored_byte_len: payload_len,
                uncompressed_byte_len: Some(payload_len),
                checksum_algorithm: "blake3".to_string(),
                checksum: payload_checksum.clone(),
                compression: "none".to_string(),
                capture_semantics: capture_semantics.clone(),
                encryption_state: "plaintext".to_string(),
                key_ref: None,
                created_at_ms: now,
                writer_generation: input.writer_generation.clone(),
                metadata_json: metadata_json.clone(),
            };
            insert_into(terminal_stream_segments::table).values(&segment).execute(connection)?;
            if self.config.failpoints.stream_segment_after_segment_insert {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "failpoint stream_segment_after_segment_insert".to_string(),
                ));
            }

            let event = NewJournalEventRow {
                id: event_id.clone(),
                session_id: input.session_id.clone(),
                pane_id: Some(input.pane_id.clone()),
                commit_id: commit.id.clone(),
                stream_id: stream_id.clone(),
                event_scope_kind: "pane".to_string(),
                event_scope_id: input.pane_id.clone(),
                event_seq: event_seq_low,
                event_type,
                byte_low: Some(byte_low),
                byte_high: Some(byte_high),
                payload_json,
                payload_schema_id,
                source_event_id_hash: source_event_id_hash.clone(),
                occurred_at_ms,
                created_at_ms: now,
                capture_semantics: capture_semantics.clone(),
                trust_level: input.trust_level.unwrap_or_else(|| "captured".to_string()),
                metadata_json: metadata_json.clone(),
            };
            insert_into(terminal_journal_events::table).values(&event).execute(connection)?;

            for (transition_index, transition) in buffer_mode_transitions.iter().enumerate() {
                let transition_offset =
                    checked_len(transition_index + 1, "buffer mode transition offset")?;
                let transition_event_seq =
                    event_seq_low.checked_add(transition_offset).ok_or_else(|| {
                        TerminalPersistenceV2Error::InvalidData(
                            "buffer mode transition event sequence overflow".to_string(),
                        )
                    })?;
                let transition_byte_low =
                    byte_low.checked_add(transition.byte_offset).ok_or_else(|| {
                        TerminalPersistenceV2Error::InvalidData(
                            "buffer mode transition byte range overflow".to_string(),
                        )
                    })?;
                let transition_byte_high =
                    transition_byte_low.checked_add(transition.byte_len).ok_or_else(|| {
                        TerminalPersistenceV2Error::InvalidData(
                            "buffer mode transition byte range overflow".to_string(),
                        )
                    })?;
                let payload_json = serde_json::to_string(&serde_json::json!({
                    "action": transition.action,
                    "mode": transition.mode,
                    "target_buffer_kind": transition.target_buffer_kind,
                    "derived_from_event_seq": event_seq_low
                }))?;
                let transition_event = NewJournalEventRow {
                    id: new_id(),
                    session_id: input.session_id.clone(),
                    pane_id: Some(input.pane_id.clone()),
                    commit_id: commit.id.clone(),
                    stream_id: stream_id.clone(),
                    event_scope_kind: "pane".to_string(),
                    event_scope_id: input.pane_id.clone(),
                    event_seq: transition_event_seq,
                    event_type: "terminal_buffer_mode".to_string(),
                    byte_low: Some(transition_byte_low),
                    byte_high: Some(transition_byte_high.min(byte_high)),
                    payload_json: Some(payload_json),
                    payload_schema_id: Some(PAYLOAD_SCHEMA_JOURNAL_EVENT_V1.to_string()),
                    source_event_id_hash: None,
                    occurred_at_ms,
                    created_at_ms: now,
                    capture_semantics: capture_semantics.clone(),
                    trust_level: "parser_derived".to_string(),
                    metadata_json: Some(serde_json::to_string(&serde_json::json!({
                        "parser": "terminal_buffer_mode_detector_v1",
                        "source_segment_id": segment_id.clone()
                    }))?),
                };
                insert_into(terminal_journal_events::table)
                    .values(&transition_event)
                    .execute(connection)?;
            }

            let outbox = NewOutboxMessageRow {
                id: new_id(),
                message_kind: "pane_history_projection".to_string(),
                dedupe_key: Some(normalize_outbox_dedupe_key(&format!(
                    "pane_history_projection:{}",
                    commit.id
                ))),
                state: "pending".to_string(),
                payload_json: serde_json::to_string(&serde_json::json!({
                    "session_id": input.session_id.clone(),
                    "pane_id": input.pane_id.clone(),
                    "stream_id": stream_id.clone(),
                    "commit_id": commit.id.clone(),
                    "event_seq_low": event_seq_low,
                    "event_seq_high": final_event_seq,
                    "byte_low": byte_low,
                    "byte_high": byte_high
                }))?,
                attempts: 0,
                max_attempts: 5,
                claimed_by: None,
                lease_token: None,
                claimed_until_ms: None,
                next_run_at_ms: now,
                last_error: None,
                created_at_ms: now,
                updated_at_ms: now,
            };
            insert_into(terminal_outbox_messages::table).values(&outbox).execute(connection)?;

            if let (Some(source_kind), Some(source_event_id_hash)) =
                (capture_source_kind.as_deref(), source_event_id_hash.as_deref())
            {
                let receipt = NewCaptureReceiptRow {
                    id: new_id(),
                    session_id: input.session_id.clone(),
                    commit_id: Some(commit.id.clone()),
                    source_kind: source_kind.to_string(),
                    source_event_id_hash: source_event_id_hash.to_string(),
                    source_payload_hash: payload_checksum.clone(),
                    received_at_ms: occurred_at_ms,
                    created_at_ms: now,
                    metadata_json: metadata_json.clone(),
                };
                insert_into(terminal_capture_receipts::table)
                    .values(&receipt)
                    .execute(connection)?;
            }

            advance_stream_cursor(connection, &cursor.id, final_event_seq + 1, byte_high, now)?;
            diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(&input.pane_id)))
                .set(terminal_panes::last_event_seq.eq(final_event_seq))
                .execute(connection)?;

            Ok(StreamSegmentReceipt {
                commit_id: commit.id,
                commit_seq: commit.commit_seq,
                segment_id,
                event_id,
                event_seq_low,
                event_seq_high,
                byte_low,
                byte_high,
                checksum: payload_checksum,
            })
        });
        if let Err(error) = &append_result
            && is_storage_full_like_error(error)
        {
            let _ = self.record_storage_pressure_write_failure(
                "append_stream_segment",
                "sqlite_full",
                Some(error.to_string()),
            );
        }
        append_result
    }

    pub fn append_journal_event(
        &self,
        input: JournalEventInput,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let occurred_at_ms = input.occurred_at_ms.unwrap_or(now);
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let payload_json = input.payload_json.as_ref().map(serde_json::to_string).transpose()?;
        let payload_schema_id = payload_json
            .as_ref()
            .map(|_| payload_schema_id_for_journal_event(&input.event_type).to_string());
        let metadata_json = json_metadata(&input.metadata)?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            ensure_active_writer(connection, &input.writer_generation, now)?;
            let capture_semantics =
                input.capture_semantics.unwrap_or_else(|| "raw_vt_stream".to_string());
            validate_capture_semantics_domain(&capture_semantics)?;
            let commit = allocate_commit(
                connection,
                &input.session_id,
                input.commit_kind.as_deref().unwrap_or("journal_event"),
                &input.writer_generation,
                occurred_at_ms,
                now,
                None,
            )?;
            let scope = event_scope(&input.session_id, input.pane_id.as_deref());
            let event_seq = if let Some(pane_id) = input.pane_id.as_deref() {
                let cursor =
                    load_stream_cursor(connection, &input.session_id, pane_id, &stream_id)?;
                advance_stream_cursor(
                    connection,
                    &cursor.id,
                    cursor.next_event_seq + 1,
                    cursor.next_byte_seq,
                    now,
                )?;
                diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(pane_id)))
                    .set(terminal_panes::last_event_seq.eq(cursor.next_event_seq))
                    .execute(connection)?;
                cursor.next_event_seq
            } else {
                commit.commit_seq
            };
            let event_id = new_id();
            let row = NewJournalEventRow {
                id: event_id.clone(),
                session_id: input.session_id,
                pane_id: input.pane_id,
                commit_id: commit.id.clone(),
                stream_id,
                event_scope_kind: scope.kind,
                event_scope_id: scope.id,
                event_seq,
                event_type: input.event_type,
                byte_low: None,
                byte_high: None,
                payload_json,
                payload_schema_id,
                source_event_id_hash: input.source_event_id_hash,
                occurred_at_ms,
                created_at_ms: now,
                capture_semantics,
                trust_level: input.trust_level.unwrap_or_else(|| "captured".to_string()),
                metadata_json,
            };
            insert_into(terminal_journal_events::table).values(&row).execute(connection)?;

            Ok(JournalEventReceipt {
                commit_id: commit.id,
                commit_seq: commit.commit_seq,
                event_id,
                event_seq,
            })
        })
    }

    pub fn write_command_block(
        &self,
        input: CommandBlockInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let id = input.id.unwrap_or_else(new_id);
        let metadata_json = json_metadata(&input.metadata)?;
        let row = NewCommandBlockRow {
            id: id.clone(),
            session_id: input.session_id,
            pane_id: input.pane_id,
            commit_id: input.commit_id,
            command_text: input.command_text,
            display_text: input.display_text,
            redacted_text: input.redacted_text,
            command_text_source: input
                .command_text_source
                .unwrap_or_else(|| "ui_submit".to_string()),
            trust_level: input.trust_level.unwrap_or_else(|| "verified".to_string()),
            state: input.state.unwrap_or_else(|| "submitted".to_string()),
            cwd: input.cwd,
            cwd_source: input.cwd_source,
            exit_code: input.exit_code,
            started_event_seq: input.started_event_seq,
            submitted_event_seq: input.submitted_event_seq,
            finished_event_seq: input.finished_event_seq,
            output_event_seq_low: input.output_event_seq_low,
            output_event_seq_high: input.output_event_seq_high,
            output_byte_low: input.output_byte_low,
            output_byte_high: input.output_byte_high,
            sensitivity_class: input.sensitivity_class.unwrap_or_else(|| "unknown".to_string()),
            created_at_ms: input.created_at_ms.unwrap_or(now),
            updated_at_ms: now,
            metadata_json,
        };
        validate_optional_range(
            row.output_event_seq_low,
            row.output_event_seq_high,
            "command output event",
        )?;
        validate_optional_half_open_range(
            row.output_byte_low,
            row.output_byte_high,
            "command output byte",
        )?;

        insert_into(terminal_command_blocks::table).values(&row).execute(&mut connection)?;
        Ok(id)
    }

    pub fn upsert_command_history_entry(
        &self,
        input: CommandHistoryEntryInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let command_hash_material = input.command_text.as_deref().unwrap_or(&input.display_text);
        let command_hash = local_keyed_command_hash(&mut connection, command_hash_material)?;
        let id = input.id.unwrap_or_else(|| {
            stable_history_id(
                &input.scope_kind,
                input.session_id.as_deref(),
                input.pane_id.as_deref(),
                &command_hash,
            )
        });
        let metadata_json = json_metadata(&input.metadata)?;
        let row = NewCommandHistoryEntryRow {
            id: id.clone(),
            session_id: input.session_id,
            pane_id: input.pane_id,
            command_block_id: input.command_block_id,
            scope_kind: input.scope_kind,
            command_text: input.command_text,
            display_text: input.display_text,
            redacted_text: input.redacted_text,
            command_hash_algorithm: COMMAND_HASH_ALGORITHM.to_string(),
            command_hash_scope: COMMAND_HASH_SCOPE.to_string(),
            command_hash,
            cwd: input.cwd,
            shell_kind: input.shell_kind,
            trust_level: input.trust_level.unwrap_or_else(|| "verified".to_string()),
            source: input.source.unwrap_or_else(|| "ui_submit".to_string()),
            sensitivity_class: input.sensitivity_class.unwrap_or_else(|| "unknown".to_string()),
            redaction_state: input.redaction_state.unwrap_or_else(|| "unscanned".to_string()),
            rerun_policy: input.rerun_policy.unwrap_or_else(|| "confirm".to_string()),
            first_used_at_ms: input.first_used_at_ms.unwrap_or(now),
            last_used_at_ms: input.last_used_at_ms.unwrap_or(now),
            use_count: input.use_count.unwrap_or(1),
            metadata_json,
        };

        insert_into(terminal_command_history_entries::table)
            .values(&row)
            .on_conflict(terminal_command_history_entries::id)
            .do_update()
            .set((
                terminal_command_history_entries::last_used_at_ms.eq(row.last_used_at_ms),
                terminal_command_history_entries::use_count
                    .eq(terminal_command_history_entries::use_count + 1),
                terminal_command_history_entries::command_block_id.eq(row.command_block_id.clone()),
                terminal_command_history_entries::cwd.eq(row.cwd.clone()),
                terminal_command_history_entries::metadata_json.eq(row.metadata_json.clone()),
            ))
            .execute(&mut connection)?;

        Ok(id)
    }

    pub fn upsert_delivery_client(
        &self,
        input: DeliveryClientInput,
    ) -> Result<DeliveryClientRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let id = input.id.unwrap_or_else(new_id);
        let row = NewDeliveryClientRow {
            id: id.clone(),
            client_kind: input.client_kind,
            install_ref_hash: input.install_ref_hash,
            browser_profile_ref_hash: input.browser_profile_ref_hash,
            user_agent_hash: input.user_agent_hash,
            created_at_ms: now,
            last_seen_at_ms: now,
            trust_state: input.trust_state.unwrap_or_else(|| "local_unverified".to_string()),
        };

        insert_into(terminal_clients::table)
            .values(&row)
            .on_conflict(terminal_clients::id)
            .do_update()
            .set((
                terminal_clients::client_kind.eq(row.client_kind.clone()),
                terminal_clients::install_ref_hash.eq(row.install_ref_hash.clone()),
                terminal_clients::browser_profile_ref_hash.eq(row.browser_profile_ref_hash.clone()),
                terminal_clients::user_agent_hash.eq(row.user_agent_hash.clone()),
                terminal_clients::last_seen_at_ms.eq(row.last_seen_at_ms),
                terminal_clients::trust_state.eq(row.trust_state.clone()),
            ))
            .execute(&mut connection)?;

        Ok(DeliveryClientRecord {
            id,
            client_kind: row.client_kind,
            last_seen_at_ms: now,
            trust_state: row.trust_state,
        })
    }

    pub fn record_delivery_progress(
        &self,
        input: DeliveryProgressInput,
    ) -> Result<DeliveryOffsetRecord, TerminalPersistenceV2Error> {
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        validate_non_negative_seq(input.last_sent_event_seq, "last sent event seq")?;
        validate_non_negative_seq(input.last_acked_event_seq, "last acked event seq")?;

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            touch_delivery_client(connection, &input.client_id, now)?;
            let persisted = load_persisted_event_high_water(
                connection,
                &input.session_id,
                &input.pane_id,
                &stream_id,
            )?;
            let existing = load_delivery_offset(
                connection,
                &input.client_id,
                &input.session_id,
                &input.pane_id,
                &stream_id,
            )?;
            let existing_sent = existing.as_ref().map_or(0, |row| row.last_sent_event_seq);
            let existing_acked = existing.as_ref().map_or(0, |row| row.last_acked_event_seq);
            let last_sent = input.last_sent_event_seq.unwrap_or(existing_sent).max(existing_sent);
            let last_acked =
                input.last_acked_event_seq.unwrap_or(existing_acked).max(existing_acked);

            if last_sent > persisted {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "last sent event seq {last_sent} is above persisted high-water {persisted}"
                )));
            }
            if last_acked > last_sent {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "last acked event seq {last_acked} is above last sent event seq {last_sent}"
                )));
            }

            let replay_from_event_seq =
                (last_acked < persisted).then_some(last_acked.saturating_add(1));
            let gap_state = match replay_from_event_seq {
                Some(from)
                    if has_history_gap_in_range(
                        connection,
                        &input.session_id,
                        &input.pane_id,
                        &stream_id,
                        from,
                        persisted,
                    )? =>
                {
                    "gap"
                }
                _ => "none",
            }
            .to_string();
            let row = NewDeliveryOffsetRow {
                id: delivery_offset_id(
                    &input.client_id,
                    &input.session_id,
                    &input.pane_id,
                    &stream_id,
                ),
                client_id: input.client_id.clone(),
                session_id: input.session_id.clone(),
                pane_id: Some(input.pane_id.clone()),
                stream_id: stream_id.clone(),
                last_sent_event_seq: last_sent,
                last_acked_event_seq: last_acked,
                last_persisted_event_seq: persisted,
                replay_from_event_seq,
                gap_state,
                updated_at_ms: now,
            };

            insert_into(terminal_delivery_offsets::table)
                .values(&row)
                .on_conflict(terminal_delivery_offsets::id)
                .do_update()
                .set((
                    terminal_delivery_offsets::last_sent_event_seq.eq(row.last_sent_event_seq),
                    terminal_delivery_offsets::last_acked_event_seq.eq(row.last_acked_event_seq),
                    terminal_delivery_offsets::last_persisted_event_seq
                        .eq(row.last_persisted_event_seq),
                    terminal_delivery_offsets::replay_from_event_seq.eq(row.replay_from_event_seq),
                    terminal_delivery_offsets::gap_state.eq(row.gap_state.clone()),
                    terminal_delivery_offsets::updated_at_ms.eq(row.updated_at_ms),
                ))
                .execute(connection)?;

            load_delivery_offset(
                connection,
                &input.client_id,
                &input.session_id,
                &input.pane_id,
                &stream_id,
            )?
            .map(Into::into)
            .ok_or_else(|| {
                TerminalPersistenceV2Error::InvalidData(
                    "delivery offset upsert did not return a row".to_string(),
                )
            })
        })
    }

    pub fn delivery_replay_window(
        &self,
        input: DeliveryOffsetInput,
    ) -> Result<DeliveryReplayWindow, TerminalPersistenceV2Error> {
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let mut connection = self.connection()?;
        let persisted = load_persisted_event_high_water(
            &mut connection,
            &input.session_id,
            &input.pane_id,
            &stream_id,
        )?;
        let offset = load_delivery_offset(
            &mut connection,
            &input.client_id,
            &input.session_id,
            &input.pane_id,
            &stream_id,
        )?;
        let from_event_seq = offset
            .as_ref()
            .and_then(|row| row.replay_from_event_seq)
            .or_else(|| {
                let acked = offset.as_ref().map_or(0, |row| row.last_acked_event_seq);
                (acked < persisted).then_some(acked.saturating_add(1))
            })
            .filter(|from| *from <= persisted);
        let gap_state = match from_event_seq {
            Some(from)
                if has_history_gap_in_range(
                    &mut connection,
                    &input.session_id,
                    &input.pane_id,
                    &stream_id,
                    from,
                    persisted,
                )? =>
            {
                "gap"
            }
            Some(_) => offset.as_ref().map_or("none", |row| row.gap_state.as_str()),
            None => "none",
        }
        .to_string();

        Ok(DeliveryReplayWindow { from_event_seq, to_event_seq: persisted, gap_state })
    }

    pub fn enqueue_outbox_message(
        &self,
        input: OutboxMessageInput,
    ) -> Result<OutboxMessageRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let max_attempts = input.max_attempts.unwrap_or(5);
        if max_attempts <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "outbox max_attempts must be positive".to_string(),
            ));
        }
        let dedupe_key = input.dedupe_key.as_deref().map(normalize_outbox_dedupe_key);
        if let Some(dedupe_key) = dedupe_key.as_deref()
            && let Some(existing) = load_outbox_message_by_dedupe(&mut connection, dedupe_key)?
        {
            return existing.try_into();
        }

        let row = NewOutboxMessageRow {
            id: new_id(),
            message_kind: input.message_kind,
            dedupe_key,
            state: "pending".to_string(),
            payload_json: serde_json::to_string(&input.payload)?,
            attempts: 0,
            max_attempts,
            claimed_by: None,
            lease_token: None,
            claimed_until_ms: None,
            next_run_at_ms: input.next_run_at_ms.unwrap_or(now),
            last_error: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        insert_into(terminal_outbox_messages::table).values(&row).execute(&mut connection)?;
        load_outbox_message(&mut connection, &row.id)?.try_into()
    }

    pub fn claim_next_outbox_message(
        &self,
        worker_id: &str,
        lease_ms: i64,
    ) -> Result<Option<OutboxMessageRecord>, TerminalPersistenceV2Error> {
        if lease_ms <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "outbox lease_ms must be positive".to_string(),
            ));
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let candidate = terminal_outbox_messages::table
                .filter(
                    terminal_outbox_messages::state
                        .eq("pending")
                        .and(terminal_outbox_messages::next_run_at_ms.le(now))
                        .or(terminal_outbox_messages::state
                            .eq("claimed")
                            .and(terminal_outbox_messages::claimed_until_ms.le(Some(now)))),
                )
                .filter(
                    terminal_outbox_messages::attempts.lt(terminal_outbox_messages::max_attempts),
                )
                .order((
                    terminal_outbox_messages::next_run_at_ms.asc(),
                    terminal_outbox_messages::created_at_ms.asc(),
                ))
                .select(OutboxMessageRow::as_select())
                .first::<OutboxMessageRow>(connection)
                .optional()?;
            let Some(candidate) = candidate else {
                return Ok(None);
            };

            let lease_token = new_id();
            let updated = diesel::update(
                terminal_outbox_messages::table
                    .filter(terminal_outbox_messages::id.eq(&candidate.id))
                    .filter(
                        terminal_outbox_messages::state.eq("pending").or(
                            terminal_outbox_messages::state
                                .eq("claimed")
                                .and(terminal_outbox_messages::claimed_until_ms.le(Some(now))),
                        ),
                    ),
            )
            .set((
                terminal_outbox_messages::state.eq("claimed"),
                terminal_outbox_messages::attempts.eq(candidate.attempts + 1),
                terminal_outbox_messages::claimed_by.eq(Some(worker_id.to_string())),
                terminal_outbox_messages::lease_token.eq(Some(lease_token)),
                terminal_outbox_messages::claimed_until_ms.eq(Some(now + lease_ms)),
                terminal_outbox_messages::last_error.eq::<Option<String>>(None),
                terminal_outbox_messages::updated_at_ms.eq(now),
            ))
            .execute(connection)?;
            if updated == 0 {
                return Ok(None);
            }

            load_outbox_message(connection, &candidate.id)?.try_into().map(Some)
        })
    }

    pub fn mark_outbox_message_done(
        &self,
        message_id: &str,
        lease_token: &str,
    ) -> Result<bool, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let updated = diesel::update(
            terminal_outbox_messages::table
                .filter(terminal_outbox_messages::id.eq(message_id))
                .filter(terminal_outbox_messages::lease_token.eq(Some(lease_token.to_string())))
                .filter(terminal_outbox_messages::state.eq("claimed")),
        )
        .set((
            terminal_outbox_messages::state.eq("done"),
            terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
            terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
            terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
            terminal_outbox_messages::updated_at_ms.eq(now),
        ))
        .execute(&mut connection)?;
        Ok(updated > 0)
    }

    pub fn fail_outbox_message(
        &self,
        message_id: &str,
        lease_token: &str,
        error: &str,
    ) -> Result<OutboxMessageRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let row = terminal_outbox_messages::table
                .filter(terminal_outbox_messages::id.eq(message_id))
                .filter(terminal_outbox_messages::lease_token.eq(Some(lease_token.to_string())))
                .filter(terminal_outbox_messages::state.eq("claimed"))
                .select(OutboxMessageRow::as_select())
                .first::<OutboxMessageRow>(connection)?;
            let next_state =
                if row.attempts >= row.max_attempts { "quarantined" } else { "pending" };
            let retry_delay_ms = 1_000_i64.saturating_mul(row.attempts.max(1));
            diesel::update(
                terminal_outbox_messages::table.filter(terminal_outbox_messages::id.eq(message_id)),
            )
            .set((
                terminal_outbox_messages::state.eq(next_state),
                terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
                terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
                terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
                terminal_outbox_messages::next_run_at_ms.eq(now + retry_delay_ms),
                terminal_outbox_messages::last_error.eq(Some(error.to_string())),
                terminal_outbox_messages::updated_at_ms.eq(now),
            ))
            .execute(connection)?;
            load_outbox_message(connection, message_id)?.try_into()
        })
    }

    pub fn outbox_diagnostics(
        &self,
    ) -> Result<OutboxDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_outbox_diagnostics(&mut connection, self.config.clock.now_ms())
    }

    pub fn compression_diagnostics(
        &self,
    ) -> Result<CompressionDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_compression_diagnostics(&mut connection, self.config.clock.now_ms())
    }

    pub fn retention_diagnostics(
        &self,
        selected_policy_id: Option<&str>,
    ) -> Result<RetentionDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_retention_diagnostics(
            &mut connection,
            self.config.clock.now_ms(),
            selected_policy_id,
        )
    }

    pub fn write_screen_snapshot(
        &self,
        input: ScreenSnapshotInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        validate_positive_dimensions(input.rows, input.cols)?;
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let screen_json = serde_json::to_string(&input.screen)?;
        let checksum = blake3_hash_text(&screen_json);
        let metadata_json = json_metadata(&input.metadata)?;
        let id = input.id.unwrap_or_else(new_id);

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            ensure_active_writer(connection, &input.writer_generation, now)?;
            let commit = allocate_commit(
                connection,
                &input.session_id,
                "screen_snapshot",
                &input.writer_generation,
                now,
                now,
                None,
            )?;
            let row = NewScreenSnapshotRow {
                id: id.clone(),
                session_id: input.session_id,
                pane_id: input.pane_id,
                commit_id: commit.id,
                projection_source: input
                    .projection_source
                    .unwrap_or_else(|| "terminal_projection".to_string()),
                buffer_kind: input.buffer_kind.unwrap_or_else(|| "normal".to_string()),
                rows: input.rows,
                cols: input.cols,
                base_event_seq: input.base_event_seq,
                high_water_event_seq: input.high_water_event_seq,
                high_water_byte_seq: input.high_water_byte_seq,
                screen_json,
                parser_version: input
                    .parser_version
                    .unwrap_or_else(|| "terminal_projection/0.1".to_string()),
                projection_version: input
                    .projection_version
                    .unwrap_or_else(|| "screen_snapshot_v1".to_string()),
                checksum_algorithm: "blake3".to_string(),
                checksum,
                created_at_ms: now,
                metadata_json,
            };
            insert_into(terminal_screen_snapshots::table).values(&row).execute(connection)?;
            Ok(())
        })?;

        Ok(id)
    }

    pub fn write_topology_snapshot(
        &self,
        input: TopologySnapshotInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let pane_high_water_json = serde_json::to_string(&input.pane_high_water)?;
        let topology_json = serde_json::to_string(&input.topology)?;
        let checksum = blake3_hash_text(&topology_json);
        let metadata_json = json_metadata(&input.metadata)?;
        let id = input.id.unwrap_or_else(new_id);

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            ensure_active_writer(connection, &input.writer_generation, now)?;
            let commit = allocate_commit(
                connection,
                &input.session_id,
                "topology_snapshot",
                &input.writer_generation,
                now,
                now,
                None,
            )?;
            let row = NewTopologySnapshotRow {
                id: id.clone(),
                session_id: input.session_id,
                commit_id: commit.id,
                high_water_commit_seq: commit.commit_seq,
                pane_high_water_json,
                topology_json,
                payload_schema_id: Some(PAYLOAD_SCHEMA_TOPOLOGY_SNAPSHOT_V1.to_string()),
                checksum_algorithm: "blake3".to_string(),
                checksum,
                source: input.source.unwrap_or_else(|| "runtime".to_string()),
                created_at_ms: now,
                metadata_json,
            };
            insert_into(terminal_topology_snapshots::table).values(&row).execute(connection)?;
            Ok(())
        })?;

        Ok(id)
    }
}
