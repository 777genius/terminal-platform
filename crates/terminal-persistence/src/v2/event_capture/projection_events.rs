use super::super::*;

impl TerminalPersistenceV2 {
    pub fn record_screen_snapshot_event(
        &self,
        input: ScreenSnapshotEventInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.record_screen_snapshot_event_with_connection(&mut connection, input)
    }

    pub(crate) fn record_screen_snapshot_event_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: ScreenSnapshotEventInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        self.upsert_runtime_session_with_connection(
            connection,
            SessionInput {
                id: Some(input.session_id.clone()),
                route: input.route,
                title: input.title,
                launch: input.launch,
                source: Some("runtime_screen_snapshot".to_string()),
                durability_profile: None,
                retention_policy_id: None,
                private_mode: false,
                metadata: Some(serde_json::json!({ "capture_source": "rendered_screen_snapshot" })),
            },
        )?;
        self.upsert_runtime_pane_with_connection(
            connection,
            PaneInput {
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
            },
        )?;

        let high_water_event_seq = self.pane_last_event_seq_with_connection(
            connection,
            &input.session_id,
            &input.screen.pane_id.0.to_string(),
        )?;
        let projection_sequence = input.screen.sequence;
        let lease = self.acquire_writer_generation_with_retry_on_connection(
            connection,
            "runtime-screen-snapshot",
            60_000,
        )?;
        let write_result = self.write_screen_snapshot_with_connection(
            connection,
            ScreenSnapshotInput {
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
                    "projection_sequence": projection_sequence,
                    "capture_semantics": input.capture_semantics
                        .unwrap_or_else(|| "rendered_plaintext_snapshot".to_string())
                })),
            },
        );
        let release_result = self.release_writer_generation_with_connection(connection, &lease.id);

        match (write_result, release_result) {
            (Ok(id), Ok(())) => Ok(id),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    fn pane_last_event_seq_with_connection(
        &self,
        connection: &mut SqliteConnection,
        session_id: &str,
        pane_id: &str,
    ) -> Result<i64, TerminalPersistenceV2Error> {
        Ok(terminal_panes::table
            .filter(terminal_panes::session_id.eq(session_id))
            .filter(terminal_panes::id.eq(pane_id))
            .select(terminal_panes::last_event_seq)
            .first::<i64>(connection)
            .optional()?
            .unwrap_or(0)
            .max(0))
    }

    pub fn record_topology_snapshot_event(
        &self,
        input: TopologySnapshotEventInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.record_topology_snapshot_event_with_connection(&mut connection, input)
    }

    pub(crate) fn record_topology_snapshot_event_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: TopologySnapshotEventInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        self.upsert_runtime_session_with_connection(
            connection,
            SessionInput {
                id: Some(input.session_id.clone()),
                route: input.route,
                title: input.title,
                launch: input.launch,
                source: Some("runtime_topology_snapshot".to_string()),
                durability_profile: None,
                retention_policy_id: None,
                private_mode: false,
                metadata: Some(serde_json::json!({ "capture_source": "topology_snapshot" })),
            },
        )?;

        let pane_high_water =
            topology_pane_high_water_from_store(connection, &input.session_id, &input.topology)?;
        let lease = self.acquire_writer_generation_with_retry_on_connection(
            connection,
            "runtime-topology-snapshot",
            60_000,
        )?;
        let write_result = self.write_topology_snapshot_with_connection(
            connection,
            TopologySnapshotInput {
                id: None,
                session_id: input.session_id,
                writer_generation: lease.id.clone(),
                pane_high_water,
                topology: serde_json::to_value(&input.topology)?,
                source: Some("runtime_topology_snapshot".to_string()),
                metadata: Some(serde_json::json!({
                    "capture_source": "topology_snapshot"
                })),
            },
        );
        let release_result = self.release_writer_generation_with_connection(connection, &lease.id);

        match (write_result, release_result) {
            (Ok(id), Ok(())) => Ok(id),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }
}
