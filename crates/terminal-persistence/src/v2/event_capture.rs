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
}
