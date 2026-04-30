use super::*;

impl TerminalPersistenceV2 {
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
