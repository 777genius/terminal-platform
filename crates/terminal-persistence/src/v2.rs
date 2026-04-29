use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper,
    dsl::insert_into,
    prelude::*,
    result::{DatabaseErrorKind, Error as DieselError},
    sqlite::SqliteConnection,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use terminal_backend_api::{BackendCapabilities, ShellLaunchSpec};
use terminal_domain::{BackendKind, SessionRoute};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    db::{
        connection::establish_initialized_connection,
        schema::{
            terminal_backend_capability_reports, terminal_command_blocks,
            terminal_command_history_entries, terminal_commit_log, terminal_db_identity,
            terminal_feature_gates, terminal_journal_events, terminal_panes,
            terminal_restore_drills, terminal_screen_snapshots, terminal_session_cursors,
            terminal_sessions, terminal_stream_cursors, terminal_stream_segments,
            terminal_topology_snapshots, terminal_writer_generations,
        },
    },
    legacy::SavedNativeSession,
};

pub const TERMINAL_PERSISTENCE_APP_ID: i32 = 0x5450_5632;
const DEFAULT_RETENTION_POLICY_ID: &str = "default_full_history";
const DEFAULT_STREAM_ID: &str = "primary";

#[derive(Debug, Error)]
pub enum TerminalPersistenceV2Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("diesel connection: {0}")]
    Connection(#[from] diesel::ConnectionError),
    #[error("diesel query: {0}")]
    Query(#[from] DieselError),
    #[error("migration: {0}")]
    Migration(String),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("wrong sqlite database application_id: {application_id}")]
    WrongDatabase { application_id: i32 },
    #[error("wrong terminal database identity: product={product}, schema_family={schema_family}")]
    IdentityMismatch { product: String, schema_family: String },
    #[error("invalid terminal persistence data: {0}")]
    InvalidData(String),
    #[error("an active terminal writer generation already exists")]
    WriterAlreadyActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurabilityProfile {
    ReliableHistory,
    PerformanceHistory,
    Test,
}

impl DurabilityProfile {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReliableHistory => "reliable_history",
            Self::PerformanceHistory => "performance_history",
            Self::Test => "test",
        }
    }

    #[must_use]
    pub fn sqlite_synchronous(self) -> &'static str {
        match self {
            Self::ReliableHistory => "FULL",
            Self::PerformanceHistory | Self::Test => "NORMAL",
        }
    }
}

impl Default for DurabilityProfile {
    fn default() -> Self {
        Self::ReliableHistory
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureGateName {
    TerminalPersistenceV2Shadow,
    TerminalPersistenceV2Capture,
    TerminalPersistenceV2AuthoritativeReads,
    TerminalPersistenceV2Authoritative,
    MuxStructuredCapture,
    SegmentCompressionZstd,
    RawHistoryExport,
    EncryptedTerminalHistory,
}

impl FeatureGateName {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TerminalPersistenceV2Shadow => "terminal_persistence_v2_shadow",
            Self::TerminalPersistenceV2Capture => "terminal_persistence_v2_capture",
            Self::TerminalPersistenceV2AuthoritativeReads => {
                "terminal_persistence_v2_authoritative_reads"
            }
            Self::TerminalPersistenceV2Authoritative => "terminal_persistence_v2_authoritative",
            Self::MuxStructuredCapture => "mux_structured_capture",
            Self::SegmentCompressionZstd => "segment_compression_zstd",
            Self::RawHistoryExport => "raw_history_export",
            Self::EncryptedTerminalHistory => "encrypted_terminal_history",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureGateState {
    Disabled,
    Shadow,
    Enabled,
    ForceDisabled,
}

impl FeatureGateState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Shadow => "shadow",
            Self::Enabled => "enabled",
            Self::ForceDisabled => "force_disabled",
        }
    }

