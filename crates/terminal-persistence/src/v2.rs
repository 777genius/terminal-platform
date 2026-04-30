use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper,
    dsl::{insert_into, max},
    prelude::*,
    result::{DatabaseErrorKind, Error as DieselError},
    sqlite::SqliteConnection,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use terminal_backend_api::{BackendCapabilities, ShellLaunchSpec};
use terminal_domain::{BackendKind, SessionRoute};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    db::{
        connection::establish_initialized_connection,
        schema::{
            terminal_backend_capability_reports, terminal_backup_records,
            terminal_capture_receipts, terminal_clients, terminal_command_blocks,
            terminal_command_history_entries, terminal_commit_log, terminal_data_health_records,
            terminal_db_identity, terminal_delete_requests, terminal_deletion_tombstones,
            terminal_delivery_offsets, terminal_export_requests, terminal_feature_gates,
            terminal_history_gaps, terminal_integrity_checks, terminal_journal_events,
            terminal_outbox_messages, terminal_panes, terminal_restore_drills,
            terminal_screen_snapshots, terminal_search_documents, terminal_session_cursors,
            terminal_sessions, terminal_storage_pressure_events, terminal_stream_cursors,
            terminal_stream_segments, terminal_support_bundles, terminal_topology_snapshots,
            terminal_writer_generations,
        },
    },
    legacy::SavedNativeSession,
};