    fn parse(value: &str) -> Result<Self, TerminalPersistenceV2Error> {
        match value {
            "disabled" => Ok(Self::Disabled),
            "shadow" => Ok(Self::Shadow),
            "enabled" => Ok(Self::Enabled),
            "force_disabled" => Ok(Self::ForceDisabled),
            other => Err(TerminalPersistenceV2Error::InvalidData(format!(
                "unknown feature gate state: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PersistenceClock;

impl PersistenceClock {
    #[must_use]
    pub fn now_ms(self) -> i64 {
        current_time_ms()
    }
}

impl Default for PersistenceClock {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct TerminalPersistenceV2Config {
    pub durability_profile: DurabilityProfile,
    pub busy_timeout_ms: u32,
    pub wal_autocheckpoint_pages: u32,
    pub clock: PersistenceClock,
}

impl TerminalPersistenceV2Config {
    #[must_use]
    pub fn reliable_history() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn test() -> Self {
        Self {
            durability_profile: DurabilityProfile::Test,
            busy_timeout_ms: 1_000,
            wal_autocheckpoint_pages: 64,
            clock: PersistenceClock,
        }
    }
}

impl Default for TerminalPersistenceV2Config {
    fn default() -> Self {
        Self {
            durability_profile: DurabilityProfile::ReliableHistory,
            busy_timeout_ms: 5_000,
            wal_autocheckpoint_pages: 1_000,
            clock: PersistenceClock,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalPersistenceV2 {
    path: PathBuf,
    config: TerminalPersistenceV2Config,
}

impl TerminalPersistenceV2 {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, TerminalPersistenceV2Error> {
        Self::open_with_config(path, TerminalPersistenceV2Config::default())
    }

    pub fn open_with_config(
        path: impl Into<PathBuf>,
        config: TerminalPersistenceV2Config,
    ) -> Result<Self, TerminalPersistenceV2Error> {
        let path = path.into();
        let mut connection = establish_initialized_connection(&path, &config)?;
        verify_seeded_defaults(&mut connection)?;
        Ok(Self { path, config })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connection(&self) -> Result<SqliteConnection, TerminalPersistenceV2Error> {
        establish_initialized_connection(&self.path, &self.config)
    }

    pub fn feature_gate_state(
        &self,
        name: FeatureGateName,
    ) -> Result<FeatureGateState, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let row = terminal_feature_gates::table
            .filter(terminal_feature_gates::feature_name.eq(name.as_str()))
            .select(FeatureGateRow::as_select())
            .first::<FeatureGateRow>(&mut connection)?;
        FeatureGateState::parse(&row.state)
    }

    pub fn set_feature_gate_state(
        &self,
        name: FeatureGateName,
        state: FeatureGateState,
        reason: Option<&str>,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let enabled_at =
            matches!(state, FeatureGateState::Enabled | FeatureGateState::Shadow).then_some(now);
        let disabled_at =
            matches!(state, FeatureGateState::Disabled | FeatureGateState::ForceDisabled)
                .then_some(now);

        diesel::update(
            terminal_feature_gates::table
                .filter(terminal_feature_gates::feature_name.eq(name.as_str())),
        )
        .set((
            terminal_feature_gates::state.eq(state.as_str()),
            terminal_feature_gates::reason.eq(reason.map(ToOwned::to_owned)),
            terminal_feature_gates::enabled_at_ms.eq(enabled_at),
            terminal_feature_gates::disabled_at_ms.eq(disabled_at),
            terminal_feature_gates::updated_at_ms.eq(now),
        ))
        .execute(&mut connection)?;
        Ok(())
    }

    pub fn create_session(
        &self,
        input: SessionInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let session_id = input.id.unwrap_or_else(new_id);
        let route_json = serde_json::to_string(&input.route)?;
        let launch_json = input.launch.as_ref().map(serde_json::to_string).transpose()?;
        let metadata_json = json_metadata(&input.metadata)?;
        let durability_profile = input.durability_profile.unwrap_or(self.config.durability_profile);
        let retention_policy_id =
            input.retention_policy_id.unwrap_or_else(|| DEFAULT_RETENTION_POLICY_ID.to_string());

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let row = NewTerminalSessionRow {
                id: session_id.clone(),
                route_json,
                title: input.title,
                launch_json,
                source: input.source.unwrap_or_else(|| "runtime".to_string()),
                durability_profile: durability_profile.as_str().to_string(),
                retention_policy_id,
                private_mode: bool_to_int(input.private_mode),
                created_at_ms: now,
                updated_at_ms: now,
                closed_at_ms: None,
                state: "active".to_string(),
                metadata_json,
            };
            insert_into(terminal_sessions::table).values(&row).execute(connection)?;

            let cursor = NewSessionCursorRow {
                session_id: session_id.clone(),
                next_commit_seq: 1,
                writer_generation: None,
                updated_at_ms: now,
            };
            insert_into(terminal_session_cursors::table).values(&cursor).execute(connection)?;

            Ok(())
        })?;

        Ok(session_id)
    }

    pub fn create_pane(&self, input: PaneInput) -> Result<String, TerminalPersistenceV2Error> {
        validate_positive_dimensions(input.rows, input.cols)?;
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let pane_id = input.id.unwrap_or_else(new_id);
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let metadata_json = json_metadata(&input.metadata)?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let row = NewTerminalPaneRow {
                id: pane_id.clone(),
                session_id: input.session_id.clone(),
                tab_id: input.tab_id,
                stream_id: stream_id.clone(),
                title: input.title,
                rows: input.rows,
                cols: input.cols,
                last_event_seq: 0,
                created_at_ms: now,
                closed_at_ms: None,
                metadata_json,
            };
            insert_into(terminal_panes::table).values(&row).execute(connection)?;

            let cursor = NewStreamCursorRow {
                id: stream_cursor_id(&pane_id, &stream_id),
                session_id: input.session_id,
                pane_id: pane_id.clone(),
                stream_id,
                next_event_seq: 1,
                next_byte_seq: 0,
                updated_at_ms: now,
            };
            insert_into(terminal_stream_cursors::table).values(&cursor).execute(connection)?;

            Ok(())
        })?;

        Ok(pane_id)
    }

    pub fn record_backend_capability_report(
        &self,
        input: BackendCapabilityReportInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let id = input.id.unwrap_or_else(new_id);
        let evidence_json = json_metadata(&input.evidence)?;
        let row = NewBackendCapabilityReportRow {
            id: id.clone(),
            session_id: input.session_id,
            backend_kind: input.backend_kind,
            backend_version: input.backend_version,
            backend_binary_path_hash: input.backend_binary_path_hash,
            route_kind: input.route_kind,
            probe_status: input.probe_status,
            capture_strategy: input.capture_strategy,
            capture_semantics: input.capture_semantics,
            can_preserve_process_when_live: bool_to_int(input.can_preserve_process_when_live),
            can_capture_scrollback: bool_to_int(input.can_capture_scrollback),
            command_boundary_confidence: input.command_boundary_confidence,
            evidence_json,
            created_at_ms: now,
            expires_at_ms: input.expires_at_ms.unwrap_or(now + 24 * 60 * 60 * 1_000),
            stale_reason: None,
        };
        insert_into(terminal_backend_capability_reports::table)
            .values(&row)
            .execute(&mut connection)?;
        Ok(id)
    }

    pub fn import_saved_native_session_snapshot(
        &self,
        saved: &SavedNativeSession,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let lease = self.acquire_writer_generation("legacy-save-session", 60_000)?;
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
            last_event_seq: u64_to_i64(screen.sequence, "screen sequence")?,
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
                terminal_panes::last_event_seq.eq(row.last_event_seq),
                terminal_panes::metadata_json.eq(row.metadata_json.clone()),
            ))
            .execute(&mut connection)?;

        let cursor = NewStreamCursorRow {
            id: stream_cursor_id(&pane_id, &stream_id),
            session_id: saved.session_id.0.to_string(),
            pane_id,
            stream_id,
            next_event_seq: row.last_event_seq + 1,
            next_byte_seq: 0,
            updated_at_ms: now,
        };
        insert_into(terminal_stream_cursors::table)
            .values(&cursor)
            .on_conflict(terminal_stream_cursors::id)
            .do_update()
            .set((
                terminal_stream_cursors::next_event_seq.eq(cursor.next_event_seq),
                terminal_stream_cursors::updated_at_ms.eq(cursor.updated_at_ms),
            ))
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

            Ok(())
        })?;

        Ok(WriterGenerationLease {
            id,
            process_id,
            lease_token,
            lease_expires_at_ms: now + lease_ms,
        })
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
        diesel::update(
            terminal_writer_generations::table
                .filter(terminal_writer_generations::id.eq(writer_generation))
                .filter(terminal_writer_generations::state.eq("active")),
        )
        .set((
            terminal_writer_generations::heartbeat_at_ms.eq(now),
            terminal_writer_generations::lease_expires_at_ms.eq(now + lease_ms),
        ))
        .execute(&mut connection)?;
        Ok(())
    }

    pub fn release_writer_generation(
        &self,
        writer_generation: &str,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        diesel::update(
            terminal_writer_generations::table
                .filter(terminal_writer_generations::id.eq(writer_generation))
                .filter(terminal_writer_generations::state.eq("active")),
        )
        .set((
            terminal_writer_generations::state.eq("released"),
            terminal_writer_generations::released_at_ms.eq(Some(now)),
        ))
        .execute(&mut connection)?;
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

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let occurred_at_ms = input.occurred_at_ms.unwrap_or(now);
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let payload_len = checked_len(input.payload.len(), "payload length")?;
        let payload_checksum = blake3_hash_bytes(&input.payload);
        let metadata_json = json_metadata(&input.metadata)?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
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

            let event = NewJournalEventRow {
                id: event_id.clone(),
                session_id: input.session_id.clone(),
                pane_id: Some(input.pane_id.clone()),
                commit_id: commit.id.clone(),
                stream_id: stream_id.clone(),
                event_scope_kind: "pane".to_string(),
                event_scope_id: input.pane_id.clone(),
                event_seq: event_seq_low,
                event_type: input.event_type.unwrap_or_else(|| "terminal_output".to_string()),
                byte_low: Some(byte_low),
                byte_high: Some(byte_high),
                payload_json: input.payload_json.as_ref().map(serde_json::to_string).transpose()?,
                payload_schema_id: None,
                source_event_id_hash: input.source_event_id_hash,
                occurred_at_ms,
                created_at_ms: now,
                capture_semantics,
                trust_level: input.trust_level.unwrap_or_else(|| "captured".to_string()),
                metadata_json,
            };
            insert_into(terminal_journal_events::table).values(&event).execute(connection)?;

            advance_stream_cursor(connection, &cursor.id, event_seq_high + 1, byte_high, now)?;
            diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(&input.pane_id)))
                .set(terminal_panes::last_event_seq.eq(event_seq_high))
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
        })
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
        let metadata_json = json_metadata(&input.metadata)?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            ensure_active_writer(connection, &input.writer_generation, now)?;
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
                payload_schema_id: None,
                source_event_id_hash: input.source_event_id_hash,
                occurred_at_ms,
                created_at_ms: now,
                capture_semantics: input
                    .capture_semantics
                    .unwrap_or_else(|| "raw_vt_stream".to_string()),
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
        validate_optional_range(row.output_byte_low, row.output_byte_high, "command output byte")?;

        insert_into(terminal_command_blocks::table).values(&row).execute(&mut connection)?;
        Ok(id)
    }

    pub fn upsert_command_history_entry(
        &self,
        input: CommandHistoryEntryInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let command_hash = input.command_hash.unwrap_or_else(|| {
            blake3_hash_text(input.command_text.as_deref().unwrap_or(&input.display_text))
        });
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
            command_hash_algorithm: "blake3".to_string(),
            command_hash_scope: "local_profile".to_string(),
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
                payload_schema_id: None,
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

    pub fn restore_plan(
        &self,
        session_id: &str,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let latest_screen = terminal_screen_snapshots::table
            .filter(terminal_screen_snapshots::session_id.eq(session_id))
            .order(terminal_screen_snapshots::created_at_ms.desc())
            .select(ScreenSnapshotRow::as_select())
            .first::<ScreenSnapshotRow>(&mut connection)
            .optional()?;
        let latest_topology = terminal_topology_snapshots::table
            .filter(terminal_topology_snapshots::session_id.eq(session_id))
            .order(terminal_topology_snapshots::created_at_ms.desc())
            .select(TopologySnapshotRow::as_select())
            .first::<TopologySnapshotRow>(&mut connection)
            .optional()?;
        let segment_count: i64 = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .count()
            .get_result(&mut connection)?;
        let gap_count: i64 = terminal_journal_events::table
            .filter(terminal_journal_events::session_id.eq(session_id))
            .filter(terminal_journal_events::event_type.eq("history_gap"))
            .count()
            .get_result(&mut connection)?;

        let guarantee_level = match (segment_count > 0, latest_screen.is_some(), gap_count > 0) {
            (true, true, false) => RestoreGuaranteeLevel::BasicHistory,
            (true, _, true) => RestoreGuaranteeLevel::DegradedHistory,
            (false, true, _) => RestoreGuaranteeLevel::VisualSnapshotOnly,
            _ => RestoreGuaranteeLevel::None,
        };
        let high_water_commit_seq = terminal_commit_log::table
            .filter(terminal_commit_log::session_id.eq(session_id))
            .select(diesel::dsl::max(terminal_commit_log::commit_seq))
            .first::<Option<i64>>(&mut connection)?
            .unwrap_or(0);

        Ok(RestorePlan {
            session_id: session_id.to_string(),
            guarantee_level,
            latest_screen_snapshot_id: latest_screen.as_ref().map(|row| row.id.clone()),
            latest_topology_snapshot_id: latest_topology.as_ref().map(|row| row.id.clone()),
            high_water_commit_seq,
            evidence: vec![
                RestoreEvidence {
                    kind: "stream_segment_count".to_string(),
                    value: segment_count.to_string(),
                },
                RestoreEvidence {
                    kind: "history_gap_count".to_string(),
                    value: gap_count.to_string(),
                },
            ],
        })
    }

    pub fn record_restore_drill(
        &self,
        session_id: &str,
        plan: &RestorePlan,
        result: &str,
        duration_ms: Option<i64>,
        error: Option<&str>,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let id = new_id();
        let evidence_json = Some(serde_json::to_string(&plan.evidence)?);
        let row = NewRestoreDrillRow {
            id: id.clone(),
            session_id: session_id.to_string(),
            drill_kind: "restore_plan".to_string(),
            result: result.to_string(),
            restore_guarantee_level: plan.guarantee_level.as_str().to_string(),
            checked_at_ms: now,
            duration_ms,
            source_snapshot_id: plan.latest_screen_snapshot_id.clone(),
            evidence_json,
            error: error.map(ToOwned::to_owned),
            metadata_json: None,
        };
        insert_into(terminal_restore_drills::table).values(&row).execute(&mut connection)?;
        Ok(id)
    }

    pub fn list_stream_segments(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: i64,
        limit: i64,
    ) -> Result<Vec<StreamSegmentRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .filter(terminal_stream_segments::pane_id.eq(pane_id))
            .filter(terminal_stream_segments::event_seq_high.ge(from_event_seq))
            .order(terminal_stream_segments::event_seq_low.asc())
            .limit(limit)
            .select(StreamSegmentRow::as_select())
            .load::<StreamSegmentRow>(&mut connection)
            .map(|rows| rows.into_iter().map(StreamSegmentRecord::from).collect())
            .map_err(Into::into)
    }

    pub fn list_command_history(
        &self,
        session_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CommandHistoryEntryRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let mut query = terminal_command_history_entries::table.into_boxed();
        if let Some(session_id) = session_id {
            query = query.filter(terminal_command_history_entries::session_id.eq(session_id));
        }
        query
            .order(terminal_command_history_entries::last_used_at_ms.desc())
            .limit(limit)
            .select(CommandHistoryEntryRow::as_select())
            .load::<CommandHistoryEntryRow>(&mut connection)
            .map(|rows| rows.into_iter().map(CommandHistoryEntryRecord::from).collect())
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInput {
    pub id: Option<String>,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub source: Option<String>,
    pub durability_profile: Option<DurabilityProfile>,
    pub retention_policy_id: Option<String>,
    pub private_mode: bool,
    pub metadata: Option<Value>,
}

impl SessionInput {
    #[must_use]
    pub fn new(route: SessionRoute) -> Self {
        Self {
            id: None,
            route,
            title: None,
            launch: None,
            source: None,
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneInput {
    pub id: Option<String>,
    pub session_id: String,
    pub tab_id: Option<String>,
    pub stream_id: Option<String>,
    pub title: Option<String>,
    pub rows: i32,
    pub cols: i32,
    pub metadata: Option<Value>,
}

impl PaneInput {
    #[must_use]
    pub fn new(session_id: impl Into<String>, rows: i32, cols: i32) -> Self {
        Self {
            id: None,
            session_id: session_id.into(),
            tab_id: None,
            stream_id: None,
            title: None,
            rows,
            cols,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapabilityReportInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub backend_kind: String,
    pub backend_version: Option<String>,
    pub backend_binary_path_hash: Option<String>,
    pub route_kind: String,
    pub probe_status: String,
    pub capture_strategy: String,
    pub capture_semantics: String,
    pub can_preserve_process_when_live: bool,
    pub can_capture_scrollback: bool,
    pub command_boundary_confidence: String,
    pub evidence: Option<Value>,
    pub expires_at_ms: Option<i64>,
}

impl BackendCapabilityReportInput {
    #[must_use]
    pub fn from_backend_capabilities(
        backend_kind: BackendKind,
        route_kind: impl Into<String>,
        capabilities: &BackendCapabilities,
    ) -> Self {
        Self {
            id: None,
            session_id: None,
            backend_kind: format!("{backend_kind:?}").to_lowercase(),
            backend_version: None,
            backend_binary_path_hash: None,
            route_kind: route_kind.into(),
            probe_status: "passed".to_string(),
            capture_strategy: if capabilities.raw_output_stream {
                "raw_stream".to_string()
            } else if capabilities.rendered_viewport_stream {
                "rendered_stream".to_string()
            } else {
                "snapshot_only".to_string()
            },
            capture_semantics: if capabilities.raw_output_stream {
                "raw_vt_stream".to_string()
            } else {
                "rendered_plaintext_snapshot".to_string()
            },
            can_preserve_process_when_live: capabilities.explicit_session_restore,
            can_capture_scrollback: capabilities.rendered_scrollback_snapshot,
            command_boundary_confidence: "unknown".to_string(),
            evidence: None,
            expires_at_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSegmentInput {
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
    pub writer_generation: String,
    pub payload: Vec<u8>,
    pub event_type: Option<String>,
    pub event_count: i64,
    pub occurred_at_ms: Option<i64>,
    pub capture_semantics: Option<String>,
    pub trust_level: Option<String>,
    pub payload_json: Option<Value>,
    pub source_event_id_hash: Option<String>,
    pub metadata: Option<Value>,
}

impl StreamSegmentInput {
    #[must_use]
    pub fn terminal_output(
        session_id: impl Into<String>,
        pane_id: impl Into<String>,
        writer_generation: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            pane_id: pane_id.into(),
            stream_id: None,
            writer_generation: writer_generation.into(),
            payload: payload.into(),
            event_type: Some("terminal_output".to_string()),
            event_count: 1,
            occurred_at_ms: None,
            capture_semantics: None,
            trust_level: None,
            payload_json: None,
            source_event_id_hash: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEventInput {
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: Option<String>,
    pub writer_generation: String,
    pub event_type: String,
    pub commit_kind: Option<String>,
    pub payload_json: Option<Value>,
    pub source_event_id_hash: Option<String>,
    pub occurred_at_ms: Option<i64>,
    pub capture_semantics: Option<String>,
    pub trust_level: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBlockInput {
    pub id: Option<String>,
    pub session_id: String,
    pub pane_id: String,
    pub commit_id: Option<String>,
    pub command_text: Option<String>,
    pub display_text: Option<String>,
    pub redacted_text: Option<String>,
    pub command_text_source: Option<String>,
    pub trust_level: Option<String>,
    pub state: Option<String>,
    pub cwd: Option<String>,
    pub cwd_source: Option<String>,
    pub exit_code: Option<i32>,
    pub started_event_seq: Option<i64>,
    pub submitted_event_seq: Option<i64>,
    pub finished_event_seq: Option<i64>,
    pub output_event_seq_low: Option<i64>,
    pub output_event_seq_high: Option<i64>,
    pub output_byte_low: Option<i64>,
    pub output_byte_high: Option<i64>,
    pub sensitivity_class: Option<String>,
    pub created_at_ms: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntryInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub scope_kind: String,
    pub command_text: Option<String>,
    pub display_text: String,
    pub redacted_text: Option<String>,
    pub command_hash: Option<String>,
    pub cwd: Option<String>,
    pub shell_kind: Option<String>,
    pub trust_level: Option<String>,
    pub source: Option<String>,
    pub sensitivity_class: Option<String>,
    pub redaction_state: Option<String>,
    pub rerun_policy: Option<String>,
    pub first_used_at_ms: Option<i64>,
    pub last_used_at_ms: Option<i64>,
    pub use_count: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshotInput {
    pub id: Option<String>,
    pub session_id: String,
    pub pane_id: String,
    pub writer_generation: String,
    pub projection_source: Option<String>,
    pub buffer_kind: Option<String>,
    pub rows: i32,
    pub cols: i32,
    pub base_event_seq: i64,
    pub high_water_event_seq: i64,
    pub high_water_byte_seq: Option<i64>,
    pub screen: Value,
    pub parser_version: Option<String>,
    pub projection_version: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySnapshotInput {
    pub id: Option<String>,
    pub session_id: String,
    pub writer_generation: String,
    pub pane_high_water: Value,
    pub topology: Value,
    pub source: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreGuaranteeLevel {
    None,
    VisualSnapshotOnly,
    BasicHistory,
    DegradedHistory,
    RawStreamReplay,
    LiveMuxAttach,
}

impl RestoreGuaranteeLevel {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::VisualSnapshotOnly => "visual_snapshot_only",
            Self::BasicHistory => "basic_history",
            Self::DegradedHistory => "degraded_history",
            Self::RawStreamReplay => "raw_stream_replay",
            Self::LiveMuxAttach => "live_mux_attach",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreEvidence {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestorePlan {
    pub session_id: String,
    pub guarantee_level: RestoreGuaranteeLevel,
    pub latest_screen_snapshot_id: Option<String>,
    pub latest_topology_snapshot_id: Option<String>,
    pub high_water_commit_seq: i64,
    pub evidence: Vec<RestoreEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterGenerationLease {
    pub id: String,
    pub process_id: String,
    pub lease_token: String,
    pub lease_expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSegmentReceipt {
    pub commit_id: String,
    pub commit_seq: i64,
    pub segment_id: String,
    pub event_id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEventReceipt {
    pub commit_id: String,
    pub commit_seq: i64,
    pub event_id: String,
    pub event_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSegmentRecord {
    pub id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub payload: Vec<u8>,
    pub checksum: String,
}

impl From<StreamSegmentRow> for StreamSegmentRecord {
    fn from(row: StreamSegmentRow) -> Self {
        Self {
            id: row.id,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            payload: row.payload,
            checksum: row.checksum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandHistoryEntryRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub display_text: String,
    pub last_used_at_ms: i64,
    pub use_count: i64,
}

impl From<CommandHistoryEntryRow> for CommandHistoryEntryRecord {
    fn from(row: CommandHistoryEntryRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            display_text: row.display_text,
            last_used_at_ms: row.last_used_at_ms,
            use_count: row.use_count,
        }
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = terminal_db_identity)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TerminalDbIdentityRow {
    pub id: i32,
    pub product: String,
    pub schema_family: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub app_version: Option<String>,
    pub diesel_version: Option<String>,
    pub sqlite_version: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_feature_gates)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct FeatureGateRow {
    id: String,
    feature_name: String,
    state: String,
    rollout_scope: String,
    reason: Option<String>,
    enabled_at_ms: Option<i64>,
    disabled_at_ms: Option<i64>,
    updated_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_sessions)]
struct NewTerminalSessionRow {
    id: String,
    route_json: String,
    title: Option<String>,
    launch_json: Option<String>,
    source: String,
    durability_profile: String,
    retention_policy_id: String,
    private_mode: i32,
    created_at_ms: i64,
    updated_at_ms: i64,
    closed_at_ms: Option<i64>,
    state: String,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_panes)]
struct NewTerminalPaneRow {
    id: String,
    session_id: String,
    tab_id: Option<String>,
    stream_id: String,
    title: Option<String>,
    rows: i32,
    cols: i32,
    last_event_seq: i64,
    created_at_ms: i64,
    closed_at_ms: Option<i64>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_backend_capability_reports)]
struct NewBackendCapabilityReportRow {
    id: String,
    session_id: Option<String>,
    backend_kind: String,
    backend_version: Option<String>,
    backend_binary_path_hash: Option<String>,
    route_kind: String,
    probe_status: String,
    capture_strategy: String,
    capture_semantics: String,
    can_preserve_process_when_live: i32,
    can_capture_scrollback: i32,
    command_boundary_confidence: String,
    evidence_json: Option<String>,
    created_at_ms: i64,
    expires_at_ms: i64,
    stale_reason: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_writer_generations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct WriterGenerationRow {
    id: String,
    process_id: String,
    lease_token: String,
    state: String,
    acquired_at_ms: i64,
    heartbeat_at_ms: i64,
    lease_expires_at_ms: i64,
    released_at_ms: Option<i64>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_writer_generations)]
struct NewWriterGenerationRow {
    id: String,
    process_id: String,
    lease_token: String,
    state: String,
    acquired_at_ms: i64,
    heartbeat_at_ms: i64,
    lease_expires_at_ms: i64,
    released_at_ms: Option<i64>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_session_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct SessionCursorRow {
    session_id: String,
    next_commit_seq: i64,
    writer_generation: Option<String>,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_session_cursors)]
struct NewSessionCursorRow {
    session_id: String,
    next_commit_seq: i64,
    writer_generation: Option<String>,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_stream_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct StreamCursorRow {
    id: String,
    session_id: String,
    pane_id: String,
    stream_id: String,
    next_event_seq: i64,
    next_byte_seq: i64,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_stream_cursors)]
struct NewStreamCursorRow {
    id: String,
    session_id: String,
    pane_id: String,
    stream_id: String,
    next_event_seq: i64,
    next_byte_seq: i64,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_commit_log)]
struct NewCommitLogRow {
    id: String,
    session_id: String,
    commit_seq: i64,
    commit_kind: String,
    writer_generation: String,
    occurred_at_ms: i64,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone)]
struct CommitAllocation {
    id: String,
    commit_seq: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_stream_segments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct StreamSegmentRow {
    id: String,
    session_id: String,
    pane_id: String,
    commit_id: String,
    stream_id: String,
    event_seq_low: i64,
    event_seq_high: i64,
    byte_low: i64,
    byte_high: i64,
    payload: Vec<u8>,
    payload_len: i64,
    stored_byte_len: i64,
    uncompressed_byte_len: Option<i64>,
    checksum_algorithm: String,
    checksum: String,
    compression: String,
    capture_semantics: String,
    encryption_state: String,
    key_ref: Option<String>,
    created_at_ms: i64,
    writer_generation: String,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_stream_segments)]
struct NewStreamSegmentRow {
    id: String,
    session_id: String,
    pane_id: String,
    commit_id: String,
    stream_id: String,
    event_seq_low: i64,
    event_seq_high: i64,
    byte_low: i64,
    byte_high: i64,
    payload: Vec<u8>,
    payload_len: i64,
    stored_byte_len: i64,
    uncompressed_byte_len: Option<i64>,
    checksum_algorithm: String,
    checksum: String,
    compression: String,
    capture_semantics: String,
    encryption_state: String,
    key_ref: Option<String>,
    created_at_ms: i64,
    writer_generation: String,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_journal_events)]
struct NewJournalEventRow {
    id: String,
    session_id: String,
    pane_id: Option<String>,
    commit_id: String,
    stream_id: String,
    event_scope_kind: String,
    event_scope_id: String,
    event_seq: i64,
    event_type: String,
    byte_low: Option<i64>,
    byte_high: Option<i64>,
    payload_json: Option<String>,
    payload_schema_id: Option<String>,
    source_event_id_hash: Option<String>,
    occurred_at_ms: i64,
    created_at_ms: i64,
    capture_semantics: String,
    trust_level: String,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_command_blocks)]
struct NewCommandBlockRow {
    id: String,
    session_id: String,
    pane_id: String,
    commit_id: Option<String>,
    command_text: Option<String>,
    display_text: Option<String>,
    redacted_text: Option<String>,
    command_text_source: String,
    trust_level: String,
    state: String,
    cwd: Option<String>,
    cwd_source: Option<String>,
    exit_code: Option<i32>,
    started_event_seq: Option<i64>,
    submitted_event_seq: Option<i64>,
    finished_event_seq: Option<i64>,
    output_event_seq_low: Option<i64>,
    output_event_seq_high: Option<i64>,
    output_byte_low: Option<i64>,
    output_byte_high: Option<i64>,
    sensitivity_class: String,
    created_at_ms: i64,
    updated_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_command_history_entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct CommandHistoryEntryRow {
    id: String,
    session_id: Option<String>,
    pane_id: Option<String>,
    command_block_id: Option<String>,
    scope_kind: String,
    command_text: Option<String>,
    display_text: String,
    redacted_text: Option<String>,
    command_hash_algorithm: String,
    command_hash_scope: String,
    command_hash: String,
    cwd: Option<String>,
    shell_kind: Option<String>,
    trust_level: String,
    source: String,
    sensitivity_class: String,
    redaction_state: String,
    rerun_policy: String,
    first_used_at_ms: i64,
    last_used_at_ms: i64,
    use_count: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_command_history_entries)]
struct NewCommandHistoryEntryRow {
    id: String,
    session_id: Option<String>,
    pane_id: Option<String>,
    command_block_id: Option<String>,
    scope_kind: String,
    command_text: Option<String>,
    display_text: String,
    redacted_text: Option<String>,
    command_hash_algorithm: String,
    command_hash_scope: String,
    command_hash: String,
    cwd: Option<String>,
    shell_kind: Option<String>,
    trust_level: String,
    source: String,
    sensitivity_class: String,
    redaction_state: String,
    rerun_policy: String,
    first_used_at_ms: i64,
    last_used_at_ms: i64,
    use_count: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_screen_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct ScreenSnapshotRow {
    id: String,
    session_id: String,
    pane_id: String,
    commit_id: String,
    projection_source: String,
    buffer_kind: String,
    rows: i32,
    cols: i32,
    base_event_seq: i64,
    high_water_event_seq: i64,
    high_water_byte_seq: Option<i64>,
    screen_json: String,
    parser_version: String,
    projection_version: String,
    checksum_algorithm: String,
    checksum: String,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_screen_snapshots)]
struct NewScreenSnapshotRow {
    id: String,
    session_id: String,
    pane_id: String,
    commit_id: String,
    projection_source: String,
    buffer_kind: String,
    rows: i32,
    cols: i32,
    base_event_seq: i64,
    high_water_event_seq: i64,
    high_water_byte_seq: Option<i64>,
    screen_json: String,
    parser_version: String,
    projection_version: String,
    checksum_algorithm: String,
    checksum: String,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_topology_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct TopologySnapshotRow {
    id: String,
    session_id: String,
    commit_id: String,
    high_water_commit_seq: i64,
    pane_high_water_json: String,
    topology_json: String,
    payload_schema_id: Option<String>,
    checksum_algorithm: String,
    checksum: String,
    source: String,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_topology_snapshots)]
struct NewTopologySnapshotRow {
    id: String,
    session_id: String,
    commit_id: String,
    high_water_commit_seq: i64,
    pane_high_water_json: String,
    topology_json: String,
    payload_schema_id: Option<String>,
    checksum_algorithm: String,
    checksum: String,
    source: String,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_restore_drills)]
struct NewRestoreDrillRow {
    id: String,
    session_id: String,
    drill_kind: String,
    result: String,
    restore_guarantee_level: String,
    checked_at_ms: i64,
    duration_ms: Option<i64>,
    source_snapshot_id: Option<String>,
    evidence_json: Option<String>,
    error: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_db_identity)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct DbIdentityProbeRow {
    id: i32,
}

fn verify_seeded_defaults(
    connection: &mut SqliteConnection,
) -> Result<(), TerminalPersistenceV2Error> {
    let identity = terminal_db_identity::table
        .select(DbIdentityProbeRow::as_select())
        .first::<DbIdentityProbeRow>(connection)
        .optional()?;
    if identity.is_none() {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_db_identity was not initialized".to_string(),
        ));
    }

    let gate_count: i64 = terminal_feature_gates::table.count().get_result(connection)?;
    if gate_count == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_feature_gates seed rows are missing".to_string(),
        ));
    }

    Ok(())
}

fn allocate_commit(
    connection: &mut SqliteConnection,
    session_id: &str,
    commit_kind: &str,
    writer_generation: &str,
    occurred_at_ms: i64,
    created_at_ms: i64,
    metadata_json: Option<String>,
) -> Result<CommitAllocation, TerminalPersistenceV2Error> {
    let cursor = terminal_session_cursors::table
        .filter(terminal_session_cursors::session_id.eq(session_id))
        .select(SessionCursorRow::as_select())
        .first::<SessionCursorRow>(connection)?;
    let commit = CommitAllocation { id: new_id(), commit_seq: cursor.next_commit_seq };
    let row = NewCommitLogRow {
        id: commit.id.clone(),
        session_id: session_id.to_string(),
        commit_seq: commit.commit_seq,
        commit_kind: commit_kind.to_string(),
        writer_generation: writer_generation.to_string(),
        occurred_at_ms,
        created_at_ms,
        metadata_json,
    };

    insert_into(terminal_commit_log::table).values(&row).execute(connection)?;
    diesel::update(
        terminal_session_cursors::table.filter(terminal_session_cursors::session_id.eq(session_id)),
    )
    .set((
        terminal_session_cursors::next_commit_seq.eq(commit.commit_seq + 1),
        terminal_session_cursors::writer_generation.eq(Some(writer_generation.to_string())),
        terminal_session_cursors::updated_at_ms.eq(created_at_ms),
    ))
    .execute(connection)?;

    Ok(commit)
}

fn load_stream_cursor(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<StreamCursorRow, TerminalPersistenceV2Error> {
    terminal_stream_cursors::table
        .filter(terminal_stream_cursors::session_id.eq(session_id))
        .filter(terminal_stream_cursors::pane_id.eq(pane_id))
        .filter(terminal_stream_cursors::stream_id.eq(stream_id))
        .select(StreamCursorRow::as_select())
        .first::<StreamCursorRow>(connection)
        .map_err(Into::into)
}

fn advance_stream_cursor(
    connection: &mut SqliteConnection,
    cursor_id: &str,
    next_event_seq: i64,
    next_byte_seq: i64,
    updated_at_ms: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    diesel::update(
        terminal_stream_cursors::table.filter(terminal_stream_cursors::id.eq(cursor_id)),
    )
    .set((
        terminal_stream_cursors::next_event_seq.eq(next_event_seq),
        terminal_stream_cursors::next_byte_seq.eq(next_byte_seq),
        terminal_stream_cursors::updated_at_ms.eq(updated_at_ms),
    ))
    .execute(connection)?;
    Ok(())
}

fn ensure_active_writer(
    connection: &mut SqliteConnection,
    writer_generation: &str,
    now_ms: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let row = terminal_writer_generations::table
        .filter(terminal_writer_generations::id.eq(writer_generation))
        .filter(terminal_writer_generations::state.eq("active"))
        .select(WriterGenerationRow::as_select())
        .first::<WriterGenerationRow>(connection)?;
    if row.lease_expires_at_ms < now_ms {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "writer generation lease expired".to_string(),
        ));
    }
    Ok(())
}

fn map_writer_generation_insert_error(error: DieselError) -> TerminalPersistenceV2Error {
    match error {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            TerminalPersistenceV2Error::WriterAlreadyActive
        }
        other => other.into(),
    }
}

fn event_scope(session_id: &str, pane_id: Option<&str>) -> EventScope {
    match pane_id {
        Some(pane_id) => EventScope { kind: "pane".to_string(), id: pane_id.to_string() },
        None => EventScope { kind: "session".to_string(), id: session_id.to_string() },
    }
}

struct EventScope {
    kind: String,
    id: String,
}

fn validate_positive_dimensions(rows: i32, cols: i32) -> Result<(), TerminalPersistenceV2Error> {
    if rows <= 0 || cols <= 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "terminal dimensions must be positive, got rows={rows}, cols={cols}"
        )));
    }
    Ok(())
}

fn validate_optional_range(
    low: Option<i64>,
    high: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    match (low, high) {
        (Some(low), Some(high)) if low <= high => Ok(()),
        (None, None) => Ok(()),
        _ => Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} range must be either empty or fully populated"
        ))),
    }
}