pub const TERMINAL_PERSISTENCE_APP_ID: i32 = 0x5450_5632;
const DEFAULT_RETENTION_POLICY_ID: &str = "default_full_history";
const DEFAULT_STREAM_ID: &str = "primary";
const DEFAULT_HISTORY_SEGMENT_LIMIT: i64 = 256;
const MAX_HISTORY_SEGMENT_LIMIT: i64 = 2_000;
const DEFAULT_HISTORY_BYTE_LIMIT: i64 = 1024 * 1024;
const MAX_HISTORY_BYTE_LIMIT: i64 = 16 * 1024 * 1024;
const MAX_HISTORY_GAP_LIMIT: i64 = 256;
const DEFAULT_COMMAND_HISTORY_LIMIT: i64 = 100;
const MAX_COMMAND_HISTORY_LIMIT: i64 = 1_000;

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
    #[error("terminal persistence executor stopped")]
    ExecutorStopped,
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

    pub fn upsert_runtime_session(
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
            insert_into(terminal_sessions::table)
                .values(&row)
                .on_conflict(terminal_sessions::id)
                .do_update()
                .set((
                    terminal_sessions::route_json.eq(row.route_json.clone()),
                    terminal_sessions::title.eq(row.title.clone()),
                    terminal_sessions::launch_json.eq(row.launch_json.clone()),
                    terminal_sessions::source.eq(row.source.clone()),
                    terminal_sessions::durability_profile.eq(row.durability_profile.clone()),
                    terminal_sessions::private_mode.eq(row.private_mode),
                    terminal_sessions::updated_at_ms.eq(row.updated_at_ms),
                    terminal_sessions::state.eq(row.state.clone()),
                    terminal_sessions::metadata_json.eq(row.metadata_json.clone()),
                ))
                .execute(connection)?;

            let cursor = NewSessionCursorRow {
                session_id: session_id.clone(),
                next_commit_seq: 1,
                writer_generation: None,
                updated_at_ms: now,
            };
            insert_into(terminal_session_cursors::table)
                .values(&cursor)
                .on_conflict(terminal_session_cursors::session_id)
                .do_nothing()
                .execute(connection)?;

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

    pub fn upsert_runtime_pane(
        &self,
        input: PaneInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
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
                .execute(connection)?;

            let cursor = NewStreamCursorRow {
                id: stream_cursor_id(&pane_id, &stream_id),
                session_id: input.session_id,
                pane_id: pane_id.clone(),
                stream_id,
                next_event_seq: 1,
                next_byte_seq: 0,
                updated_at_ms: now,
            };
            insert_into(terminal_stream_cursors::table)
                .values(&cursor)
                .on_conflict(terminal_stream_cursors::id)
                .do_nothing()
                .execute(connection)?;

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

        let lease = self.acquire_writer_generation_with_retry("runtime-ui-input", 60_000)?;
        let event_result = self.append_ui_input_event_and_command(&input, &lease.id);
        let release_result = self.release_writer_generation(&lease.id);

        match (event_result, release_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    fn append_ui_input_event_and_command(
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
        let command_metadata_json = Some(serde_json::to_string(&serde_json::json!({
            "capture_source": "ui_input",
            "rerun_policy": "confirm"
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
                payload_schema_id: None,
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

                let command_hash = blake3_hash_text(command_text);
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
                    command_hash_algorithm: "blake3".to_string(),
                    command_hash_scope: "local_profile".to_string(),
                    command_hash,
                    cwd: None,
                    shell_kind: input.shell_kind.clone(),
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

    fn append_history_gap_event(
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
                payload_schema_id: None,
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

        let pane_high_water = topology_pane_high_water(&input.topology);
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
        let source_event_id_hash = input.source_event_id_hash.clone();
        let capture_source_kind = source_event_id_hash
            .as_ref()
            .map(|_| stream_capture_source_kind(&input.pane_id, &stream_id));

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
                source_event_id_hash: source_event_id_hash.clone(),
                occurred_at_ms,
                created_at_ms: now,
                capture_semantics,
                trust_level: input.trust_level.unwrap_or_else(|| "captured".to_string()),
                metadata_json: metadata_json.clone(),
            };
            insert_into(terminal_journal_events::table).values(&event).execute(connection)?;

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
                    "event_seq_high": event_seq_high,
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

        let mut guarantee_level = match (segment_count > 0, latest_screen.is_some(), gap_count > 0)
        {
            (true, true, false) => RestoreGuaranteeLevel::BasicHistory,
            (_, _, true) => RestoreGuaranteeLevel::DegradedHistory,
            (false, true, _) => RestoreGuaranteeLevel::VisualSnapshotOnly,
            _ => RestoreGuaranteeLevel::None,
        };
        let high_water_commit_seq = terminal_commit_log::table
            .filter(terminal_commit_log::session_id.eq(session_id))
            .select(diesel::dsl::max(terminal_commit_log::commit_seq))
            .first::<Option<i64>>(&mut connection)?
            .unwrap_or(0);
        let latest_restore_drill_status = terminal_restore_drills::table
            .filter(terminal_restore_drills::session_id.eq(session_id))
            .order(terminal_restore_drills::checked_at_ms.desc())
            .select(terminal_restore_drills::result)
            .first::<String>(&mut connection)
            .optional()?;
        if matches!(latest_restore_drill_status.as_deref(), Some("failed" | "degraded")) {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        let authoritative_reads_gate = terminal_feature_gates::table
            .filter(
                terminal_feature_gates::feature_name
                    .eq(FeatureGateName::TerminalPersistenceV2AuthoritativeReads.as_str()),
            )
            .select(terminal_feature_gates::state)
            .first::<String>(&mut connection)
            .optional()?
            .unwrap_or_else(|| FeatureGateState::Disabled.as_str().to_string());
        if authoritative_reads_gate == FeatureGateState::ForceDisabled.as_str() {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }

        let mut evidence = vec![
            RestoreEvidence {
                kind: "stream_segment_count".to_string(),
                value: segment_count.to_string(),
            },
            RestoreEvidence { kind: "history_gap_count".to_string(), value: gap_count.to_string() },
            RestoreEvidence {
                kind: "authoritative_reads_gate_state".to_string(),
                value: authoritative_reads_gate,
            },
        ];
        if let Some(status) = &latest_restore_drill_status {
            evidence.push(RestoreEvidence {
                kind: "latest_restore_drill_status".to_string(),
                value: status.clone(),
            });
        }

        Ok(RestorePlan {
            session_id: session_id.to_string(),
            guarantee_level,
            latest_screen_snapshot_id: latest_screen.as_ref().map(|row| row.id.clone()),
            latest_topology_snapshot_id: latest_topology.as_ref().map(|row| row.id.clone()),
            high_water_commit_seq,
            latest_restore_drill_status,
            evidence,
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

    pub fn run_restore_drill(
        &self,
        session_id: &str,
    ) -> Result<RestoreDrillRecord, TerminalPersistenceV2Error> {
        let started_at_ms = self.config.clock.now_ms();
        let plan = self.restore_plan(session_id)?;
        let mut connection = self.connection()?;
        let validation = validate_history_checksums(&mut connection, Some(session_id))?;
        let finished_at_ms = self.config.clock.now_ms();
        let result =
            if validation.has_failures() {
                "failed"
            } else {
                match &plan.guarantee_level {
                    RestoreGuaranteeLevel::BasicHistory
                    | RestoreGuaranteeLevel::VisualSnapshotOnly => "passed",
                    RestoreGuaranteeLevel::RawStreamReplay
                    | RestoreGuaranteeLevel::LiveMuxAttach => "passed",
                    RestoreGuaranteeLevel::DegradedHistory => "degraded",
                    RestoreGuaranteeLevel::None => "skipped",
                }
            };
        let error = validation.has_failures().then(|| validation.summary());
        let mut evidence = plan.evidence.clone();
        evidence.extend(validation.to_restore_evidence());
        let evidence_json = Some(serde_json::to_string(&evidence)?);
        let metadata_json = Some(serde_json::to_string(&serde_json::json!({
            "started_at_ms": started_at_ms,
            "validation": validation.to_json(),
        }))?);
        let id = new_id();
        let row = NewRestoreDrillRow {
            id: id.clone(),
            session_id: session_id.to_string(),
            drill_kind: "restore_drill".to_string(),
            result: result.to_string(),
            restore_guarantee_level: plan.guarantee_level.as_str().to_string(),
            checked_at_ms: finished_at_ms,
            duration_ms: Some((finished_at_ms - started_at_ms).max(0)),
            source_snapshot_id: plan.latest_screen_snapshot_id.clone(),
            evidence_json,
            error: error.clone(),
            metadata_json,
        };
        insert_into(terminal_restore_drills::table).values(&row).execute(&mut connection)?;
        persist_history_validation_health_records(
            &mut connection,
            Some(session_id),
            &validation,
            finished_at_ms,
            Some(&id),
        )?;

        Ok(RestoreDrillRecord {
            id,
            session_id: session_id.to_string(),
            drill_kind: "restore_drill".to_string(),
            result: result.to_string(),
            restore_guarantee_level: plan.guarantee_level.as_str().to_string(),
            checked_at_ms: finished_at_ms,
            duration_ms: Some((finished_at_ms - started_at_ms).max(0)),
            source_snapshot_id: plan.latest_screen_snapshot_id,
            error,
        })
    }

    pub fn run_integrity_check(&self) -> Result<IntegrityCheckRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let checked_at_ms = self.config.clock.now_ms();
        let quick_check = run_quick_check(&mut connection)?;
        let foreign_key_violations = run_foreign_key_check(&mut connection)?;
        let validation = validate_history_checksums(&mut connection, None)?;
        let result = if quick_check.iter().all(|value| value == "ok")
            && foreign_key_violations.is_empty()
            && !validation.has_failures()
        {
            "passed"
        } else {
            "failed"
        };
        let details = serde_json::json!({
            "quick_check": quick_check,
            "foreign_key_violations": foreign_key_violations,
            "history_validation": validation.to_json(),
        });
        let error = (result != "passed").then(|| {
            format!(
                "quick_check={}, foreign_key_violations={}, checksum_failures={}",
                details["quick_check"],
                details["foreign_key_violations"].as_array().map_or(0, Vec::len),
                validation.failure_count()
            )
        });
        let id = new_id();
        let row = NewIntegrityCheckRow {
            id: id.clone(),
            check_kind: "sqlite_and_history_invariants".to_string(),
            scope_kind: "database".to_string(),
            scope_ref: None,
            result: result.to_string(),
            checked_at_ms,
            details_json: Some(serde_json::to_string(&details)?),
            error: error.clone(),
            metadata_json: None,
        };
        insert_into(terminal_integrity_checks::table).values(&row).execute(&mut connection)?;
        persist_history_validation_health_records(
            &mut connection,
            None,
            &validation,
            checked_at_ms,
            Some(&id),
        )?;

        Ok(IntegrityCheckRecord {
            id,
            check_kind: "sqlite_and_history_invariants".to_string(),
            scope_kind: "database".to_string(),
            scope_ref: None,
            result: result.to_string(),
            checked_at_ms,
            details_json: Some(details),
            error,
        })
    }

    pub fn list_open_data_health_records(
        &self,
        session_id: Option<&str>,
    ) -> Result<Vec<DataHealthRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let mut query = terminal_data_health_records::table
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .into_boxed();
        if let Some(session_id) = session_id {
            query = query.filter(terminal_data_health_records::session_id.eq(session_id));
        }
        query
            .order(terminal_data_health_records::detected_at_ms.desc())
            .select(DataHealthRecordRow::as_select())
            .load::<DataHealthRecordRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub fn record_storage_pressure_event(
        &self,
        input: StoragePressureEventInput,
    ) -> Result<StoragePressureRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewStoragePressureEventRow {
            id: input.id.unwrap_or_else(new_id),
            state: input.state.unwrap_or_else(|| "active".to_string()),
            db_file_bytes: input.db_file_bytes,
            wal_file_bytes: input.wal_file_bytes,
            disk_free_bytes: input.disk_free_bytes,
            temp_free_bytes: input.temp_free_bytes,
            quota_bytes: input.quota_bytes,
            action_taken: input.action_taken.unwrap_or_else(|| "warn_only".to_string()),
            reason: input.reason,
            created_at_ms: now,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_storage_pressure_events::table)
            .values(&row)
            .execute(&mut connection)?;
        Ok(StoragePressureRecord::from(row))
    }

    pub fn probe_storage_health(
        &self,
    ) -> Result<StoragePressureRecord, TerminalPersistenceV2Error> {
        let db_file_bytes = fs::metadata(&self.path)
            .ok()
            .map(|metadata| metadata.len())
            .map(|len| u64_to_i64(len, "database file size"))
            .transpose()?;
        let wal_path = sqlite_sidecar_path(&self.path, "-wal");
        let wal_file_bytes = fs::metadata(&wal_path)
            .ok()
            .map(|metadata| metadata.len())
            .map(|len| u64_to_i64(len, "wal file size"))
            .transpose()?;

        self.record_storage_pressure_event(StoragePressureEventInput {
            id: None,
            state: Some("ok".to_string()),
            db_file_bytes,
            wal_file_bytes,
            disk_free_bytes: None,
            temp_free_bytes: None,
            quota_bytes: None,
            action_taken: Some("none".to_string()),
            reason: Some("manual_probe".to_string()),
            metadata: Some(serde_json::json!({
                "db_path_hash": path_hash(&self.path),
                "wal_path_hash": path_hash(&wal_path),
            })),
        })
    }

    pub fn create_delete_request(
        &self,
        input: DeleteRequestInput,
    ) -> Result<DeleteRequestRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewDeleteRequestRow {
            id: input.id.unwrap_or_else(new_id),
            session_id: input.session_id,
            request_kind: input.request_kind.unwrap_or_else(|| "user_delete".to_string()),
            state: "pending".to_string(),
            policy_id: input.policy_id,
            requested_at_ms: now,
            approved_at_ms: None,
            completed_at_ms: None,
            requester_ref_hash: input.requester_ref.map(|value| blake3_hash_text(&value)),
            reason: input.reason,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_delete_requests::table).values(&row).execute(&mut connection)?;
        Ok(DeleteRequestRecord::try_from(row)?)
    }

    pub fn complete_delete_request_with_tombstone(
        &self,
        delete_request_id: &str,
        deleted_scope: &str,
        evidence: Option<Value>,
        metadata: Option<Value>,
    ) -> Result<DeletionTombstoneRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let request = terminal_delete_requests::table
                .filter(terminal_delete_requests::id.eq(delete_request_id))
                .select(DeleteRequestRow::as_select())
                .first::<DeleteRequestRow>(connection)?;
            if request.state == "completed" {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "delete request is already completed".to_string(),
                ));
            }

            diesel::update(
                terminal_delete_requests::table
                    .filter(terminal_delete_requests::id.eq(delete_request_id)),
            )
            .set((
                terminal_delete_requests::state.eq("completed"),
                terminal_delete_requests::completed_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;

            let row = NewDeletionTombstoneRow {
                id: new_id(),
                delete_request_id: Some(delete_request_id.to_string()),
                session_id: request.session_id,
                deleted_scope: deleted_scope.to_string(),
                policy_id: request.policy_id,
                deleted_at_ms: now,
                evidence_json: evidence.as_ref().map(serde_json::to_string).transpose()?,
                metadata_json: json_metadata(&metadata)?,
            };
            insert_into(terminal_deletion_tombstones::table).values(&row).execute(connection)?;
            Ok(DeletionTombstoneRecord::try_from(row)?)
        })
    }

    pub fn create_export_request(
        &self,
        input: ExportRequestInput,
    ) -> Result<ExportRequestRecord, TerminalPersistenceV2Error> {
        if input.include_raw {
            self.ensure_raw_history_export_enabled()?;
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let manifest = privacy_manifest("export", input.include_raw, input.session_id.as_deref());
        let row = NewExportRequestRow {
            id: input.id.unwrap_or_else(new_id),
            session_id: input.session_id,
            export_kind: input.export_kind.unwrap_or_else(|| "redacted_logical".to_string()),
            state: "pending".to_string(),
            redaction_profile_id: input
                .redaction_profile_id
                .or_else(|| Some("default".to_string())),
            include_raw: bool_to_int(input.include_raw),
            approved_at_ms: None,
            requested_at_ms: now,
            completed_at_ms: None,
            manifest_json: Some(serde_json::to_string(&manifest)?),
            output_ref_hash: input.output_ref.map(|value| blake3_hash_text(&value)),
            error: None,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_export_requests::table).values(&row).execute(&mut connection)?;
        Ok(ExportRequestRecord::try_from(row)?)
    }

    pub fn create_support_bundle(
        &self,
        input: SupportBundleInput,
    ) -> Result<SupportBundleRecord, TerminalPersistenceV2Error> {
        if input.include_raw {
            self.ensure_raw_history_export_enabled()?;
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let manifest = privacy_manifest("support_bundle", input.include_raw, None);
        let row = NewSupportBundleRow {
            id: input.id.unwrap_or_else(new_id),
            scope_json: serde_json::to_string(&input.scope)?,
            state: "pending".to_string(),
            redaction_profile_id: input
                .redaction_profile_id
                .or_else(|| Some("default".to_string())),
            include_raw: bool_to_int(input.include_raw),
            requested_at_ms: now,
            completed_at_ms: None,
            manifest_json: Some(serde_json::to_string(&manifest)?),
            output_ref_hash: input.output_ref.map(|value| blake3_hash_text(&value)),
            error: None,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_support_bundles::table).values(&row).execute(&mut connection)?;
        Ok(SupportBundleRecord::try_from(row)?)
    }

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

    pub fn vacuum_into_backup(
        &self,
        target_path: impl AsRef<Path>,
    ) -> Result<BackupRecord, TerminalPersistenceV2Error> {
        let target_path = target_path.as_ref().to_path_buf();
        if target_path.exists() {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "backup target already exists: {}",
                target_path.display()
            )));
        }
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let id = new_id();
        let started_at_ms = self.config.clock.now_ms();
        let target_ref_hash = path_hash(&target_path);
        let source_db_path_hash = path_hash(&self.path);
        let mut connection = self.connection()?;
        let running = NewBackupRecordRow {
            id: id.clone(),
            backup_kind: "vacuum_into".to_string(),
            state: "running".to_string(),
            target_ref_hash: Some(target_ref_hash.clone()),
            manifest_json: None,
            checksum_algorithm: None,
            checksum: None,
            source_db_path_hash: Some(source_db_path_hash.clone()),
            started_at_ms,
            finished_at_ms: None,
            quick_check_result: None,
            error: None,
            metadata_json: None,
        };
        insert_into(terminal_backup_records::table).values(&running).execute(&mut connection)?;

        let backup_result = self.finish_vacuum_into_backup(
            &id,
            &target_path,
            started_at_ms,
            target_ref_hash,
            source_db_path_hash,
        );
        if let Err(error) = &backup_result {
            let _ = self.mark_backup_failed(&id, error.to_string());
        }
        backup_result
    }

    fn finish_vacuum_into_backup(
        &self,
        id: &str,
        target_path: &Path,
        started_at_ms: i64,
        target_ref_hash: String,
        source_db_path_hash: String,
    ) -> Result<BackupRecord, TerminalPersistenceV2Error> {
        let target_arg = target_path.to_str().ok_or_else(|| {
            TerminalPersistenceV2Error::InvalidData("backup target path is not UTF-8".to_string())
        })?;
        let mut vacuum_connection = self.connection()?;
        diesel::sql_query("VACUUM INTO ?")
            .bind::<diesel::sql_types::Text, _>(target_arg.to_string())
            .execute(&mut vacuum_connection)?;

        let checksum = blake3_hash_file(target_path)?;
        let file_bytes = u64_to_i64(fs::metadata(target_path)?.len(), "backup file size")?;
        let mut backup_connection = establish_initialized_connection(target_path, &self.config)?;
        let quick_check = run_quick_check(&mut backup_connection)?;
        let quick_check_result = quick_check.join("; ");
        if !quick_check.iter().all(|value| value == "ok") {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "backup quick_check failed: {quick_check_result}"
            )));
        }

        let finished_at_ms = self.config.clock.now_ms();
        let manifest = serde_json::json!({
            "backup_kind": "vacuum_into",
            "file_bytes": file_bytes,
            "target_ref_hash": target_ref_hash,
            "source_db_path_hash": source_db_path_hash,
            "checksum_algorithm": "blake3",
            "checksum": checksum,
            "quick_check_result": quick_check_result,
            "started_at_ms": started_at_ms,
            "finished_at_ms": finished_at_ms,
        });
        let manifest_json = serde_json::to_string(&manifest)?;

        let mut connection = self.connection()?;
        diesel::update(terminal_backup_records::table.filter(terminal_backup_records::id.eq(id)))
            .set((
                terminal_backup_records::state.eq("succeeded"),
                terminal_backup_records::manifest_json.eq(Some(manifest_json.clone())),
                terminal_backup_records::checksum_algorithm.eq(Some("blake3".to_string())),
                terminal_backup_records::checksum.eq(Some(checksum.clone())),
                terminal_backup_records::finished_at_ms.eq(Some(finished_at_ms)),
                terminal_backup_records::quick_check_result.eq(Some(quick_check_result.clone())),
                terminal_backup_records::error.eq::<Option<String>>(None),
            ))
            .execute(&mut connection)?;

        Ok(BackupRecord {
            id: id.to_string(),
            backup_kind: "vacuum_into".to_string(),
            state: "succeeded".to_string(),
            target_ref_hash: Some(target_ref_hash),
            manifest_json: Some(manifest),
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(checksum),
            source_db_path_hash: Some(source_db_path_hash),
            started_at_ms,
            finished_at_ms: Some(finished_at_ms),
            quick_check_result: Some(quick_check_result),
            error: None,
        })
    }

    fn mark_backup_failed(
        &self,
        id: &str,
        error: String,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        diesel::update(terminal_backup_records::table.filter(terminal_backup_records::id.eq(id)))
            .set((
                terminal_backup_records::state.eq("failed"),
                terminal_backup_records::finished_at_ms.eq(Some(self.config.clock.now_ms())),
                terminal_backup_records::error.eq(Some(error)),
            ))
            .execute(&mut connection)?;
        Ok(())
    }

    fn ensure_raw_history_export_enabled(&self) -> Result<(), TerminalPersistenceV2Error> {
        match self.feature_gate_state(FeatureGateName::RawHistoryExport)? {
            FeatureGateState::Enabled => Ok(()),
            other => Err(TerminalPersistenceV2Error::InvalidData(format!(
                "raw history export is disabled by feature gate: {}",
                other.as_str()
            ))),
        }
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

    pub fn hydrate_pane_history(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let from_event_seq = from_event_seq.unwrap_or(1).max(1);
        let max_segments = max_segments
            .unwrap_or(DEFAULT_HISTORY_SEGMENT_LIMIT)
            .clamp(1, MAX_HISTORY_SEGMENT_LIMIT);
        let max_bytes =
            max_bytes.unwrap_or(DEFAULT_HISTORY_BYTE_LIMIT).clamp(1, MAX_HISTORY_BYTE_LIMIT);

        let latest_screen_snapshot = terminal_screen_snapshots::table
            .filter(terminal_screen_snapshots::session_id.eq(session_id))
            .filter(terminal_screen_snapshots::pane_id.eq(pane_id))
            .order((
                terminal_screen_snapshots::high_water_event_seq.desc(),
                terminal_screen_snapshots::created_at_ms.desc(),
            ))
            .select(ScreenSnapshotRow::as_select())
            .first::<ScreenSnapshotRow>(&mut connection)
            .optional()?
            .map(ScreenSnapshotRecord::from);

        let fetched_segments = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .filter(terminal_stream_segments::pane_id.eq(pane_id))
            .filter(terminal_stream_segments::event_seq_high.ge(from_event_seq))
            .order(terminal_stream_segments::event_seq_low.asc())
            .limit(max_segments + 1)
            .select(StreamSegmentRow::as_select())
            .load::<StreamSegmentRow>(&mut connection)?;

        let mut segments = Vec::new();
        let mut total_payload_bytes = 0_i64;
        let mut has_more_segments = fetched_segments.len() > max_segments as usize;
        for row in fetched_segments.into_iter().take(max_segments as usize) {
            let row_payload_bytes = row.payload_len.max(0);
            if total_payload_bytes > 0 && total_payload_bytes + row_payload_bytes > max_bytes {
                has_more_segments = true;
                break;
            }
            total_payload_bytes += row_payload_bytes;
            segments.push(StreamSegmentRecord::from(row));
        }

        let gaps = terminal_history_gaps::table
            .filter(terminal_history_gaps::session_id.eq(session_id))
            .filter(
                terminal_history_gaps::pane_id
                    .is_null()
                    .or(terminal_history_gaps::pane_id.eq(pane_id)),
            )
            .order(terminal_history_gaps::opened_at_ms.asc())
            .limit(MAX_HISTORY_GAP_LIMIT)
            .select(HistoryGapRow::as_select())
            .load::<HistoryGapRow>(&mut connection)?
            .into_iter()
            .map(HistoryGapRecord::from)
            .collect::<Vec<_>>();

        let restore_plan = self.restore_plan(session_id)?;
        let replay_strategy = PaneHistoryReplayStrategy::from_evidence(
            &segments,
            latest_screen_snapshot.as_ref(),
            &gaps,
        );
        let next_event_seq = segments.last().map(|segment| segment.event_seq_high + 1);

        Ok(PaneHistoryHydrationRecord {
            session_id: session_id.to_string(),
            pane_id: pane_id.to_string(),
            from_event_seq,
            max_segments,
            max_bytes,
            restore_plan,
            latest_screen_snapshot,
            segments,
            gaps,
            replay_strategy,
            has_more_segments,
            next_event_seq,
            total_payload_bytes,
        })
    }

    pub fn list_command_history(
        &self,
        session_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CommandHistoryEntryRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let limit = if limit <= 0 {
            DEFAULT_COMMAND_HISTORY_LIMIT
        } else {
            limit.min(MAX_COMMAND_HISTORY_LIMIT)
        };
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
pub struct DeliveryClientInput {
    pub id: Option<String>,
    pub client_kind: String,
    pub install_ref_hash: Option<String>,
    pub browser_profile_ref_hash: Option<String>,
    pub user_agent_hash: Option<String>,
    pub trust_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOffsetInput {
    pub client_id: String,
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryProgressInput {
    pub client_id: String,
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
    pub last_sent_event_seq: Option<i64>,
    pub last_acked_event_seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessageInput {
    pub message_kind: String,
    pub payload: Value,
    pub dedupe_key: Option<String>,
    pub max_attempts: Option<i64>,
    pub next_run_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePressureEventInput {
    pub id: Option<String>,
    pub state: Option<String>,
    pub db_file_bytes: Option<i64>,
    pub wal_file_bytes: Option<i64>,
    pub disk_free_bytes: Option<i64>,
    pub temp_free_bytes: Option<i64>,
    pub quota_bytes: Option<i64>,
    pub action_taken: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequestInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub request_kind: Option<String>,
    pub policy_id: Option<String>,
    pub requester_ref: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequestInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub export_kind: Option<String>,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub output_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportBundleInput {
    pub id: Option<String>,
    pub scope: Value,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub output_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocumentInput {
    pub document_id: Option<String>,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub document_kind: Option<String>,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_profile_id: Option<String>,
    pub raw_text: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInputEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub data: String,
    pub is_paste: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_event_id: Option<String>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub shell_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub tab_id: Option<String>,
    pub payload: Vec<u8>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub source_sequence: Option<u64>,
    pub occurred_at_ms: Option<i64>,
    pub capture_semantics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryGapEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub tab_id: Option<String>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub skipped_events: u64,
    pub estimated_dropped_bytes: Option<i64>,
    pub reason: String,
    pub occurred_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshotEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub tab_id: Option<String>,
    pub screen: ScreenSnapshot,
    pub buffer_kind: Option<String>,
    pub capture_semantics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySnapshotEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub topology: TopologySnapshot,
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
    pub latest_restore_drill_status: Option<String>,
    pub evidence: Vec<RestoreEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneHistoryReplayStrategy {
    Empty,
    RawVtStream,
    RenderedSnapshot,
    Mixed,
    Degraded,
}

impl PaneHistoryReplayStrategy {
    #[must_use]
    fn from_evidence(
        segments: &[StreamSegmentRecord],
        latest_screen_snapshot: Option<&ScreenSnapshotRecord>,
        gaps: &[HistoryGapRecord],
    ) -> Self {
        if !gaps.is_empty() {
            return Self::Degraded;
        }
        let has_raw = segments.iter().any(|segment| segment.capture_semantics == "raw_vt_stream");
        let has_rendered =
            segments.iter().any(|segment| segment.capture_semantics != "raw_vt_stream")
                || latest_screen_snapshot.is_some();
        match (has_raw, has_rendered) {
            (true, false) => Self::RawVtStream,
            (false, true) => Self::RenderedSnapshot,
            (true, true) => Self::Mixed,
            (false, false) => Self::Empty,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::RawVtStream => "raw_vt_stream",
            Self::RenderedSnapshot => "rendered_snapshot",
            Self::Mixed => "mixed",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSnapshotRecord {
    pub id: String,
    pub session_id: String,
    pub pane_id: String,
    pub projection_source: String,
    pub buffer_kind: String,
    pub rows: i32,
    pub cols: i32,
    pub base_event_seq: i64,
    pub high_water_event_seq: i64,
    pub high_water_byte_seq: Option<i64>,
    pub screen_json: String,
    pub parser_version: String,
    pub projection_version: String,
    pub checksum: String,
    pub created_at_ms: i64,
}

impl From<ScreenSnapshotRow> for ScreenSnapshotRecord {
    fn from(row: ScreenSnapshotRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            projection_source: row.projection_source,
            buffer_kind: row.buffer_kind,
            rows: row.rows,
            cols: row.cols,
            base_event_seq: row.base_event_seq,
            high_water_event_seq: row.high_water_event_seq,
            high_water_byte_seq: row.high_water_byte_seq,
            screen_json: row.screen_json,
            parser_version: row.parser_version,
            projection_version: row.projection_version,
            checksum: row.checksum,
            created_at_ms: row.created_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryGapRecord {
    pub id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: String,
    pub gap_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub estimated_dropped_bytes: Option<i64>,
    pub estimated_dropped_events: Option<i64>,
    pub reason: String,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
}

impl From<HistoryGapRow> for HistoryGapRecord {
    fn from(row: HistoryGapRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            stream_id: row.stream_id,
            gap_kind: row.gap_kind,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            estimated_dropped_bytes: row.estimated_dropped_bytes,
            estimated_dropped_events: row.estimated_dropped_events,
            reason: row.reason,
            opened_at_ms: row.opened_at_ms,
            closed_at_ms: row.closed_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryHydrationRecord {
    pub session_id: String,
    pub pane_id: String,
    pub from_event_seq: i64,
    pub max_segments: i64,
    pub max_bytes: i64,
    pub restore_plan: RestorePlan,
    pub latest_screen_snapshot: Option<ScreenSnapshotRecord>,
    pub segments: Vec<StreamSegmentRecord>,
    pub gaps: Vec<HistoryGapRecord>,
    pub replay_strategy: PaneHistoryReplayStrategy,
    pub has_more_segments: bool,
    pub next_event_seq: Option<i64>,
    pub total_payload_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreDrillRecord {
    pub id: String,
    pub session_id: String,
    pub drill_kind: String,
    pub result: String,
    pub restore_guarantee_level: String,
    pub checked_at_ms: i64,
    pub duration_ms: Option<i64>,
    pub source_snapshot_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityCheckRecord {
    pub id: String,
    pub check_kind: String,
    pub scope_kind: String,
    pub scope_ref: Option<String>,
    pub result: String,
    pub checked_at_ms: i64,
    pub details_json: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataHealthRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub detection_kind: String,
    pub severity: String,
    pub first_bad_event_seq: Option<i64>,
    pub affected_ref: Option<String>,
    pub action_state: String,
    pub detected_at_ms: i64,
    pub resolved_at_ms: Option<i64>,
    pub details_json: Option<Value>,
}

impl TryFrom<DataHealthRecordRow> for DataHealthRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: DataHealthRecordRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            detection_kind: row.detection_kind,
            severity: row.severity,
            first_bad_event_seq: row.first_bad_event_seq,
            affected_ref: row.affected_ref,
            action_state: row.action_state,
            detected_at_ms: row.detected_at_ms,
            resolved_at_ms: row.resolved_at_ms,
            details_json: row.details_json.map(|value| serde_json::from_str(&value)).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub backup_kind: String,
    pub state: String,
    pub target_ref_hash: Option<String>,
    pub manifest_json: Option<Value>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub source_db_path_hash: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub quick_check_result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePressureRecord {
    pub id: String,
    pub state: String,
    pub db_file_bytes: Option<i64>,
    pub wal_file_bytes: Option<i64>,
    pub disk_free_bytes: Option<i64>,
    pub temp_free_bytes: Option<i64>,
    pub quota_bytes: Option<i64>,
    pub action_taken: String,
    pub reason: Option<String>,
    pub created_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<StoragePressureEventRow> for StoragePressureRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: StoragePressureEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl From<NewStoragePressureEventRow> for StoragePressureRecord {
    fn from(row: NewStoragePressureEventRow) -> Self {
        Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row.metadata_json.and_then(|value| serde_json::from_str(&value).ok()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteRequestRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub request_kind: String,
    pub state: String,
    pub policy_id: Option<String>,
    pub requested_at_ms: i64,
    pub approved_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub requester_ref_hash: Option<String>,
    pub reason: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<DeleteRequestRow> for DeleteRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: DeleteRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            request_kind: row.request_kind,
            state: row.state,
            policy_id: row.policy_id,
            requested_at_ms: row.requested_at_ms,
            approved_at_ms: row.approved_at_ms,
            completed_at_ms: row.completed_at_ms,
            requester_ref_hash: row.requester_ref_hash,
            reason: row.reason,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<NewDeleteRequestRow> for DeleteRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewDeleteRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            request_kind: row.request_kind,
            state: row.state,
            policy_id: row.policy_id,
            requested_at_ms: row.requested_at_ms,
            approved_at_ms: row.approved_at_ms,
            completed_at_ms: row.completed_at_ms,
            requester_ref_hash: row.requester_ref_hash,
            reason: row.reason,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletionTombstoneRecord {
    pub id: String,
    pub delete_request_id: Option<String>,
    pub session_id: Option<String>,
    pub deleted_scope: String,
    pub policy_id: Option<String>,
    pub deleted_at_ms: i64,
    pub evidence_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewDeletionTombstoneRow> for DeletionTombstoneRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewDeletionTombstoneRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            delete_request_id: row.delete_request_id,
            session_id: row.session_id,
            deleted_scope: row.deleted_scope,
            policy_id: row.policy_id,
            deleted_at_ms: row.deleted_at_ms,
            evidence_json: row
                .evidence_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRequestRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub export_kind: String,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub approved_at_ms: Option<i64>,
    pub requested_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub manifest_json: Option<Value>,
    pub output_ref_hash: Option<String>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewExportRequestRow> for ExportRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewExportRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            export_kind: row.export_kind,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            approved_at_ms: row.approved_at_ms,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportBundleRecord {
    pub id: String,
    pub scope_json: Value,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub requested_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub manifest_json: Option<Value>,
    pub output_ref_hash: Option<String>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewSupportBundleRow> for SupportBundleRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewSupportBundleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            scope_json: serde_json::from_str(&row.scope_json)?,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocumentRecord {
    pub document_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub document_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_profile_id: Option<String>,
    pub redaction_state: String,
    pub source_hash_algorithm: String,
    pub source_hash: String,
    pub text_preview: String,
    pub updated_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<SearchDocumentRow> for SearchDocumentRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: SearchDocumentRow) -> Result<Self, Self::Error> {
        Ok(Self {
            document_id: row.document_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            command_block_id: row.command_block_id,
            document_kind: row.document_kind,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            redaction_profile_id: row.redaction_profile_id,
            redaction_state: row.redaction_state,
            source_hash_algorithm: row.source_hash_algorithm,
            source_hash: row.source_hash,
            text_preview: row.text_preview,
            updated_at_ms: row.updated_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryClientRecord {
    pub id: String,
    pub client_kind: String,
    pub last_seen_at_ms: i64,
    pub trust_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryOffsetRecord {
    pub id: String,
    pub client_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: String,
    pub last_sent_event_seq: i64,
    pub last_acked_event_seq: i64,
    pub last_persisted_event_seq: i64,
    pub replay_from_event_seq: Option<i64>,
    pub gap_state: String,
    pub updated_at_ms: i64,
}

impl From<DeliveryOffsetRow> for DeliveryOffsetRecord {
    fn from(row: DeliveryOffsetRow) -> Self {
        Self {
            id: row.id,
            client_id: row.client_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            stream_id: row.stream_id,
            last_sent_event_seq: row.last_sent_event_seq,
            last_acked_event_seq: row.last_acked_event_seq,
            last_persisted_event_seq: row.last_persisted_event_seq,
            replay_from_event_seq: row.replay_from_event_seq,
            gap_state: row.gap_state,
            updated_at_ms: row.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReplayWindow {
    pub from_event_seq: Option<i64>,
    pub to_event_seq: i64,
    pub gap_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxMessageRecord {
    pub id: String,
    pub message_kind: String,
    pub dedupe_key: Option<String>,
    pub state: String,
    pub payload_json: Value,
    pub attempts: i64,
    pub max_attempts: i64,
    pub claimed_by: Option<String>,
    pub lease_token: Option<String>,
    pub claimed_until_ms: Option<i64>,
    pub next_run_at_ms: i64,
    pub last_error: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl TryFrom<OutboxMessageRow> for OutboxMessageRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: OutboxMessageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            message_kind: row.message_kind,
            dedupe_key: row.dedupe_key,
            state: row.state,
            payload_json: serde_json::from_str(&row.payload_json)?,
            attempts: row.attempts,
            max_attempts: row.max_attempts,
            claimed_by: row.claimed_by,
            lease_token: row.lease_token,
            claimed_until_ms: row.claimed_until_ms,
            next_run_at_ms: row.next_run_at_ms,
            last_error: row.last_error,
            created_at_ms: row.created_at_ms,
            updated_at_ms: row.updated_at_ms,
        })
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamSegmentRecord {
    pub id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub payload: Vec<u8>,
    pub checksum: String,
    pub capture_semantics: String,
    pub created_at_ms: i64,
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
            capture_semantics: row.capture_semantics,
            created_at_ms: row.created_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_capture_receipts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct CaptureReceiptRow {
    id: String,
    session_id: String,
    commit_id: Option<String>,
    source_kind: String,
    source_event_id_hash: String,
    source_payload_hash: String,
    received_at_ms: i64,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_capture_receipts)]
struct NewCaptureReceiptRow {
    id: String,
    session_id: String,
    commit_id: Option<String>,
    source_kind: String,
    source_event_id_hash: String,
    source_payload_hash: String,
    received_at_ms: i64,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_clients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct DeliveryClientRow {
    id: String,
    client_kind: String,
    install_ref_hash: Option<String>,
    browser_profile_ref_hash: Option<String>,
    user_agent_hash: Option<String>,
    created_at_ms: i64,
    last_seen_at_ms: i64,
    trust_state: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_clients)]
struct NewDeliveryClientRow {
    id: String,
    client_kind: String,
    install_ref_hash: Option<String>,
    browser_profile_ref_hash: Option<String>,
    user_agent_hash: Option<String>,
    created_at_ms: i64,
    last_seen_at_ms: i64,
    trust_state: String,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_delivery_offsets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct DeliveryOffsetRow {
    id: String,
    client_id: String,
    session_id: String,
    pane_id: Option<String>,
    stream_id: String,
    last_sent_event_seq: i64,
    last_acked_event_seq: i64,
    last_persisted_event_seq: i64,
    replay_from_event_seq: Option<i64>,
    gap_state: String,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_delivery_offsets)]
struct NewDeliveryOffsetRow {
    id: String,
    client_id: String,
    session_id: String,
    pane_id: Option<String>,
    stream_id: String,
    last_sent_event_seq: i64,
    last_acked_event_seq: i64,
    last_persisted_event_seq: i64,
    replay_from_event_seq: Option<i64>,
    gap_state: String,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_outbox_messages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct OutboxMessageRow {
    id: String,
    message_kind: String,
    dedupe_key: Option<String>,
    state: String,
    payload_json: String,
    attempts: i64,
    max_attempts: i64,
    claimed_by: Option<String>,
    lease_token: Option<String>,
    claimed_until_ms: Option<i64>,
    next_run_at_ms: i64,
    last_error: Option<String>,
    created_at_ms: i64,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_outbox_messages)]
struct NewOutboxMessageRow {
    id: String,
    message_kind: String,
    dedupe_key: Option<String>,
    state: String,
    payload_json: String,
    attempts: i64,
    max_attempts: i64,
    claimed_by: Option<String>,
    lease_token: Option<String>,
    claimed_until_ms: Option<i64>,
    next_run_at_ms: i64,
    last_error: Option<String>,
    created_at_ms: i64,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_history_gaps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct HistoryGapRow {
    id: String,
    session_id: String,
    pane_id: Option<String>,
    stream_id: String,
    gap_kind: String,
    event_seq_low: Option<i64>,
    event_seq_high: Option<i64>,
    byte_low: Option<i64>,
    byte_high: Option<i64>,
    estimated_dropped_bytes: Option<i64>,
    estimated_dropped_events: Option<i64>,
    reason: String,
    writer_generation: Option<String>,
    opened_at_ms: i64,
    closed_at_ms: Option<i64>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_history_gaps)]
struct NewHistoryGapRow {
    id: String,
    session_id: String,
    pane_id: Option<String>,
    stream_id: String,
    gap_kind: String,
    event_seq_low: Option<i64>,
    event_seq_high: Option<i64>,
    byte_low: Option<i64>,
    byte_high: Option<i64>,
    estimated_dropped_bytes: Option<i64>,
    estimated_dropped_events: Option<i64>,
    reason: String,
    writer_generation: Option<String>,
    opened_at_ms: i64,
    closed_at_ms: Option<i64>,
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

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_integrity_checks)]
struct NewIntegrityCheckRow {
    id: String,
    check_kind: String,
    scope_kind: String,
    scope_ref: Option<String>,
    result: String,
    checked_at_ms: i64,
    details_json: Option<String>,
    error: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_data_health_records)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct DataHealthRecordRow {
    id: String,
    session_id: Option<String>,
    pane_id: Option<String>,
    detection_kind: String,
    severity: String,
    first_bad_event_seq: Option<i64>,
    affected_ref: Option<String>,
    action_state: String,
    detected_at_ms: i64,
    resolved_at_ms: Option<i64>,
    details_json: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_data_health_records)]
struct NewDataHealthRecordRow {
    id: String,
    session_id: Option<String>,
    pane_id: Option<String>,
    detection_kind: String,
    severity: String,
    first_bad_event_seq: Option<i64>,
    affected_ref: Option<String>,
    action_state: String,
    detected_at_ms: i64,
    resolved_at_ms: Option<i64>,
    details_json: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_backup_records)]
struct NewBackupRecordRow {
    id: String,
    backup_kind: String,
    state: String,
    target_ref_hash: Option<String>,
    manifest_json: Option<String>,
    checksum_algorithm: Option<String>,
    checksum: Option<String>,
    source_db_path_hash: Option<String>,
    started_at_ms: i64,
    finished_at_ms: Option<i64>,
    quick_check_result: Option<String>,
    error: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_storage_pressure_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct StoragePressureEventRow {
    id: String,
    state: String,
    db_file_bytes: Option<i64>,
    wal_file_bytes: Option<i64>,
    disk_free_bytes: Option<i64>,
    temp_free_bytes: Option<i64>,
    quota_bytes: Option<i64>,
    action_taken: String,
    reason: Option<String>,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_storage_pressure_events)]
struct NewStoragePressureEventRow {
    id: String,
    state: String,
    db_file_bytes: Option<i64>,
    wal_file_bytes: Option<i64>,
    disk_free_bytes: Option<i64>,
    temp_free_bytes: Option<i64>,
    quota_bytes: Option<i64>,
    action_taken: String,
    reason: Option<String>,
    created_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_delete_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct DeleteRequestRow {
    id: String,
    session_id: Option<String>,
    request_kind: String,
    state: String,
    policy_id: Option<String>,
    requested_at_ms: i64,
    approved_at_ms: Option<i64>,
    completed_at_ms: Option<i64>,
    requester_ref_hash: Option<String>,
    reason: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_delete_requests)]
struct NewDeleteRequestRow {
    id: String,
    session_id: Option<String>,
    request_kind: String,
    state: String,
    policy_id: Option<String>,
    requested_at_ms: i64,
    approved_at_ms: Option<i64>,
    completed_at_ms: Option<i64>,
    requester_ref_hash: Option<String>,
    reason: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_deletion_tombstones)]
struct NewDeletionTombstoneRow {
    id: String,
    delete_request_id: Option<String>,
    session_id: Option<String>,
    deleted_scope: String,
    policy_id: Option<String>,
    deleted_at_ms: i64,
    evidence_json: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_export_requests)]
struct NewExportRequestRow {
    id: String,
    session_id: Option<String>,
    export_kind: String,
    state: String,
    redaction_profile_id: Option<String>,
    include_raw: i32,
    approved_at_ms: Option<i64>,
    requested_at_ms: i64,
    completed_at_ms: Option<i64>,
    manifest_json: Option<String>,
    output_ref_hash: Option<String>,
    error: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_support_bundles)]
struct NewSupportBundleRow {
    id: String,
    scope_json: String,
    state: String,
    redaction_profile_id: Option<String>,
    include_raw: i32,
    requested_at_ms: i64,
    completed_at_ms: Option<i64>,
    manifest_json: Option<String>,
    output_ref_hash: Option<String>,
    error: Option<String>,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_search_documents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct SearchDocumentRow {
    rowid: i32,
    document_id: String,
    session_id: String,
    pane_id: Option<String>,
    command_block_id: Option<String>,
    document_kind: String,
    event_seq_low: Option<i64>,
    event_seq_high: Option<i64>,
    byte_low: Option<i64>,
    byte_high: Option<i64>,
    redaction_profile_id: Option<String>,
    redaction_state: String,
    source_hash_algorithm: String,
    source_hash: String,
    text_preview: String,
    updated_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_search_documents)]
struct NewSearchDocumentRow {
    document_id: String,
    session_id: String,
    pane_id: Option<String>,
    command_block_id: Option<String>,
    document_kind: String,
    event_seq_low: Option<i64>,
    event_seq_high: Option<i64>,
    byte_low: Option<i64>,
    byte_high: Option<i64>,
    redaction_profile_id: Option<String>,
    redaction_state: String,
    source_hash_algorithm: String,
    source_hash: String,
    text_preview: String,
    updated_at_ms: i64,
    metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_db_identity)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
struct DbIdentityProbeRow {
    id: i32,
}

#[derive(Debug, QueryableByName)]
struct QuickCheckRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    quick_check: String,
}

#[derive(Debug, QueryableByName, Serialize)]
struct ForeignKeyCheckRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    table_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::BigInt>)]
    rowid: Option<i64>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    parent: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    fkid: i32,
}

#[derive(Debug, Clone)]
struct HistoryValidation {
    stream_segments_checked: usize,
    screen_snapshots_checked: usize,
    topology_snapshots_checked: usize,
    failures: Vec<String>,
}

impl HistoryValidation {
    fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    fn failure_count(&self) -> usize {
        self.failures.len()
    }

    fn summary(&self) -> String {
        if self.failures.is_empty() {
            "history validation passed".to_string()
        } else {
            self.failures.join("; ")
        }
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "stream_segments_checked": self.stream_segments_checked,
            "screen_snapshots_checked": self.screen_snapshots_checked,
            "topology_snapshots_checked": self.topology_snapshots_checked,
            "failures": self.failures,
        })
    }

    fn to_restore_evidence(&self) -> Vec<RestoreEvidence> {
        vec![
            RestoreEvidence {
                kind: "stream_segments_checked".to_string(),
                value: self.stream_segments_checked.to_string(),
            },
            RestoreEvidence {
                kind: "screen_snapshots_checked".to_string(),
                value: self.screen_snapshots_checked.to_string(),
            },
            RestoreEvidence {
                kind: "topology_snapshots_checked".to_string(),
                value: self.topology_snapshots_checked.to_string(),
            },
            RestoreEvidence {
                kind: "history_validation_failures".to_string(),
                value: self.failures.len().to_string(),
            },
        ]
    }
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

fn run_quick_check(
    connection: &mut SqliteConnection,
) -> Result<Vec<String>, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA quick_check")
        .load::<QuickCheckRow>(connection)
        .map(|rows| rows.into_iter().map(|row| row.quick_check).collect())
        .map_err(Into::into)
}

fn run_foreign_key_check(
    connection: &mut SqliteConnection,
) -> Result<Vec<ForeignKeyCheckRow>, TerminalPersistenceV2Error> {
    diesel::sql_query(
        "SELECT \"table\" AS table_name, rowid, parent, fkid FROM pragma_foreign_key_check",
    )
    .load::<ForeignKeyCheckRow>(connection)
    .map_err(Into::into)
}

fn validate_history_checksums(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
) -> Result<HistoryValidation, TerminalPersistenceV2Error> {
    let mut failures = Vec::new();

    let mut segment_query = terminal_stream_segments::table.into_boxed();
    if let Some(session_id) = session_id {
        segment_query = segment_query.filter(terminal_stream_segments::session_id.eq(session_id));
    }
    let segment_rows = segment_query
        .select((
            terminal_stream_segments::id,
            terminal_stream_segments::payload,
            terminal_stream_segments::checksum_algorithm,
            terminal_stream_segments::checksum,
        ))
        .load::<(String, Vec<u8>, String, String)>(connection)?;
    for (id, payload, algorithm, checksum) in &segment_rows {
        validate_checksum_bytes("stream_segment", id, payload, algorithm, checksum, &mut failures);
    }

    let mut screen_query = terminal_screen_snapshots::table.into_boxed();
    if let Some(session_id) = session_id {
        screen_query = screen_query.filter(terminal_screen_snapshots::session_id.eq(session_id));
    }
    let screen_rows = screen_query
        .select((
            terminal_screen_snapshots::id,
            terminal_screen_snapshots::screen_json,
            terminal_screen_snapshots::checksum_algorithm,
            terminal_screen_snapshots::checksum,
        ))
        .load::<(String, String, String, String)>(connection)?;
    for (id, payload, algorithm, checksum) in &screen_rows {
        validate_checksum_text("screen_snapshot", id, payload, algorithm, checksum, &mut failures);
    }

    let mut topology_query = terminal_topology_snapshots::table.into_boxed();
    if let Some(session_id) = session_id {
        topology_query =
            topology_query.filter(terminal_topology_snapshots::session_id.eq(session_id));
    }
    let topology_rows = topology_query
        .select((
            terminal_topology_snapshots::id,
            terminal_topology_snapshots::topology_json,
            terminal_topology_snapshots::checksum_algorithm,
            terminal_topology_snapshots::checksum,
        ))
        .load::<(String, String, String, String)>(connection)?;
    for (id, payload, algorithm, checksum) in &topology_rows {
        validate_checksum_text(
            "topology_snapshot",
            id,
            payload,
            algorithm,
            checksum,
            &mut failures,
        );
    }

    Ok(HistoryValidation {
        stream_segments_checked: segment_rows.len(),
        screen_snapshots_checked: screen_rows.len(),
        topology_snapshots_checked: topology_rows.len(),
        failures,
    })
}

fn persist_history_validation_health_records(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    validation: &HistoryValidation,
    detected_at_ms: i64,
    evidence_ref: Option<&str>,
) -> Result<(), TerminalPersistenceV2Error> {
    for failure in &validation.failures {
        let detection_kind =
            if failure.contains("checksum mismatch") { "checksum_mismatch" } else { "manual" };
        let is_canonical_stream = failure.starts_with("stream_segment:");
        let severity = if is_canonical_stream { "critical" } else { "error" };
        let action_state = if is_canonical_stream { "quarantined" } else { "rebuild_pending" };
        let affected_ref = Some(failure.clone());
        let existing = terminal_data_health_records::table
            .filter(terminal_data_health_records::affected_ref.eq(affected_ref.clone()))
            .filter(terminal_data_health_records::detection_kind.eq(detection_kind))
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .select(DataHealthRecordRow::as_select())
            .first::<DataHealthRecordRow>(connection)
            .optional()?;
        if existing.is_some() {
            continue;
        }

        let details_json = Some(serde_json::to_string(&serde_json::json!({
            "failure": failure,
            "evidence_ref": evidence_ref,
            "validation": {
                "stream_segments_checked": validation.stream_segments_checked,
                "screen_snapshots_checked": validation.screen_snapshots_checked,
                "topology_snapshots_checked": validation.topology_snapshots_checked
            }
        }))?);
        let row = NewDataHealthRecordRow {
            id: new_id(),
            session_id: session_id.map(ToOwned::to_owned),
            pane_id: None,
            detection_kind: detection_kind.to_string(),
            severity: severity.to_string(),
            first_bad_event_seq: None,
            affected_ref,
            action_state: action_state.to_string(),
            detected_at_ms,
            resolved_at_ms: None,
            details_json,
            metadata_json: None,
        };
        insert_into(terminal_data_health_records::table).values(&row).execute(connection)?;
    }
    Ok(())
}

fn validate_checksum_bytes(
    row_kind: &str,
    id: &str,
    payload: &[u8],
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    if algorithm != "blake3" {
        failures.push(format!("{row_kind}:{id} uses unsupported checksum algorithm {algorithm}"));
        return;
    }
    let actual = blake3_hash_bytes(payload);
    if actual != expected {
        failures.push(format!("{row_kind}:{id} checksum mismatch"));
    }
}

fn validate_checksum_text(
    row_kind: &str,
    id: &str,
    payload: &str,
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    validate_checksum_bytes(row_kind, id, payload.as_bytes(), algorithm, expected, failures);
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

fn load_capture_receipt(
    connection: &mut SqliteConnection,
    session_id: &str,
    source_kind: &str,
    source_event_id_hash: &str,
) -> Result<Option<CaptureReceiptRow>, TerminalPersistenceV2Error> {
    terminal_capture_receipts::table
        .filter(terminal_capture_receipts::session_id.eq(session_id))
        .filter(terminal_capture_receipts::source_kind.eq(source_kind))
        .filter(terminal_capture_receipts::source_event_id_hash.eq(source_event_id_hash))
        .select(CaptureReceiptRow::as_select())
        .first::<CaptureReceiptRow>(connection)
        .optional()
        .map_err(Into::into)
}

fn stream_segment_receipt_from_capture_receipt(
    connection: &mut SqliteConnection,
    receipt: &CaptureReceiptRow,
) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
    let commit_id = receipt.commit_id.as_deref().ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData(format!(
            "stream capture receipt {} does not point to a commit",
            receipt.id
        ))
    })?;
    stream_segment_receipt_from_commit(connection, commit_id)
}

fn stream_segment_receipt_from_commit(
    connection: &mut SqliteConnection,
    commit_ref: &str,
) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
    let segment = terminal_stream_segments::table
        .filter(terminal_stream_segments::commit_id.eq(commit_ref))
        .select(StreamSegmentRow::as_select())
        .first::<StreamSegmentRow>(connection)?;
    let event_id = terminal_journal_events::table
        .filter(terminal_journal_events::commit_id.eq(commit_ref))
        .select(terminal_journal_events::id)
        .first::<String>(connection)?;
    let commit_seq = terminal_commit_log::table
        .filter(terminal_commit_log::id.eq(commit_ref))
        .select(terminal_commit_log::commit_seq)
        .first::<i64>(connection)?;

    Ok(StreamSegmentReceipt {
        commit_id: commit_ref.to_string(),
        commit_seq,
        segment_id: segment.id,
        event_id,
        event_seq_low: segment.event_seq_low,
        event_seq_high: segment.event_seq_high,
        byte_low: segment.byte_low,
        byte_high: segment.byte_high,
        checksum: segment.checksum,
    })
}

fn touch_delivery_client(
    connection: &mut SqliteConnection,
    client_id: &str,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let updated =
        diesel::update(terminal_clients::table.filter(terminal_clients::id.eq(client_id)))
            .set(terminal_clients::last_seen_at_ms.eq(now))
            .execute(connection)?;
    if updated == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "delivery client not found: {client_id}"
        )));
    }
    Ok(())
}

fn load_delivery_offset(
    connection: &mut SqliteConnection,
    client_id: &str,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<Option<DeliveryOffsetRow>, TerminalPersistenceV2Error> {
    terminal_delivery_offsets::table
        .filter(terminal_delivery_offsets::client_id.eq(client_id))
        .filter(terminal_delivery_offsets::session_id.eq(session_id))
        .filter(terminal_delivery_offsets::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_delivery_offsets::stream_id.eq(stream_id))
        .select(DeliveryOffsetRow::as_select())
        .first::<DeliveryOffsetRow>(connection)
        .optional()
        .map_err(Into::into)
}

fn load_persisted_event_high_water(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    let segment_high = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .filter(terminal_stream_segments::stream_id.eq(stream_id))
        .select(max(terminal_stream_segments::event_seq_high))
        .first::<Option<i64>>(connection)?
        .unwrap_or(0);
    let gap_high = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_history_gaps::stream_id.eq(stream_id))
        .filter(terminal_history_gaps::event_seq_high.is_not_null())
        .select(max(terminal_history_gaps::event_seq_high))
        .first::<Option<i64>>(connection)?
        .unwrap_or(0);

    Ok(segment_high.max(gap_high))
}

fn has_history_gap_in_range(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
    from_event_seq: i64,
    to_event_seq: i64,
) -> Result<bool, TerminalPersistenceV2Error> {
    if from_event_seq > to_event_seq {
        return Ok(false);
    }
    let count = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_history_gaps::stream_id.eq(stream_id))
        .filter(terminal_history_gaps::event_seq_low.le(Some(to_event_seq)))
        .filter(terminal_history_gaps::event_seq_high.ge(Some(from_event_seq)))
        .count()
        .get_result::<i64>(connection)?;
    Ok(count > 0)
}

fn load_outbox_message(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<OutboxMessageRow, TerminalPersistenceV2Error> {
    terminal_outbox_messages::table
        .filter(terminal_outbox_messages::id.eq(id))
        .select(OutboxMessageRow::as_select())
        .first::<OutboxMessageRow>(connection)
        .map_err(Into::into)
}

fn load_outbox_message_by_dedupe(
    connection: &mut SqliteConnection,
    dedupe_key: &str,
) -> Result<Option<OutboxMessageRow>, TerminalPersistenceV2Error> {
    terminal_outbox_messages::table
        .filter(terminal_outbox_messages::dedupe_key.eq(Some(dedupe_key.to_string())))
        .select(OutboxMessageRow::as_select())
        .first::<OutboxMessageRow>(connection)
        .optional()
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

fn validate_optional_half_open_range(
    low: Option<i64>,
    high: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    match (low, high) {
        (Some(low), Some(high)) if low < high => Ok(()),
        (None, None) => Ok(()),
        _ => Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} range must be empty or half-open with low < high"
        ))),
    }
}

fn validate_non_negative_seq(
    value: Option<i64>,
    label: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if let Some(value) = value
        && value < 0
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{label} must not be negative"
        )));
    }
    Ok(())
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

fn topology_pane_high_water(topology: &TopologySnapshot) -> Value {
    let mut map = serde_json::Map::new();
    for tab in &topology.tabs {
        collect_topology_pane_high_water(&tab.root, &mut map);
    }
    Value::Object(map)
}

fn collect_topology_pane_high_water(
    node: &terminal_mux_domain::PaneTreeNode,
    map: &mut serde_json::Map<String, Value>,
) {
    match node {
        terminal_mux_domain::PaneTreeNode::Leaf { pane_id } => {
            map.entry(pane_id.0.to_string()).or_insert(Value::from(0));
        }
        terminal_mux_domain::PaneTreeNode::Split(split) => {
            collect_topology_pane_high_water(&split.first, map);
            collect_topology_pane_high_water(&split.second, map);
        }
    }
}

fn stream_cursor_id(pane_id: &str, stream_id: &str) -> String {
    format!("stream-cursor-{}", blake3_hash_text(&format!("{pane_id}\0{stream_id}")))
}

fn stream_capture_source_kind(pane_id: &str, stream_id: &str) -> String {
    format!("stream-segment-{}", blake3_hash_text(&format!("{pane_id}\0{stream_id}")))
}

fn delivery_offset_id(client_id: &str, session_id: &str, pane_id: &str, stream_id: &str) -> String {
    format!(
        "delivery-offset-{}",
        blake3_hash_text(&format!("{client_id}\0{session_id}\0{pane_id}\0{stream_id}"))
    )
}

fn normalize_outbox_dedupe_key(value: &str) -> String {
    format!("blake3:{}", blake3_hash_text(value))
}

fn ui_input_capture_source_kind(pane_id: &str) -> String {
    format!("ui-input-{}", blake3_hash_text(pane_id))
}

fn stable_ui_command_block_id(
    session_id: &str,
    pane_id: &str,
    source_event_id_hash: &str,
) -> String {
    format!(
        "command-block-{}",
        blake3_hash_text(&format!("{session_id}\0{pane_id}\0{source_event_id_hash}"))
    )
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

fn stable_search_document_id(
    session_id: &str,
    pane_id: Option<&str>,
    command_block_id: Option<&str>,
    source_hash: &str,
) -> String {
    let material = format!(
        "{}\0{}\0{}\0{}",
        session_id,
        pane_id.unwrap_or_default(),
        command_block_id.unwrap_or_default(),
        source_hash
    );
    format!("search-document-{}", blake3_hash_text(&material))
}

fn command_text_from_ui_input(data: &str) -> Option<String> {
    let trimmed_end = data.trim_end_matches(['\r', '\n']);
    if trimmed_end.len() == data.len() {
        return None;
    }
    let command = trimmed_end.trim();
    if command.is_empty() { None } else { Some(command.to_string()) }
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

fn blake3_hash_file(path: &Path) -> Result<String, TerminalPersistenceV2Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn path_hash(path: &Path) -> String {
    blake3_hash_text(&path.to_string_lossy())
}

fn privacy_manifest(kind: &str, include_raw: bool, session_id: Option<&str>) -> Value {
    let included_classes = if include_raw {
        vec![
            "class_public_diagnostic",
            "class_local_metadata",
            "class_user_context",
            "class_sensitive_content",
        ]
    } else {
        vec!["class_public_diagnostic", "class_local_metadata", "class_user_context_redacted"]
    };
    let excluded_classes = if include_raw {
        vec!["class_secret_material"]
    } else {
        vec!["class_sensitive_content", "class_secret_material"]
    };
    serde_json::json!({
        "kind": kind,
        "include_raw": include_raw,
        "session_id": session_id,
        "included_classes": included_classes,
        "excluded_classes": excluded_classes,
        "raw_terminal_output": include_raw,
        "raw_command_text": include_raw,
        "prompt_injection_text_is_data": true,
    })
}

fn limit_text_preview(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn redact_terminal_text(value: &str) -> String {
    let mut redacted = Vec::new();
    for token in value.split_whitespace() {
        redacted.push(redact_token(token));
    }
    redacted.join(" ")
}

fn redact_token(token: &str) -> String {
    const KEY_PREFIXES: [&str; 8] = [
        "password=",
        "passwd=",
        "pwd=",
        "token=",
        "access_token=",
        "api_key=",
        "apikey=",
        "secret=",
    ];
    let lower = token.to_ascii_lowercase();
    if lower == "bearer" {
        return token.to_string();
    }
    if token.len() >= 24
        && token.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        && token.chars().any(|ch| ch.is_ascii_digit())
        && token.chars().any(|ch| ch.is_ascii_alphabetic())
    {
        return "[redacted-secret]".to_string();
    }
    for prefix in KEY_PREFIXES {
        if lower.starts_with(prefix) {
            return format!("{}[redacted]", &token[..prefix.len().min(token.len())]);
        }
    }
    token.to_string()
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

        let hydrated = reopened
            .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
            .expect("pane history should hydrate");

        assert_eq!(hydrated.segments.len(), 2);
        assert_eq!(hydrated.gaps.len(), 0);
        assert_eq!(hydrated.replay_strategy, PaneHistoryReplayStrategy::RawVtStream);
        assert_eq!(
            hydrated
                .segments
                .iter()
                .flat_map(|segment| segment.payload.clone())
                .collect::<Vec<_>>(),
            b"git status\r\nfatal: not a git repository\r\n"
        );
    }

    #[test]
    fn dedupes_retried_stream_segment_capture_by_source_event_id() {
        let store = test_store("stream-retry-dedupe");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        let mut input = StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"cargo test\r\n".to_vec(),
        );
        input.source_event_id_hash = Some(blake3_hash_text("runtime-output-seq:42"));

        let first =
            store.append_stream_segment(input.clone()).expect("first capture should persist");
        let retry =
            store.append_stream_segment(input).expect("retry should return existing receipt");
        let segments =
            store.list_stream_segments(&session_id, &pane_id, 1, 10).expect("segments should list");

        assert_eq!(retry, first);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].payload, b"cargo test\r\n");
    }

    #[test]
    fn rejects_retry_with_same_source_event_id_and_different_payload() {
        let store = test_store("stream-retry-conflict");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        let mut input = StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"first\r\n".to_vec(),
        );
        input.source_event_id_hash = Some(blake3_hash_text("runtime-output-seq:43"));
        store.append_stream_segment(input.clone()).expect("first capture should persist");

        input.writer_generation = writer.id;
        input.payload = b"changed\r\n".to_vec();
        let error = store.append_stream_segment(input).expect_err("conflicting retry should fail");
        let segments =
            store.list_stream_segments(&session_id, &pane_id, 1, 10).expect("segments should list");

        assert!(
            matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("payload hash mismatch"))
        );
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].payload, b"first\r\n");
    }

    #[test]
    fn records_delivery_offsets_and_builds_replay_window() {
        let store = test_store("delivery-offset");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id.clone(),
                b"first\r\n".to_vec(),
            ))
            .expect("first segment should persist");
        store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id,
                b"second\r\n".to_vec(),
            ))
            .expect("second segment should persist");
        let client = store
            .upsert_delivery_client(DeliveryClientInput {
                id: Some("browser-a".to_string()),
                client_kind: "browser".to_string(),
                install_ref_hash: None,
                browser_profile_ref_hash: None,
                user_agent_hash: None,
                trust_state: None,
            })
            .expect("client should persist");

        let sent = store
            .record_delivery_progress(DeliveryProgressInput {
                client_id: client.id.clone(),
                session_id: session_id.clone(),
                pane_id: pane_id.clone(),
                stream_id: None,
                last_sent_event_seq: Some(2),
                last_acked_event_seq: None,
            })
            .expect("sent offset should persist");
        let acked = store
            .record_delivery_progress(DeliveryProgressInput {
                client_id: client.id.clone(),
                session_id: session_id.clone(),
                pane_id: pane_id.clone(),
                stream_id: None,
                last_sent_event_seq: None,
                last_acked_event_seq: Some(1),
            })
            .expect("acked offset should persist");
        let window = store
            .delivery_replay_window(DeliveryOffsetInput {
                client_id: client.id.clone(),
                session_id: session_id.clone(),
                pane_id: pane_id.clone(),
                stream_id: None,
            })
            .expect("replay window should load");
        let replay = store
            .hydrate_pane_history(
                &session_id,
                &pane_id,
                window.from_event_seq,
                Some(10),
                Some(1024),
            )
            .expect("replay history should hydrate");

        assert_eq!(sent.last_sent_event_seq, 2);
        assert_eq!(acked.last_acked_event_seq, 1);
        assert_eq!(acked.replay_from_event_seq, Some(2));
        assert_eq!(window.from_event_seq, Some(2));
        assert_eq!(window.to_event_seq, 2);
        assert_eq!(window.gap_state, "none");
        assert_eq!(replay.segments.len(), 1);
        assert_eq!(replay.segments[0].payload, b"second\r\n");

        let fully_acked = store
            .record_delivery_progress(DeliveryProgressInput {
                client_id: client.id.clone(),
                session_id: session_id.clone(),
                pane_id: pane_id.clone(),
                stream_id: None,
                last_sent_event_seq: None,
                last_acked_event_seq: Some(2),
            })
            .expect("fully acked offset should persist");
        let empty_window = store
            .delivery_replay_window(DeliveryOffsetInput {
                client_id: client.id,
                session_id,
                pane_id,
                stream_id: None,
            })
            .expect("empty replay window should load");

        assert_eq!(fully_acked.replay_from_event_seq, None);
        assert_eq!(empty_window.from_event_seq, None);
        assert_eq!(empty_window.to_event_seq, 2);
    }

    #[test]
    fn delivery_replay_window_surfaces_gap_state() {
        let store = test_store("delivery-gap");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store.release_writer_generation(&writer.id).expect("writer should release");
        store
            .record_history_gap_event(HistoryGapEventInput {
                session_id: session_id.clone(),
                route: route(),
                title: Some("shell".to_string()),
                launch: None,
                pane_id: pane_id.clone(),
                tab_id: None,
                rows: Some(24),
                cols: Some(80),
                skipped_events: 2,
                estimated_dropped_bytes: Some(64),
                reason: "test_delivery_gap".to_string(),
                occurred_at_ms: None,
            })
            .expect("history gap should persist");
        let client = store
            .upsert_delivery_client(DeliveryClientInput {
                id: Some("browser-gap".to_string()),
                client_kind: "browser".to_string(),
                install_ref_hash: None,
                browser_profile_ref_hash: None,
                user_agent_hash: None,
                trust_state: None,
            })
            .expect("client should persist");

        let window = store
            .delivery_replay_window(DeliveryOffsetInput {
                client_id: client.id,
                session_id,
                pane_id,
                stream_id: None,
            })
            .expect("replay window should load");

        assert_eq!(window.from_event_seq, Some(1));
        assert_eq!(window.to_event_seq, 2);
        assert_eq!(window.gap_state, "gap");
    }

    #[test]
    fn stream_segment_enqueue_projection_outbox_message() {
        let store = test_store("outbox-stream");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        let receipt = store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id,
                b"outbox\r\n".to_vec(),
            ))
            .expect("stream segment should persist");

        let message = store
            .claim_next_outbox_message("projection-worker", 60_000)
            .expect("claim should load")
            .expect("projection outbox message should exist");

        assert_eq!(message.message_kind, "pane_history_projection");
        assert_eq!(message.state, "claimed");
        assert_eq!(message.attempts, 1);
        assert_eq!(message.payload_json["session_id"], session_id);
        assert_eq!(message.payload_json["pane_id"], pane_id);
        assert_eq!(message.payload_json["commit_id"], receipt.commit_id);
    }

    #[test]
    fn outbox_dedupes_claims_and_completes_by_lease_token() {
        let store = test_store("outbox-dedupe");
        let first = store
            .enqueue_outbox_message(OutboxMessageInput {
                message_kind: "restore_drill".to_string(),
                payload: serde_json::json!({ "session_id": "session-a" }),
                dedupe_key: Some("restore-drill:session-a".to_string()),
                max_attempts: None,
                next_run_at_ms: None,
            })
            .expect("first outbox message should enqueue");
        let second = store
            .enqueue_outbox_message(OutboxMessageInput {
                message_kind: "restore_drill".to_string(),
                payload: serde_json::json!({ "session_id": "session-a" }),
                dedupe_key: Some("restore-drill:session-a".to_string()),
                max_attempts: None,
                next_run_at_ms: None,
            })
            .expect("deduped outbox message should load");

        let claim = store
            .claim_next_outbox_message("worker-a", 60_000)
            .expect("claim should succeed")
            .expect("message should be claimable");
        let second_claim = store
            .claim_next_outbox_message("worker-b", 60_000)
            .expect("second claim should not fail");
        let wrong_token_done = store
            .mark_outbox_message_done(&claim.id, "wrong-token")
            .expect("wrong token completion should be safe");
        let done = store
            .mark_outbox_message_done(
                &claim.id,
                claim.lease_token.as_deref().expect("claim should have a lease token"),
            )
            .expect("completion should succeed");
        let no_more = store
            .claim_next_outbox_message("worker-a", 60_000)
            .expect("done message should not be claimable");

        assert_eq!(first.id, second.id);
        assert_eq!(claim.id, first.id);
        assert!(second_claim.is_none());
        assert!(!wrong_token_done);
        assert!(done);
        assert!(no_more.is_none());
    }

    #[test]
    fn outbox_quarantines_poison_message_after_max_attempts() {
        let store = test_store("outbox-quarantine");
        let message = store
            .enqueue_outbox_message(OutboxMessageInput {
                message_kind: "integrity_check".to_string(),
                payload: serde_json::json!({ "scope": "test" }),
                dedupe_key: None,
                max_attempts: Some(1),
                next_run_at_ms: None,
            })
            .expect("message should enqueue");
        let claim = store
            .claim_next_outbox_message("worker-a", 60_000)
            .expect("claim should succeed")
            .expect("message should be claimable");

        let failed = store
            .fail_outbox_message(
                &message.id,
                claim.lease_token.as_deref().expect("claim should have a lease token"),
                "synthetic failure",
            )
            .expect("failure should persist");
        let no_more = store
            .claim_next_outbox_message("worker-b", 60_000)
            .expect("quarantined message should not be claimable");

        assert_eq!(failed.state, "quarantined");
        assert_eq!(failed.last_error.as_deref(), Some("synthetic failure"));
        assert!(no_more.is_none());
    }

    #[test]
    fn records_history_gaps_as_readable_restore_evidence() {
        let store = test_store("history-gap");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store.release_writer_generation(&writer.id).expect("writer should release");

        store
            .record_history_gap_event(HistoryGapEventInput {
                session_id: session_id.clone(),
                route: route(),
                title: Some("shell".to_string()),
                launch: None,
                pane_id: pane_id.clone(),
                tab_id: None,
                rows: Some(24),
                cols: Some(80),
                skipped_events: 3,
                estimated_dropped_bytes: Some(128),
                reason: "test_receiver_lag".to_string(),
                occurred_at_ms: Some(42),
            })
            .expect("history gap should persist");

        let hydrated = store
            .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
            .expect("pane history should hydrate");

        assert_eq!(hydrated.gaps.len(), 1);
        assert_eq!(hydrated.gaps[0].event_seq_low, Some(1));
        assert_eq!(hydrated.gaps[0].event_seq_high, Some(3));
        assert_eq!(hydrated.gaps[0].estimated_dropped_events, Some(3));
        assert_eq!(hydrated.gaps[0].reason, "test_receiver_lag");
        assert_eq!(hydrated.replay_strategy, PaneHistoryReplayStrategy::Degraded);
        assert_eq!(hydrated.restore_plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
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

        let fallback_limit = store
            .list_command_history(Some(&session_id), -1)
            .expect("invalid history limit should fall back");
        assert_eq!(fallback_limit.len(), 1);
    }

    #[test]
    fn records_ui_input_as_verified_command_history() {
        let store = test_store("ui-input");
        let session_id = Uuid::new_v4().to_string();
        let pane_id = Uuid::new_v4().to_string();

        store
            .record_ui_input_event(UiInputEventInput {
                session_id: session_id.clone(),
                route: route(),
                title: Some("shell".to_string()),
                launch: None,
                pane_id: pane_id.clone(),
                data: "git status\r".to_string(),
                is_paste: false,
                source_event_id: None,
                rows: None,
                cols: None,
                shell_kind: Some("cmd".to_string()),
            })
            .expect("ui input should persist");

        let history =
            store.list_command_history(Some(&session_id), 10).expect("command history should load");
        let segments = store
            .list_stream_segments(&session_id, &pane_id, 1, 10)
            .expect("rendered/raw segments query should be valid");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].display_text, "git status");
        assert!(segments.is_empty());
    }

    #[test]
    fn dedupes_retried_ui_input_by_client_event_id() {
        let store = test_store("ui-input-retry");
        let session_id = Uuid::new_v4().to_string();
        let pane_id = Uuid::new_v4().to_string();
        let input = UiInputEventInput {
            session_id: session_id.clone(),
            route: route(),
            title: Some("shell".to_string()),
            launch: None,
            pane_id: pane_id.clone(),
            data: "git status\r".to_string(),
            is_paste: false,
            source_event_id: Some("browser-submit-1".to_string()),
            rows: None,
            cols: None,
            shell_kind: Some("cmd".to_string()),
        };

        store.record_ui_input_event(input.clone()).expect("first ui input should persist");
        store.record_ui_input_event(input).expect("retry should be deduped");

        let history =
            store.list_command_history(Some(&session_id), 10).expect("command history should load");
        let mut connection = store.connection().expect("connection should open");
        let event_count = terminal_journal_events::table
            .filter(terminal_journal_events::session_id.eq(&session_id))
            .filter(terminal_journal_events::pane_id.eq(Some(pane_id.clone())))
            .count()
            .get_result::<i64>(&mut connection)
            .expect("journal count should load");
        let command_block_count = terminal_command_blocks::table
            .filter(terminal_command_blocks::session_id.eq(&session_id))
            .filter(terminal_command_blocks::pane_id.eq(&pane_id))
            .count()
            .get_result::<i64>(&mut connection)
            .expect("command block count should load");
        let receipt_count = terminal_capture_receipts::table
            .filter(terminal_capture_receipts::session_id.eq(&session_id))
            .count()
            .get_result::<i64>(&mut connection)
            .expect("receipt count should load");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].display_text, "git status");
        assert_eq!(history[0].use_count, 1);
        assert_eq!(event_count, 1);
        assert_eq!(command_block_count, 1);
        assert_eq!(receipt_count, 1);
    }

    #[test]
    fn rejects_ui_input_retry_with_same_client_event_id_and_different_payload() {
        let store = test_store("ui-input-retry-conflict");
        let session_id = Uuid::new_v4().to_string();
        let pane_id = Uuid::new_v4().to_string();
        let input = UiInputEventInput {
            session_id: session_id.clone(),
            route: route(),
            title: Some("shell".to_string()),
            launch: None,
            pane_id: pane_id.clone(),
            data: "git status\r".to_string(),
            is_paste: false,
            source_event_id: Some("browser-submit-2".to_string()),
            rows: None,
            cols: None,
            shell_kind: Some("cmd".to_string()),
        };
        store.record_ui_input_event(input.clone()).expect("first ui input should persist");

        let mut conflicting = input;
        conflicting.data = "git branch\r".to_string();
        let error =
            store.record_ui_input_event(conflicting).expect_err("conflicting retry should fail");
        let history =
            store.list_command_history(Some(&session_id), 10).expect("command history should load");

        assert!(
            matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("payload hash mismatch"))
        );
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].display_text, "git status");
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
        assert_eq!(plan.latest_restore_drill_status, None);
        assert!(plan.evidence.iter().any(|evidence| {
            evidence.kind == "authoritative_reads_gate_state" && evidence.value == "disabled"
        }));
    }

    #[test]
    fn runs_integrity_check_and_restore_drill() {
        let store = test_store("integrity-drill");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        let output = store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id.clone(),
                b"durable history\r\n".to_vec(),
            ))
            .expect("segment should persist");
        store
            .write_screen_snapshot(ScreenSnapshotInput {
                id: None,
                session_id: session_id.clone(),
                pane_id: pane_id.clone(),
                writer_generation: writer.id.clone(),
                projection_source: None,
                buffer_kind: None,
                rows: 24,
                cols: 80,
                base_event_seq: output.event_seq_low,
                high_water_event_seq: output.event_seq_high,
                high_water_byte_seq: Some(output.byte_high),
                screen: serde_json::json!({"lines":["durable history"]}),
                parser_version: None,
                projection_version: None,
                metadata: None,
            })
            .expect("screen snapshot should persist");

        let integrity = store.run_integrity_check().expect("integrity check should run");
        let drill = store.run_restore_drill(&session_id).expect("restore drill should run");

        assert_eq!(integrity.result, "passed");
        assert_eq!(drill.result, "passed");
        assert_eq!(drill.restore_guarantee_level, "basic_history");
        assert!(drill.error.is_none());

        let plan = store.restore_plan(&session_id).expect("restore plan should reload");
        assert_eq!(plan.latest_restore_drill_status.as_deref(), Some("passed"));
    }

    #[test]
    fn integrity_check_detects_checksum_mismatch() {
        let store = test_store("integrity-mismatch");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id,
                writer.id,
                b"tamper target\r\n".to_vec(),
            ))
            .expect("segment should persist");
        let mut connection = store.connection().expect("connection should open");
        diesel::update(terminal_stream_segments::table)
            .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
            .execute(&mut connection)
            .expect("test should corrupt checksum");

        let integrity = store.run_integrity_check().expect("integrity check should run");

        assert_eq!(integrity.result, "failed");
        assert!(integrity.error.as_deref().unwrap_or_default().contains("checksum_failures=1"));
        let health = store.list_open_data_health_records(None).expect("health records should list");
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].detection_kind, "checksum_mismatch");
        assert_eq!(health[0].severity, "critical");
        assert_eq!(health[0].action_state, "quarantined");
        assert!(health[0].affected_ref.as_deref().unwrap_or_default().contains("stream_segment"));

        let duplicate_integrity = store.run_integrity_check().expect("second check should run");
        let duplicate_health =
            store.list_open_data_health_records(None).expect("health records should list");
        assert_eq!(duplicate_integrity.result, "failed");
        assert_eq!(duplicate_health.len(), 1);
    }

    #[test]
    fn failed_restore_drill_downgrades_restore_plan() {
        let store = test_store("restore-drill-downgrade");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        let output = store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id.clone(),
                b"visible before corruption\r\n".to_vec(),
            ))
            .expect("segment should persist");
        store
            .write_screen_snapshot(ScreenSnapshotInput {
                id: None,
                session_id: session_id.clone(),
                pane_id,
                writer_generation: writer.id,
                projection_source: None,
                buffer_kind: None,
                rows: 24,
                cols: 80,
                base_event_seq: output.event_seq_low,
                high_water_event_seq: output.event_seq_high,
                high_water_byte_seq: Some(output.byte_high),
                screen: serde_json::json!({"lines":["visible before corruption"]}),
                parser_version: None,
                projection_version: None,
                metadata: None,
            })
            .expect("screen snapshot should persist");
        let mut connection = store.connection().expect("connection should open");
        diesel::update(terminal_stream_segments::table)
            .filter(terminal_stream_segments::id.eq(&output.segment_id))
            .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
            .execute(&mut connection)
            .expect("test should corrupt checksum");

        let drill = store.run_restore_drill(&session_id).expect("restore drill should run");
        let plan = store.restore_plan(&session_id).expect("restore plan should reload");

        assert_eq!(drill.result, "failed");
        assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
        assert_eq!(plan.latest_restore_drill_status.as_deref(), Some("failed"));
        assert!(plan.evidence.iter().any(|evidence| {
            evidence.kind == "latest_restore_drill_status" && evidence.value == "failed"
        }));
    }

    #[test]
    fn creates_vacuum_backup_that_reopens_with_history() {
        let store = test_store("vacuum-backup");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id,
                b"backup history\r\n".to_vec(),
            ))
            .expect("segment should persist");
        let target_path = std::env::temp_dir()
            .join(format!("terminal-persistence-v2-backup-{}.sqlite3", Uuid::new_v4()));

        let backup = store.vacuum_into_backup(&target_path).expect("backup should succeed");
        let backup_store = TerminalPersistenceV2::open_with_config(
            &target_path,
            TerminalPersistenceV2Config::test(),
        )
        .expect("backup should reopen");
        let segments = backup_store
            .list_stream_segments(&session_id, &pane_id, 1, 10)
            .expect("backup should contain history");
        let payload = segments.into_iter().flat_map(|segment| segment.payload).collect::<Vec<_>>();

        assert_eq!(backup.state, "succeeded");
        assert_eq!(backup.quick_check_result.as_deref(), Some("ok"));
        assert_eq!(payload, b"backup history\r\n");

        let _ = std::fs::remove_file(&target_path);
        let _ = std::fs::remove_file(target_path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(target_path.with_extension("sqlite3-shm"));
    }

    #[test]
    fn storage_probe_and_search_documents_are_redacted() {
        let store = test_store("storage-search");
        let (session_id, pane_id, _writer) = session_and_pane(&store);

        let pressure = store.probe_storage_health().expect("storage probe should persist");
        let document = store
            .upsert_redacted_search_document(SearchDocumentInput {
                document_id: None,
                session_id: session_id.clone(),
                pane_id: Some(pane_id),
                command_block_id: None,
                document_kind: None,
                event_seq_low: Some(1),
                event_seq_high: Some(1),
                byte_low: Some(0),
                byte_high: Some(64),
                redaction_profile_id: None,
                raw_text:
                    "curl -H Authorization: Bearer sk_live_secret_token_123456 password=hunter2"
                        .to_string(),
                metadata: None,
            })
            .expect("search document should persist");
        let documents =
            store.list_search_documents(&session_id, 10).expect("search documents should list");

        assert_eq!(pressure.state, "ok");
        assert_eq!(pressure.action_taken, "none");
        assert!(pressure.db_file_bytes.is_some());
        assert_eq!(document.redaction_state, "redacted");
        assert!(!document.text_preview.contains("hunter2"));
        assert!(!document.text_preview.contains("sk_live_secret"));
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].document_id, document.document_id);
    }

    #[test]
    fn export_and_support_are_redacted_by_default_and_raw_is_gated() {
        let store = test_store("privacy-workflows");
        let (session_id, _pane_id, _writer) = session_and_pane(&store);

        let export = store
            .create_export_request(ExportRequestInput {
                id: None,
                session_id: Some(session_id.clone()),
                export_kind: None,
                redaction_profile_id: None,
                include_raw: false,
                output_ref: Some("C:\\temp\\redacted-export.json".to_string()),
                metadata: None,
            })
            .expect("redacted export request should persist");
        let support = store
            .create_support_bundle(SupportBundleInput {
                id: None,
                scope: serde_json::json!({"session_id": session_id}),
                redaction_profile_id: None,
                include_raw: false,
                output_ref: Some("support-bundle.zip".to_string()),
                metadata: None,
            })
            .expect("redacted support bundle should persist");
        let raw_export = store.create_export_request(ExportRequestInput {
            id: None,
            session_id: None,
            export_kind: Some("raw_transcript".to_string()),
            redaction_profile_id: None,
            include_raw: true,
            output_ref: None,
            metadata: None,
        });

        assert!(!export.include_raw);
        assert_eq!(
            export.manifest_json.as_ref().and_then(|value| value["raw_terminal_output"].as_bool()),
            Some(false)
        );
        assert!(!support.include_raw);
        assert_eq!(
            support
                .manifest_json
                .as_ref()
                .and_then(|value| value["excluded_classes"].as_array())
                .map(Vec::len),
            Some(2)
        );
        assert!(
            matches!(raw_export, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("raw history export is disabled"))
        );

        store
            .set_feature_gate_state(
                FeatureGateName::RawHistoryExport,
                FeatureGateState::Enabled,
                Some("test raw export approval"),
            )
            .expect("raw export gate should enable");
        let approved_raw_export = store
            .create_export_request(ExportRequestInput {
                id: None,
                session_id: None,
                export_kind: Some("raw_transcript".to_string()),
                redaction_profile_id: None,
                include_raw: true,
                output_ref: None,
                metadata: None,
            })
            .expect("raw export should persist when gate is enabled");
        assert!(approved_raw_export.include_raw);
    }

    #[test]
    fn delete_request_writes_tombstone_without_deleting_canonical_history() {
        let store = test_store("delete-workflow");
        let (session_id, pane_id, writer) = session_and_pane(&store);
        store
            .append_stream_segment(StreamSegmentInput::terminal_output(
                session_id.clone(),
                pane_id.clone(),
                writer.id,
                b"history must not disappear silently\r\n".to_vec(),
            ))
            .expect("segment should persist");

        let request = store
            .create_delete_request(DeleteRequestInput {
                id: None,
                session_id: Some(session_id.clone()),
                request_kind: Some("user_delete".to_string()),
                policy_id: None,
                requester_ref: Some("local-user".to_string()),
                reason: Some("test delete request".to_string()),
                metadata: None,
            })
            .expect("delete request should persist");
        let tombstone = store
            .complete_delete_request_with_tombstone(
                &request.id,
                "session",
                Some(serde_json::json!({"canonical_delete_deferred": true})),
                None,
            )
            .expect("tombstone should persist");
        let segments = store
            .list_stream_segments(&session_id, &pane_id, 1, 10)
            .expect("canonical history should remain readable");

        assert_eq!(request.state, "pending");
        assert_eq!(tombstone.session_id.as_deref(), Some(session_id.as_str()));
        assert_eq!(tombstone.deleted_scope, "session");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].payload, b"history must not disappear silently\r\n");
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