fn checked_len(len: usize, label: &str) -> Result<i64, TerminalPersistenceV2Error> {
    i64::try_from(len).map_err(|_| {
        TerminalPersistenceV2Error::InvalidData(format!("{label} does not fit in i64"))
    })
}

fn u64_to_i64(value: u64, label: &str) -> Result<i64, TerminalPersistenceV2Error> {
    i64::try_from(value).map_err(|_| {
        TerminalPersistenceV2Error::InvalidData(format!("{label} does not fit in i64"))
    })
}

fn legacy_pane_high_water(saved: &SavedNativeSession) -> Value {
    let mut map = serde_json::Map::new();
    for screen in &saved.screens {
        map.insert(screen.pane_id.0.to_string(), Value::from(screen.sequence));
    }
    Value::Object(map)
}

fn stream_cursor_id(pane_id: &str, stream_id: &str) -> String {
    format!("stream-cursor-{}", blake3_hash_text(&format!("{pane_id}\0{stream_id}")))
}

fn stable_history_id(
    scope_kind: &str,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    command_hash: &str,
) -> String {
    let material = format!(
        "{}\0{}\0{}\0{}",
        scope_kind,
        session_id.unwrap_or_default(),
        pane_id.unwrap_or_default(),
        command_hash
    );
    format!("command-history-{}", blake3_hash_text(&material))
}

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn bool_to_int(value: bool) -> i32 {
    i32::from(value)
}

fn json_metadata(value: &Option<Value>) -> Result<Option<String>, TerminalPersistenceV2Error> {
    value.as_ref().map(serde_json::to_string).transpose().map_err(Into::into)
}

fn blake3_hash_bytes(value: &[u8]) -> String {
    blake3::hash(value).to_hex().to_string()
}

fn blake3_hash_text(value: &str) -> String {
    blake3_hash_bytes(value.as_bytes())
}

fn current_time_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => match i64::try_from(duration.as_millis()) {
            Ok(value) => value,
            Err(_) => i64::MAX,
        },
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_domain::{BackendKind, RouteAuthority, SessionRoute};

    fn test_store(label: &str) -> TerminalPersistenceV2 {
        let path = std::env::temp_dir()
            .join(format!("terminal-persistence-v2-{label}-{}.sqlite3", Uuid::new_v4()));
        TerminalPersistenceV2::open_with_config(path, TerminalPersistenceV2Config::test())
            .expect("v2 store should open")
    }

    fn route() -> SessionRoute {
        SessionRoute {
            backend: BackendKind::Native,
            authority: RouteAuthority::LocalDaemon,
            external: None,
        }
    }

    fn session_and_pane(store: &TerminalPersistenceV2) -> (String, String, WriterGenerationLease) {
        let session_id =
            store.create_session(SessionInput::new(route())).expect("session should save");
        let pane_id = store
            .create_pane(PaneInput::new(session_id.clone(), 24, 80))
            .expect("pane should save");
        let writer =
            store.acquire_writer_generation("test-process", 60_000).expect("writer should acquire");
        (session_id, pane_id, writer)
    }

    #[test]
    fn opens_db_and_seeds_feature_gates() {
        let store = test_store("seeds");

        assert_eq!(
            store
                .feature_gate_state(FeatureGateName::TerminalPersistenceV2Capture)
                .expect("gate should load"),
            FeatureGateState::Disabled
        );
    }

    #[test]
    fn creates_session_pane_and_reopens_with_history_cursor() {
        let store = test_store("session-pane");
        let path = store.path().to_path_buf();
        let session_id =
            store.create_session(SessionInput::new(route())).expect("session should save");
        let pane_id = store
            .create_pane(PaneInput::new(session_id.clone(), 30, 120))
            .expect("pane should save");

        let reopened =
            TerminalPersistenceV2::open_with_config(path, TerminalPersistenceV2Config::test())
                .expect("store should reopen");
        let mut connection = reopened.connection().expect("connection should open");
        let cursor = load_stream_cursor(&mut connection, &session_id, &pane_id, DEFAULT_STREAM_ID)
            .expect("stream cursor should exist");

        assert_eq!(cursor.next_event_seq, 1);
        assert_eq!(cursor.next_byte_seq, 0);
    }

    #[test]
    fn enforces_single_active_writer_generation() {
        let store = test_store("writer-generation");

        let first = store
            .acquire_writer_generation("process-a", 60_000)
            .expect("first writer should acquire");
        let second = store.acquire_writer_generation("process-b", 60_000);

        assert!(matches!(second, Err(TerminalPersistenceV2Error::WriterAlreadyActive)));
        store.release_writer_generation(&first.id).expect("writer should release");
        store
            .acquire_writer_generation("process-b", 60_000)
            .expect("new writer should acquire after release");
    }

    #[test]
    fn appends_raw_stream_segments_and_replays_after_reopen() {
        let store = test_store("stream");
        let path = store.path().to_path_buf();
        let (session_id, pane_id, writer) = session_and_pane(&store);

        let first = store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id.clone(),
                b"git status\r\n".to_vec(),
            ))
            .expect("first segment should persist");
        let second = store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id,
                b"fatal: not a git repository\r\n".to_vec(),
            ))
            .expect("second segment should persist");

        assert_eq!(first.event_seq_low, 1);
        assert_eq!(second.event_seq_low, 2);
        assert_eq!(second.byte_low, first.byte_high);

        let reopened =
            TerminalPersistenceV2::open_with_config(path, TerminalPersistenceV2Config::test())
                .expect("store should reopen");
        let segments = reopened
            .list_stream_segments(&session_id, &pane_id, 1, 10)
            .expect("segments should read");
        let payload: Vec<u8> = segments.into_iter().flat_map(|segment| segment.payload).collect();

        assert_eq!(payload, b"git status\r\nfatal: not a git repository\r\n");
    }

    #[test]
    fn persists_command_blocks_and_command_history() {
        let store = test_store("command-history");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        let output = store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id,
                b"hello\r\n".to_vec(),
            ))
            .expect("segment should persist");
        let block_id = store
            .write_command_block(CommandBlockInput {
                id: None,
                session_id: session_id.clone(),
                pane_id: pane_id.clone(),
                commit_id: Some(output.commit_id),
                command_text: Some("echo hello".to_string()),
                display_text: Some("echo hello".to_string()),
                redacted_text: None,
                command_text_source: None,
                trust_level: None,
                state: Some("finished".to_string()),
                cwd: Some("C:\\Users\\User".to_string()),
                cwd_source: Some("shell_integration".to_string()),
                exit_code: Some(0),
                started_event_seq: Some(1),
                submitted_event_seq: Some(1),
                finished_event_seq: Some(1),
                output_event_seq_low: Some(1),
                output_event_seq_high: Some(1),
                output_byte_low: Some(output.byte_low),
                output_byte_high: Some(output.byte_high),
                sensitivity_class: None,
                created_at_ms: None,
                metadata: None,
            })
            .expect("command block should persist");
        let history_id = store
            .upsert_command_history_entry(CommandHistoryEntryInput {
                id: None,
                session_id: Some(session_id.clone()),
                pane_id: Some(pane_id.clone()),
                command_block_id: Some(block_id),
                scope_kind: "session".to_string(),
                command_text: Some("echo hello".to_string()),
                display_text: "echo hello".to_string(),
                redacted_text: None,
                command_hash: None,
                cwd: Some("C:\\Users\\User".to_string()),
                shell_kind: Some("cmd".to_string()),
                trust_level: None,
                source: None,
                sensitivity_class: None,
                redaction_state: None,
                rerun_policy: None,
                first_used_at_ms: None,
                last_used_at_ms: None,
                use_count: None,
                metadata: None,
            })
            .expect("history should persist");

        let listed =
            store.list_command_history(Some(&session_id), 10).expect("history should list");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, history_id);
        assert_eq!(listed[0].display_text, "echo hello");
        assert_eq!(listed[0].use_count, 1);
    }

    #[test]
    fn restore_plan_uses_snapshots_and_stream_evidence() {
        let store = test_store("restore-plan");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id.clone(),
                b"visible history\r\n".to_vec(),
            ))
            .expect("segment should persist");
        let screen_id = store
            .write_screen_snapshot(ScreenSnapshotInput {
                id: None,
                session_id: session_id.clone(),
                pane_id,
                writer_generation: writer.id.clone(),
                projection_source: None,
                buffer_kind: None,
                rows: 24,
                cols: 80,
                base_event_seq: 1,
                high_water_event_seq: 1,
                high_water_byte_seq: Some(17),
                screen: serde_json::json!({"lines":["visible history"]}),
                parser_version: None,
                projection_version: None,
                metadata: None,
            })
            .expect("screen snapshot should persist");
        let topology_id = store
            .write_topology_snapshot(TopologySnapshotInput {
                id: None,
                session_id: session_id.clone(),
                writer_generation: writer.id,
                pane_high_water: serde_json::json!({}),
                topology: serde_json::json!({"tabs":[]}),
                source: None,
                metadata: None,
            })
            .expect("topology snapshot should persist");

        let plan = store.restore_plan(&session_id).expect("restore plan should load");

        assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::BasicHistory);
        assert_eq!(plan.latest_screen_snapshot_id, Some(screen_id));
        assert_eq!(plan.latest_topology_snapshot_id, Some(topology_id));
        assert!(plan.high_water_commit_seq >= 3);
    }

    #[test]
    fn canonical_history_prevents_parent_delete() {
        let store = test_store("restrict-delete");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id,
                writer.id,
                b"must stay durable\r\n".to_vec(),
            ))
            .expect("segment should persist");

        let mut connection = store.connection().expect("connection should open");
        let delete_result =
            diesel::delete(terminal_sessions::table.filter(terminal_sessions::id.eq(&session_id)))
                .execute(&mut connection);

        assert!(delete_result.is_err());
    }
}
