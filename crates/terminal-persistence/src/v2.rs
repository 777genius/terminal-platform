use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper,
    connection::SimpleConnection,
    dsl::{insert_into, max, min},
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
            terminal_ai_action_approvals, terminal_ai_context_items, terminal_ai_context_packages,
            terminal_backend_capability_reports, terminal_backup_records,
            terminal_capture_receipts, terminal_clients, terminal_clock_anchors,
            terminal_command_blocks, terminal_command_history_entries, terminal_commit_log,
            terminal_crypto_key_events, terminal_crypto_keys, terminal_data_health_records,
            terminal_db_identity, terminal_delete_requests, terminal_deletion_tombstones,
            terminal_delivery_offsets, terminal_export_requests, terminal_external_artifacts,
            terminal_feature_gates, terminal_history_gaps, terminal_integrity_checks,
            terminal_journal_events, terminal_maintenance_runs, terminal_outbox_messages,
            terminal_panes, terminal_payload_schemas, terminal_prompt_injection_findings,
            terminal_restore_drills, terminal_retention_policies, terminal_screen_snapshots,
            terminal_search_documents, terminal_session_cursors, terminal_sessions,
            terminal_storage_pressure_events, terminal_stream_cursors, terminal_stream_segments,
            terminal_support_bundles, terminal_topology_snapshots, terminal_writer_generations,
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
const MAX_SNAPSHOT_FALLBACK_CANDIDATES: i64 = 64;
const DEFAULT_COMMAND_HISTORY_LIMIT: i64 = 100;
const MAX_COMMAND_HISTORY_LIMIT: i64 = 1_000;
const DEFAULT_DB_PRESSURE_WARNING_BYTES: i64 = 4_i64 * 1024 * 1024 * 1024;
const DEFAULT_WAL_PRESSURE_WARNING_BYTES: i64 = 256_i64 * 1024 * 1024;
const PAYLOAD_SCHEMA_UI_INPUT_V1: &str = "terminal.ui_input.v1";
const PAYLOAD_SCHEMA_HISTORY_GAP_V1: &str = "terminal.history_gap.v1";
const PAYLOAD_SCHEMA_JOURNAL_EVENT_V1: &str = "terminal.journal_event.v1";
const PAYLOAD_SCHEMA_TOPOLOGY_SNAPSHOT_V1: &str = "terminal.topology_snapshot.v1";
const COMMAND_HASH_ALGORITHM: &str = "blake3_keyed_v1";
const COMMAND_HASH_SCOPE: &str = "local_keyed";

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
    pub storage_pressure: StoragePressureConfig,
    pub failpoints: PersistenceFailpoints,
    pub allow_test_plaintext_crypto_keys: bool,
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
            storage_pressure: StoragePressureConfig::default(),
            failpoints: PersistenceFailpoints::default(),
            allow_test_plaintext_crypto_keys: true,
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
            storage_pressure: StoragePressureConfig::default(),
            failpoints: PersistenceFailpoints::default(),
            allow_test_plaintext_crypto_keys: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoragePressureConfig {
    pub db_warning_bytes: i64,
    pub wal_warning_bytes: i64,
}

impl Default for StoragePressureConfig {
    fn default() -> Self {
        Self {
            db_warning_bytes: DEFAULT_DB_PRESSURE_WARNING_BYTES,
            wal_warning_bytes: DEFAULT_WAL_PRESSURE_WARNING_BYTES,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PersistenceFailpoints {
    pub stream_segment_after_segment_insert: bool,
    pub stream_segment_before_transaction_storage_full: bool,
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
        enforce_encryption_startup_policy(&mut connection, &config)?;
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
        load_feature_gate_state(&mut connection, name)
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

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            validate_feature_gate_transition(connection, &self.config, name, state)?;
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
            .execute(connection)?;
            Ok(())
        })
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
            let existing_private_mode = terminal_sessions::table
                .filter(terminal_sessions::id.eq(&session_id))
                .select(terminal_sessions::private_mode)
                .first::<i32>(connection)
                .optional()?
                .unwrap_or(0);
            let row = NewTerminalSessionRow {
                id: session_id.clone(),
                route_json,
                title: input.title,
                launch_json,
                source: input.source.unwrap_or_else(|| "runtime".to_string()),
                durability_profile: durability_profile.as_str().to_string(),
                retention_policy_id,
                private_mode: bool_to_int(input.private_mode || existing_private_mode != 0),
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
        validate_capture_semantics_domain(&input.capture_semantics)?;
        validate_capture_strategy_domain(&input.capture_strategy)?;
        validate_command_boundary_confidence_domain(&input.command_boundary_confidence)?;
        validate_backend_probe_status_domain(&input.probe_status)?;
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

    pub fn mark_backend_capability_reports_stale(
        &self,
        input: BackendCapabilityStaleInput,
    ) -> Result<usize, TerminalPersistenceV2Error> {
        validate_backend_capability_stale_reason(&input.stale_reason)?;
        let mut connection = self.connection()?;
        let mut query = terminal_backend_capability_reports::table
            .filter(terminal_backend_capability_reports::stale_reason.is_null())
            .into_boxed();
        if let Some(session_id) = input.session_id.as_deref() {
            query = query.filter(
                terminal_backend_capability_reports::session_id.eq(Some(session_id.to_string())),
            );
        }
        if let Some(backend_kind) = input.backend_kind.as_deref() {
            query =
                query.filter(terminal_backend_capability_reports::backend_kind.eq(backend_kind));
        }
        if let Some(route_kind) = input.route_kind.as_deref() {
            query = query.filter(terminal_backend_capability_reports::route_kind.eq(route_kind));
        }
        let ids = query
            .select(terminal_backend_capability_reports::id)
            .load::<String>(&mut connection)?;
        if ids.is_empty() {
            return Ok(0);
        }
        let updated = diesel::update(
            terminal_backend_capability_reports::table
                .filter(terminal_backend_capability_reports::id.eq_any(&ids)),
        )
        .set(terminal_backend_capability_reports::stale_reason.eq(Some(input.stale_reason)))
        .execute(&mut connection)?;
        Ok(updated)
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

    pub fn restore_plan(
        &self,
        session_id: &str,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let latest_topology =
            load_latest_valid_topology_snapshot(&mut connection, session_id, now, "restore_plan")?;
        let topology_pane_high_water = latest_topology
            .as_ref()
            .map(|topology| parse_pane_high_water_json(&topology.pane_high_water_json))
            .transpose()?;
        let latest_screen = load_latest_valid_screen_snapshot(
            &mut connection,
            session_id,
            None,
            topology_pane_high_water.as_ref(),
            now,
            "restore_plan",
        )?;
        let segment_count: i64 = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .count()
            .get_result(&mut connection)?;
        let raw_segment_count: i64 = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .filter(terminal_stream_segments::capture_semantics.eq("raw_vt_stream"))
            .count()
            .get_result(&mut connection)?;
        let rendered_segment_count = segment_count - raw_segment_count;
        let stream_event_range = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .select((
                diesel::dsl::min(terminal_stream_segments::event_seq_low),
                diesel::dsl::max(terminal_stream_segments::event_seq_high),
            ))
            .first::<(Option<i64>, Option<i64>)>(&mut connection)?;
        let persisted_gap_count: i64 = terminal_history_gaps::table
            .filter(terminal_history_gaps::session_id.eq(session_id))
            .count()
            .get_result(&mut connection)?;
        let journal_gap_count: i64 = terminal_journal_events::table
            .filter(terminal_journal_events::session_id.eq(session_id))
            .filter(terminal_journal_events::event_type.eq("history_gap"))
            .count()
            .get_result(&mut connection)?;
        let gap_count = persisted_gap_count.max(journal_gap_count);
        let high_water_commit_seq = terminal_commit_log::table
            .filter(terminal_commit_log::session_id.eq(session_id))
            .select(diesel::dsl::max(terminal_commit_log::commit_seq))
            .first::<Option<i64>>(&mut connection)?
            .unwrap_or(0);
        let latest_restore_drill = terminal_restore_drills::table
            .filter(terminal_restore_drills::session_id.eq(session_id))
            .order(terminal_restore_drills::checked_at_ms.desc())
            .select((terminal_restore_drills::id, terminal_restore_drills::result))
            .first::<(String, String)>(&mut connection)
            .optional()?;
        let latest_restore_drill_status =
            latest_restore_drill.as_ref().map(|(_, result)| result.clone());
        let authoritative_reads_gate = terminal_feature_gates::table
            .filter(
                terminal_feature_gates::feature_name
                    .eq(FeatureGateName::TerminalPersistenceV2AuthoritativeReads.as_str()),
            )
            .select(terminal_feature_gates::state)
            .first::<String>(&mut connection)
            .optional()?
            .unwrap_or_else(|| FeatureGateState::Disabled.as_str().to_string());
        let latest_capability_report =
            latest_backend_capability_report(&mut connection, session_id)?;
        let capability_stale = latest_capability_report.as_ref().map(|report| {
            report.expires_at_ms <= now
                || report.stale_reason.is_some()
                || report.probe_status != "passed"
        });
        let has_fresh_raw_capability = latest_capability_report.as_ref().is_some_and(|report| {
            capability_stale == Some(false) && report.capture_semantics == "raw_vt_stream"
        });
        let critical_health_record_count: i64 = terminal_data_health_records::table
            .filter(
                terminal_data_health_records::session_id
                    .eq(Some(session_id.to_string()))
                    .or(terminal_data_health_records::session_id.is_null()),
            )
            .filter(terminal_data_health_records::severity.eq("critical"))
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .count()
            .get_result(&mut connection)?;

        let mut guarantee_level = match (
            segment_count > 0,
            raw_segment_count > 0,
            latest_screen.is_some(),
            latest_topology.is_some(),
            gap_count > 0,
        ) {
            (_, _, _, _, true) => RestoreGuaranteeLevel::DegradedHistory,
            (true, true, true, true, false)
                if latest_restore_drill_status.as_deref() == Some("passed")
                    && has_fresh_raw_capability =>
            {
                RestoreGuaranteeLevel::RawStreamReplay
            }
            (true, _, true, _, false) => RestoreGuaranteeLevel::BasicHistory,
            (false, _, true, _, false) => RestoreGuaranteeLevel::VisualSnapshotOnly,
            _ => RestoreGuaranteeLevel::None,
        };
        if matches!(latest_restore_drill_status.as_deref(), Some("failed" | "degraded")) {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        if authoritative_reads_gate == FeatureGateState::ForceDisabled.as_str() {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        if critical_health_record_count > 0 {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        if let Some(report) = latest_capability_report.as_ref() {
            let stale = report.expires_at_ms <= now
                || report.stale_reason.is_some()
                || report.probe_status != "passed";
            if stale {
                guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
            }
            if report.capture_semantics != "raw_vt_stream"
                && matches!(guarantee_level, RestoreGuaranteeLevel::RawStreamReplay)
            {
                guarantee_level = RestoreGuaranteeLevel::BasicHistory;
            }
        }

        let mut evidence = vec![
            RestoreEvidence {
                kind: "stream_segment_count".to_string(),
                value: segment_count.to_string(),
            },
            RestoreEvidence {
                kind: "raw_stream_segment_count".to_string(),
                value: raw_segment_count.to_string(),
            },
            RestoreEvidence {
                kind: "rendered_stream_segment_count".to_string(),
                value: rendered_segment_count.to_string(),
            },
            RestoreEvidence { kind: "history_gap_count".to_string(), value: gap_count.to_string() },
            RestoreEvidence {
                kind: "authoritative_reads_gate_state".to_string(),
                value: authoritative_reads_gate,
            },
            RestoreEvidence {
                kind: "critical_data_health_record_count".to_string(),
                value: critical_health_record_count.to_string(),
            },
        ];
        if let (Some(event_seq_low), Some(event_seq_high)) = stream_event_range {
            evidence.push(RestoreEvidence {
                kind: "journal_event_range".to_string(),
                value: format!("{session_id}:{event_seq_low}:{event_seq_high}"),
            });
        }
        if let Some(screen) = latest_screen.as_ref() {
            evidence.push(RestoreEvidence {
                kind: "screen_snapshot".to_string(),
                value: screen.id.clone(),
            });
        }
        if let Some(topology) = latest_topology.as_ref() {
            evidence.push(RestoreEvidence {
                kind: "topology_snapshot".to_string(),
                value: topology.id.clone(),
            });
        }
        if let Some(status) = &latest_restore_drill_status {
            evidence.push(RestoreEvidence {
                kind: "latest_restore_drill_status".to_string(),
                value: status.clone(),
            });
        }
        if let Some((drill_id, _)) = &latest_restore_drill {
            evidence.push(RestoreEvidence {
                kind: "restore_drill".to_string(),
                value: drill_id.clone(),
            });
        }
        if let Some(report) = latest_capability_report {
            let stale = report.expires_at_ms <= now
                || report.stale_reason.is_some()
                || report.probe_status != "passed";
            evidence.push(RestoreEvidence {
                kind: "backend_capability_report".to_string(),
                value: report.id.clone(),
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capability_probe_status".to_string(),
                value: report.probe_status,
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capture_strategy".to_string(),
                value: report.capture_strategy,
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capture_semantics".to_string(),
                value: report.capture_semantics,
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capability_stale".to_string(),
                value: stale.to_string(),
            });
            if let Some(reason) = report.stale_reason {
                evidence.push(RestoreEvidence {
                    kind: "backend_capability_stale_reason".to_string(),
                    value: reason,
                });
            }
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
        connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let validation = validate_history_checksums(connection, Some(session_id))?;
            let replay_safety = collect_restore_replay_safety(connection, session_id)?;
            let finished_at_ms = self.config.clock.now_ms();
            let result = if validation.has_failures() {
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
            evidence.extend(replay_safety.to_restore_evidence());
            let evidence_json = Some(serde_json::to_string(&evidence)?);
            let metadata_json = Some(serde_json::to_string(&serde_json::json!({
                "started_at_ms": started_at_ms,
                "validation": validation.to_json(),
                "replay_safety": replay_safety,
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
            insert_into(terminal_restore_drills::table).values(&row).execute(connection)?;
            persist_history_validation_health_records(
                connection,
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
                source_snapshot_id: plan.latest_screen_snapshot_id.clone(),
                error,
            })
        })
    }

    pub fn restore_replay_safety_diagnostics(
        &self,
        session_id: &str,
    ) -> Result<RestoreReplaySafetyRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_restore_replay_safety(&mut connection, session_id)
    }

    pub fn run_integrity_check(&self) -> Result<IntegrityCheckRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let checked_at_ms = self.config.clock.now_ms();
        connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let quick_check = run_quick_check(connection)?;
            let foreign_key_violations = run_foreign_key_check(connection)?;
            let validation = validate_history_checksums(connection, None)?;
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
                    "quick_check={}, foreign_key_violations={}, history_validation_failures={}, checksum_failures={}",
                    details["quick_check"],
                    details["foreign_key_violations"].as_array().map_or(0, Vec::len),
                    validation.failure_count(),
                    validation.checksum_failure_count()
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
            insert_into(terminal_integrity_checks::table).values(&row).execute(connection)?;
            persist_history_validation_health_records(
                connection,
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
            state: input.state.unwrap_or_else(|| "ok".to_string()),
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
        validate_storage_pressure_domain(&row.state, &row.action_taken)?;
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
        let classification =
            classify_storage_pressure(db_file_bytes, wal_file_bytes, self.config.storage_pressure);

        self.record_storage_pressure_event(StoragePressureEventInput {
            id: None,
            state: Some(classification.state.to_string()),
            db_file_bytes,
            wal_file_bytes,
            disk_free_bytes: None,
            temp_free_bytes: None,
            quota_bytes: None,
            action_taken: Some(classification.action_taken.to_string()),
            reason: Some(classification.reason.to_string()),
            metadata: Some(serde_json::json!({
                "db_path_hash": path_hash(&self.path),
                "wal_path_hash": path_hash(&wal_path),
                "db_warning_bytes": self.config.storage_pressure.db_warning_bytes,
                "wal_warning_bytes": self.config.storage_pressure.wal_warning_bytes,
                "db_over_budget": classification.db_over_budget,
                "wal_over_budget": classification.wal_over_budget,
                "no_silent_delete": true,
            })),
        })
    }

    fn record_storage_pressure_write_failure(
        &self,
        operation: &str,
        reason: &str,
        error: Option<String>,
    ) -> Result<StoragePressureRecord, TerminalPersistenceV2Error> {
        let db_file_bytes = file_len_i64(&self.path)?;
        let wal_file_bytes = file_len_i64(&sqlite_sidecar_path(&self.path, "-wal"))?;
        self.record_storage_pressure_event(StoragePressureEventInput {
            id: None,
            state: Some("full".to_string()),
            db_file_bytes,
            wal_file_bytes,
            disk_free_bytes: None,
            temp_free_bytes: None,
            quota_bytes: None,
            action_taken: Some("fail_closed".to_string()),
            reason: Some(reason.to_string()),
            metadata: Some(serde_json::json!({
                "operation": operation,
                "error": error,
                "no_silent_delete": true,
                "canonical_history_preserved": true,
            })),
        })
    }

    pub fn run_maintenance(
        &self,
        input: MaintenanceRunInput,
    ) -> Result<MaintenanceRunRecord, TerminalPersistenceV2Error> {
        let id = input.id.unwrap_or_else(new_id);
        let started_at_ms = self.config.clock.now_ms();
        let run_kind = input.run_kind.unwrap_or_else(|| "scheduled_maintenance".to_string());
        let metadata_json = json_metadata(&input.metadata)?;
        let selected_policy_id = input.selected_policy_id.clone();
        let mut connection = self.connection()?;
        let row = NewMaintenanceRunRow {
            id: id.clone(),
            run_kind: run_kind.clone(),
            state: "running".to_string(),
            selected_policy_id: selected_policy_id.clone(),
            started_at_ms,
            finished_at_ms: None,
            summary_json: None,
            error: None,
            metadata_json,
        };
        insert_into(terminal_maintenance_runs::table).values(&row).execute(&mut connection)?;

        let run_result = self.finish_maintenance_run(
            &id,
            &run_kind,
            started_at_ms,
            input.run_wal_checkpoint,
            input.run_optimize,
            selected_policy_id.as_deref(),
        );
        if let Err(error) = &run_result {
            let _ = self.mark_maintenance_failed(&id, error.to_string());
        }
        run_result
    }

    fn finish_maintenance_run(
        &self,
        id: &str,
        run_kind: &str,
        started_at_ms: i64,
        run_wal_checkpoint: bool,
        run_optimize: bool,
        selected_policy_id: Option<&str>,
    ) -> Result<MaintenanceRunRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let recovery = recover_expired_maintenance_leases(&mut connection, started_at_ms)?;
        let outbox_diagnostics = collect_outbox_diagnostics(&mut connection, started_at_ms)?;
        let compression_diagnostics =
            collect_compression_diagnostics(&mut connection, started_at_ms)?;
        let retention_diagnostics =
            collect_retention_diagnostics(&mut connection, started_at_ms, selected_policy_id)?;
        let wal_checkpoint = if run_wal_checkpoint {
            Some(run_passive_wal_checkpoint(&mut connection)?)
        } else {
            None
        };
        if run_optimize {
            connection.batch_execute("PRAGMA optimize;")?;
        }

        let db_file_bytes = file_len_i64(&self.path)?;
        let wal_file_bytes = file_len_i64(&sqlite_sidecar_path(&self.path, "-wal"))?;
        let finished_at_ms = self.config.clock.now_ms();
        let summary = serde_json::json!({
            "run_kind": run_kind,
            "wal_checkpoint": wal_checkpoint,
            "optimize": {
                "ran": run_optimize,
                "mode": "pragma_optimize"
            },
            "recovery": {
                "checked_at_ms": started_at_ms,
                "stale_outbox_claims_requeued": recovery.stale_outbox_claims_requeued,
                "stale_outbox_claims_quarantined": recovery.stale_outbox_claims_quarantined,
                "stale_writer_generations_marked": recovery.stale_writer_generations_marked
            },
            "outbox": outbox_diagnostics,
            "compression": compression_diagnostics,
            "retention": retention_diagnostics,
            "storage": {
                "db_file_bytes": db_file_bytes,
                "wal_file_bytes": wal_file_bytes,
                "no_silent_delete": true
            },
            "duration_ms": (finished_at_ms - started_at_ms).max(0)
        });
        diesel::update(
            terminal_maintenance_runs::table.filter(terminal_maintenance_runs::id.eq(id)),
        )
        .set((
            terminal_maintenance_runs::state.eq("succeeded"),
            terminal_maintenance_runs::finished_at_ms.eq(Some(finished_at_ms)),
            terminal_maintenance_runs::summary_json.eq(Some(serde_json::to_string(&summary)?)),
            terminal_maintenance_runs::error.eq::<Option<String>>(None),
        ))
        .execute(&mut connection)?;
        load_maintenance_run(&mut connection, id)?.try_into()
    }

    fn mark_maintenance_failed(
        &self,
        id: &str,
        error: String,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        diesel::update(
            terminal_maintenance_runs::table.filter(terminal_maintenance_runs::id.eq(id)),
        )
        .set((
            terminal_maintenance_runs::state.eq("failed"),
            terminal_maintenance_runs::finished_at_ms.eq(Some(self.config.clock.now_ms())),
            terminal_maintenance_runs::error.eq(Some(error)),
        ))
        .execute(&mut connection)?;
        Ok(())
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
        if input.include_raw {
            ensure_no_open_critical_health_records(
                &mut connection,
                input.session_id.as_deref(),
                "raw export",
            )?;
        }
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

    pub fn approve_export_request(
        &self,
        input: ExportApprovalInput,
    ) -> Result<ExportRequestRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let request = load_export_request(connection, &input.export_request_id)?;
            if request.state == "succeeded" || request.state == "failed" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "export request cannot be approved from state {}",
                    request.state
                )));
            }

            let metadata_json = merge_json_field(
                request.metadata_json.as_deref(),
                "approval",
                serde_json::json!({
                    "approved_at_ms": now,
                    "approver_ref_hash": input.approver_ref.as_ref().map(|value| blake3_hash_text(value)),
                    "metadata": input.metadata,
                }),
            )?;

            diesel::update(
                terminal_export_requests::table
                    .filter(terminal_export_requests::id.eq(&input.export_request_id)),
            )
            .set((
                terminal_export_requests::state.eq("approved"),
                terminal_export_requests::approved_at_ms.eq(Some(now)),
                terminal_export_requests::metadata_json.eq(metadata_json),
            ))
            .execute(connection)?;

            ExportRequestRecord::try_from(load_export_request(connection, &input.export_request_id)?)
        })
    }

    pub fn verify_export_artifact(
        &self,
        input: ExportArtifactVerificationInput,
    ) -> Result<ExportArtifactVerificationRecord, TerminalPersistenceV2Error> {
        validate_external_artifact_ref(&input.artifact_ref)?;
        validate_external_artifact_target_ref(&input.artifact_ref, &self.path)?;
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let request = load_export_request(connection, &input.export_request_id)?;
            let artifact_ref_hash = blake3_hash_text(&input.artifact_ref);
            if request.output_ref_hash.as_deref() != Some(artifact_ref_hash.as_str()) {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "export artifact ref does not match request output_ref hash".to_string(),
                ));
            }
            if request.include_raw != 0 && request.approved_at_ms.is_none() {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "raw export must be explicitly approved before artifact verification"
                        .to_string(),
                ));
            }

            let artifact = terminal_external_artifacts::table
                .filter(terminal_external_artifacts::artifact_ref_hash.eq(&artifact_ref_hash))
                .select(ExternalArtifactRow::as_select())
                .first::<ExternalArtifactRow>(connection)?;
            if artifact.artifact_kind != "export_file" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "export artifact verification requires export_file artifact, got {}",
                    artifact.artifact_kind
                )));
            }
            if artifact.state != "available" && artifact.state != "verified" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "export artifact must be available or verified, got {}",
                    artifact.state
                )));
            }

            let raw_export = request.include_raw != 0;
            let encrypted_required = raw_export || input.require_encrypted;
            if encrypted_required && artifact.encryption_state != "encrypted" {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "raw or encrypted-required export must complete into an encrypted artifact"
                        .to_string(),
                ));
            }
            if encrypted_required && artifact.key_ref.is_none() {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "encrypted export artifact must reference an opaque key id".to_string(),
                ));
            }
            if encrypted_required
                && (artifact.checksum_algorithm.is_none() || artifact.checksum.is_none())
            {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "encrypted export artifact must include a stored-bytes checksum".to_string(),
                ));
            }

            let artifact_id = artifact.id.clone();
            let artifact_kind = artifact.artifact_kind.clone();
            let encryption_state = artifact.encryption_state.clone();
            let checksum_algorithm = artifact.checksum_algorithm.clone();
            let checksum = artifact.checksum.clone();
            let verification = serde_json::json!({
                "artifact_id": artifact_id,
                "artifact_ref_hash": artifact_ref_hash.clone(),
                "artifact_ref_stored": false,
                "artifact_kind": artifact_kind,
                "artifact_state": "verified",
                "encryption_state": encryption_state,
                "key_ref_present": artifact.key_ref.is_some(),
                "checksum_algorithm": checksum_algorithm,
                "checksum": checksum,
                "size_bytes": artifact.size_bytes,
                "verified_at_ms": now,
                "raw_export": raw_export,
                "encrypted_required": encrypted_required,
                "metadata": input.metadata,
            });
            let manifest_json = merge_json_field(
                request.manifest_json.as_deref(),
                "artifact_verification",
                verification.clone(),
            )?;

            diesel::update(
                terminal_external_artifacts::table
                    .filter(terminal_external_artifacts::id.eq(&artifact.id)),
            )
            .set((
                terminal_external_artifacts::state.eq("verified"),
                terminal_external_artifacts::verified_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;

            diesel::update(
                terminal_export_requests::table
                    .filter(terminal_export_requests::id.eq(&input.export_request_id)),
            )
            .set((
                terminal_export_requests::state.eq("succeeded"),
                terminal_export_requests::completed_at_ms.eq(Some(now)),
                terminal_export_requests::manifest_json.eq(manifest_json),
                terminal_export_requests::error.eq(Option::<String>::None),
            ))
            .execute(connection)?;

            Ok(ExportArtifactVerificationRecord {
                export_request_id: input.export_request_id,
                artifact_id: artifact.id,
                artifact_ref_hash,
                export_state: "succeeded".to_string(),
                artifact_state: "verified".to_string(),
                encryption_state: artifact.encryption_state,
                raw_export,
                encrypted_required,
                verified_at_ms: now,
                checksum_algorithm: artifact.checksum_algorithm,
                checksum: artifact.checksum,
                manifest_json: verification,
            })
        })
    }

    pub fn create_support_bundle(
        &self,
        input: SupportBundleInput,
    ) -> Result<SupportBundleRecord, TerminalPersistenceV2Error> {
        if input.include_raw {
            self.ensure_raw_history_export_enabled()?;
        }
        if let Some(output_ref) = input.output_ref.as_deref() {
            validate_external_artifact_ref(output_ref)?;
            validate_external_artifact_target_ref(output_ref, &self.path)?;
        }
        let mut connection = self.connection()?;
        if input.include_raw {
            ensure_no_open_critical_health_records(&mut connection, None, "raw support bundle")?;
        }
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

    pub fn support_bundle_diagnostics(
        &self,
        support_bundle_id: &str,
    ) -> Result<SupportBundleDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let bundle = load_support_bundle(&mut connection, support_bundle_id)?;
        build_support_bundle_diagnostics(
            &mut connection,
            &self.path,
            &self.config,
            &bundle,
            self.config.clock.now_ms(),
        )
    }

    pub fn complete_support_bundle(
        &self,
        input: SupportBundleCompletionInput,
    ) -> Result<SupportBundleRecord, TerminalPersistenceV2Error> {
        if let Some(artifact_ref) = input.artifact_ref.as_deref() {
            validate_external_artifact_ref(artifact_ref)?;
            validate_external_artifact_target_ref(artifact_ref, &self.path)?;
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let bundle = load_support_bundle(connection, &input.support_bundle_id)?;
            if bundle.state == "succeeded" || bundle.state == "failed" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "support bundle cannot be completed from state {}",
                    bundle.state
                )));
            }
            if bundle.include_raw != 0 {
                if load_feature_gate_state(connection, FeatureGateName::RawHistoryExport)?
                    != FeatureGateState::Enabled
                {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "raw support bundle completion requires raw history export gate"
                            .to_string(),
                    ));
                }
                ensure_no_open_critical_health_records(connection, None, "raw support bundle")?;
            }

            let artifact_verification = if let Some(artifact_ref) = input.artifact_ref.as_deref() {
                let artifact_ref_hash = blake3_hash_text(artifact_ref);
                if bundle.output_ref_hash.as_deref() != Some(artifact_ref_hash.as_str()) {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "support bundle artifact ref does not match request output_ref hash"
                            .to_string(),
                    ));
                }
                let artifact = terminal_external_artifacts::table
                    .filter(terminal_external_artifacts::artifact_ref_hash.eq(&artifact_ref_hash))
                    .select(ExternalArtifactRow::as_select())
                    .first::<ExternalArtifactRow>(connection)?;
                if artifact.artifact_kind != "support_bundle" {
                    return Err(TerminalPersistenceV2Error::InvalidData(format!(
                        "support bundle completion requires support_bundle artifact, got {}",
                        artifact.artifact_kind
                    )));
                }
                if artifact.state != "available" && artifact.state != "verified" {
                    return Err(TerminalPersistenceV2Error::InvalidData(format!(
                        "support bundle artifact must be available or verified, got {}",
                        artifact.state
                    )));
                }
                if bundle.include_raw != 0 && artifact.encryption_state != "encrypted" {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "raw support bundle must complete into an encrypted artifact".to_string(),
                    ));
                }
                if artifact.encryption_state == "encrypted"
                    && (artifact.key_ref.is_none()
                        || artifact.checksum_algorithm.is_none()
                        || artifact.checksum.is_none())
                {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "encrypted support bundle artifact must include key ref and checksum"
                            .to_string(),
                    ));
                }

                diesel::update(
                    terminal_external_artifacts::table
                        .filter(terminal_external_artifacts::id.eq(&artifact.id)),
                )
                .set((
                    terminal_external_artifacts::state.eq("verified"),
                    terminal_external_artifacts::verified_at_ms.eq(Some(now)),
                ))
                .execute(connection)?;

                Some(serde_json::json!({
                    "artifact_id": artifact.id,
                    "artifact_ref_hash": artifact_ref_hash,
                    "artifact_ref_stored": false,
                    "artifact_kind": artifact.artifact_kind,
                    "artifact_state": "verified",
                    "encryption_state": artifact.encryption_state,
                    "key_ref_present": artifact.key_ref.is_some(),
                    "checksum_algorithm": artifact.checksum_algorithm,
                    "checksum": artifact.checksum,
                    "size_bytes": artifact.size_bytes,
                    "verified_at_ms": now,
                }))
            } else {
                None
            };

            let diagnostics = build_support_bundle_diagnostics(
                connection,
                &self.path,
                &self.config,
                &bundle,
                now,
            )?;
            let mut manifest = bundle
                .manifest_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_else(|| {
                    privacy_manifest("support_bundle", bundle.include_raw != 0, None)
                });
            if !manifest.is_object() {
                manifest = serde_json::json!({ "legacy_manifest_value": manifest });
            }
            let manifest_object = manifest.as_object_mut().expect("manifest is object");
            manifest_object.insert("diagnostics".to_string(), diagnostics.manifest_json.clone());
            if let Some(artifact_verification) = artifact_verification {
                manifest_object.insert("artifact_verification".to_string(), artifact_verification);
            }
            manifest_object.insert(
                "completion".to_string(),
                serde_json::json!({
                    "completed_at_ms": now,
                    "metadata": input.metadata,
                    "raw_content_included": bundle.include_raw != 0,
                    "raw_content_included_by_default": false,
                }),
            );

            diesel::update(
                terminal_support_bundles::table
                    .filter(terminal_support_bundles::id.eq(&input.support_bundle_id)),
            )
            .set((
                terminal_support_bundles::state.eq("succeeded"),
                terminal_support_bundles::completed_at_ms.eq(Some(now)),
                terminal_support_bundles::manifest_json.eq(Some(serde_json::to_string(&manifest)?)),
                terminal_support_bundles::error.eq(Option::<String>::None),
            ))
            .execute(connection)?;

            SupportBundleRecord::try_from(load_support_bundle(
                connection,
                &input.support_bundle_id,
            )?)
        })
    }

    pub fn register_crypto_key(
        &self,
        input: CryptoKeyInput,
    ) -> Result<CryptoKeyRecord, TerminalPersistenceV2Error> {
        validate_crypto_key_domain(
            &input.key_kind,
            &input.protection_kind,
            input.state.as_deref(),
        )?;
        validate_crypto_key_ref(&input.key_ref)?;
        if input.protection_kind == "test_plaintext"
            && !self.config.allow_test_plaintext_crypto_keys
        {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "test_plaintext crypto keys are allowed only in test configuration".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let state = input.state.unwrap_or_else(|| "active".to_string());
        let row = NewCryptoKeyRow {
            id: input.id.unwrap_or_else(new_id),
            key_kind: input.key_kind,
            key_ref: input.key_ref,
            protection_kind: input.protection_kind,
            state,
            created_at_ms: now,
            rotated_at_ms: None,
            destroyed_at_ms: None,
            capability_report_json: input
                .capability_report
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            error_json: input.error.as_ref().map(serde_json::to_string).transpose()?,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_crypto_keys::table).values(&row).execute(&mut connection)?;
        Ok(CryptoKeyRecord::try_from(row)?)
    }

    pub fn record_crypto_key_event(
        &self,
        input: CryptoKeyEventInput,
    ) -> Result<CryptoKeyEventRecord, TerminalPersistenceV2Error> {
        validate_crypto_key_event_domain(&input.event_kind, &input.status)?;
        let mut connection = self.connection()?;
        let row = NewCryptoKeyEventRow {
            id: input.id.unwrap_or_else(new_id),
            key_id: input.key_id,
            event_kind: input.event_kind,
            actor: input.actor,
            occurred_at_ms: input.occurred_at_ms.unwrap_or_else(|| self.config.clock.now_ms()),
            status: input.status,
            error_json: input.error.as_ref().map(serde_json::to_string).transpose()?,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_crypto_key_events::table).values(&row).execute(&mut connection)?;
        Ok(CryptoKeyEventRecord::try_from(row)?)
    }

    pub fn complete_crypto_erase(
        &self,
        input: CryptoEraseInput,
    ) -> Result<CryptoEraseRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let key = terminal_crypto_keys::table
                .filter(terminal_crypto_keys::id.eq(&input.key_id))
                .select(CryptoKeyRow::as_select())
                .first::<CryptoKeyRow>(connection)?;
            let key_ref_hash = blake3_hash_text(&key.key_ref);
            diesel::update(terminal_crypto_keys::table.filter(terminal_crypto_keys::id.eq(&key.id)))
                .set((
                    terminal_crypto_keys::state.eq("destroyed"),
                    terminal_crypto_keys::destroyed_at_ms.eq(Some(now)),
                ))
                .execute(connection)?;

            let delete_request = NewDeleteRequestRow {
                id: input.id.unwrap_or_else(new_id),
                session_id: input.session_id.clone(),
                request_kind: "crypto_erase".to_string(),
                state: "completed".to_string(),
                policy_id: None,
                requested_at_ms: now,
                approved_at_ms: Some(now),
                completed_at_ms: Some(now),
                requester_ref_hash: input.requester_ref.map(|value| blake3_hash_text(&value)),
                reason: input.reason,
                metadata_json: json_metadata(&input.metadata)?,
            };
            insert_into(terminal_delete_requests::table)
                .values(&delete_request)
                .execute(connection)?;

            let event = NewCryptoKeyEventRow {
                id: new_id(),
                key_id: Some(key.id.clone()),
                event_kind: "destroyed".to_string(),
                actor: "crypto_erase".to_string(),
                occurred_at_ms: now,
                status: "succeeded".to_string(),
                error_json: None,
                metadata_json: Some(serde_json::to_string(&serde_json::json!({
                    "delete_request_id": delete_request.id,
                    "key_ref_hash": key_ref_hash,
                    "key_material_exported": false
                }))?),
            };
            insert_into(terminal_crypto_key_events::table).values(&event).execute(connection)?;

            let evidence = serde_json::json!({
                "key_id": key.id,
                "key_kind": key.key_kind,
                "key_ref_hash": key_ref_hash,
                "secure_deletion_limitation": "sqlite_pages_may_retain_old_plaintext_until_vacuum_or_storage_reuse",
                "canonical_history_deleted": false,
                "key_material_exported": false
            });
            let tombstone = NewDeletionTombstoneRow {
                id: new_id(),
                delete_request_id: Some(delete_request.id.clone()),
                session_id: input.session_id,
                deleted_scope: "crypto_key".to_string(),
                policy_id: None,
                deleted_at_ms: now,
                evidence_json: Some(serde_json::to_string(&evidence)?),
                metadata_json: None,
            };
            insert_into(terminal_deletion_tombstones::table)
                .values(&tombstone)
                .execute(connection)?;

            Ok(CryptoEraseRecord {
                key_id: key.id,
                key_ref_hash,
                delete_request_id: delete_request.id,
                tombstone_id: tombstone.id,
                state: "completed".to_string(),
                secure_deletion_limitation: evidence["secure_deletion_limitation"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            })
        })
    }

    pub fn encryption_capability_state(
        &self,
    ) -> Result<EncryptionCapabilityRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        encryption_capability_state_for_connection(&mut connection, &self.config)
    }

    pub fn record_external_artifact(
        &self,
        input: ExternalArtifactInput,
    ) -> Result<ExternalArtifactRecord, TerminalPersistenceV2Error> {
        validate_external_artifact_domain(
            &input.artifact_kind,
            input.state.as_deref(),
            input.encryption_state.as_deref(),
        )?;
        validate_external_artifact_ref(&input.artifact_ref)?;
        validate_external_artifact_target_ref(&input.artifact_ref, &self.path)?;
        if let Some(size_bytes) = input.size_bytes
            && size_bytes < 0
        {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "external artifact size_bytes must not be negative".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewExternalArtifactRow {
            id: input.id.unwrap_or_else(new_id),
            artifact_kind: input.artifact_kind,
            artifact_ref_hash: blake3_hash_text(&input.artifact_ref),
            state: input.state.unwrap_or_else(|| "planned".to_string()),
            encryption_state: input.encryption_state.unwrap_or_else(|| "plaintext".to_string()),
            key_ref: input.key_ref,
            checksum_algorithm: input.checksum_algorithm,
            checksum: input.checksum,
            size_bytes: input.size_bytes,
            created_at_ms: now,
            verified_at_ms: input.verified_at_ms,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_external_artifacts::table).values(&row).execute(&mut connection)?;
        Ok(ExternalArtifactRecord::try_from(row)?)
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

    pub fn create_ai_context_package(
        &self,
        input: AiContextPackageInput,
    ) -> Result<AiContextPackageRecord, TerminalPersistenceV2Error> {
        if input.include_raw {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "AI context packages cannot include raw transcript by default".to_string(),
            ));
        }
        let item_limit = input.max_items.unwrap_or(32).clamp(1, 256);
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let id = input.id.unwrap_or_else(new_id);
            let row = NewAiContextPackageRow {
                id: id.clone(),
                session_id: input.session_id.clone(),
                pane_id: input.pane_id.clone(),
                state: "ready".to_string(),
                redaction_profile_id: input
                    .redaction_profile_id
                    .or_else(|| Some("default".to_string())),
                include_raw: 0,
                requested_at_ms: now,
                built_at_ms: Some(now),
                item_count: 0,
                manifest_json: None,
                metadata_json: json_metadata(&input.metadata)?,
            };
            insert_into(terminal_ai_context_packages::table)
                .values(&row)
                .execute(connection)?;

            let mut inserted_items = Vec::new();
            inserted_items.extend(insert_ai_context_items_from_command_history(
                connection,
                &id,
                input.session_id.as_deref(),
                input.pane_id.as_deref(),
                item_limit / 2,
            )?);
            let remaining = item_limit.saturating_sub(i64::try_from(inserted_items.len()).unwrap_or(0));
            if remaining > 0 {
                inserted_items.extend(insert_ai_context_items_from_search_documents(
                    connection,
                    &id,
                    input.session_id.as_deref(),
                    input.pane_id.as_deref(),
                    remaining,
                )?);
            }

            let findings = insert_prompt_injection_findings_for_items(connection, &id, &inserted_items, now)?;
            let manifest = serde_json::json!({
                "kind": "ai_context",
                "session_id": input.session_id,
                "pane_id": input.pane_id,
                "include_raw": false,
                "raw_terminal_output": false,
                "raw_command_text": false,
                "raw_content_included": false,
                "data_only": true,
                "prompt_injection_text_is_data": true,
                "action_approval_required": true,
                "item_count": inserted_items.len(),
                "prompt_injection_finding_count": findings,
                "redaction_profile_id": row.redaction_profile_id,
                "included_classes": ["class_public_diagnostic", "class_local_metadata", "class_user_context_redacted"],
                "excluded_classes": ["class_sensitive_content", "class_secret_material"],
            });
            diesel::update(
                terminal_ai_context_packages::table.filter(terminal_ai_context_packages::id.eq(&id)),
            )
            .set((
                terminal_ai_context_packages::item_count.eq(i64::try_from(inserted_items.len()).unwrap_or(i64::MAX)),
                terminal_ai_context_packages::manifest_json.eq(Some(serde_json::to_string(&manifest)?)),
            ))
            .execute(connection)?;

            AiContextPackageRecord::try_from(load_ai_context_package(connection, &id)?)
        })
    }

    pub fn list_ai_context_items(
        &self,
        package_id: &str,
    ) -> Result<Vec<AiContextItemRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_ai_context_items::table
            .filter(terminal_ai_context_items::package_id.eq(package_id))
            .order(terminal_ai_context_items::source_kind.asc())
            .select(AiContextItemRow::as_select())
            .load::<AiContextItemRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub fn list_prompt_injection_findings(
        &self,
        package_id: &str,
    ) -> Result<Vec<PromptInjectionFindingRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_prompt_injection_findings::table
            .filter(terminal_prompt_injection_findings::package_id.eq(Some(package_id.to_string())))
            .order(terminal_prompt_injection_findings::detected_at_ms.desc())
            .select(PromptInjectionFindingRow::as_select())
            .load::<PromptInjectionFindingRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub fn request_ai_action_approval(
        &self,
        input: AiActionApprovalInput,
    ) -> Result<AiActionApprovalRecord, TerminalPersistenceV2Error> {
        validate_ai_action_kind(&input.action_kind)?;
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewAiActionApprovalRow {
            id: input.id.unwrap_or_else(new_id),
            package_id: Some(input.package_id),
            action_kind: input.action_kind,
            state: "pending".to_string(),
            requester_ref_hash: input.requester_ref.map(|value| blake3_hash_text(&value)),
            approver_ref_hash: None,
            requested_at_ms: now,
            decided_at_ms: None,
            expires_at_ms: input.expires_at_ms,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_ai_action_approvals::table).values(&row).execute(&mut connection)?;
        Ok(AiActionApprovalRecord::try_from(row)?)
    }

    pub fn decide_ai_action_approval(
        &self,
        input: AiActionDecisionInput,
    ) -> Result<AiActionApprovalRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let existing = terminal_ai_action_approvals::table
            .filter(terminal_ai_action_approvals::id.eq(&input.approval_id))
            .select(AiActionApprovalRow::as_select())
            .first::<AiActionApprovalRow>(&mut connection)?;
        if existing.state != "pending" {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "AI action approval cannot be decided from state {}",
                existing.state
            )));
        }
        let state = if input.approved { "approved" } else { "denied" };
        let metadata_json = merge_json_field(
            existing.metadata_json.as_deref(),
            "decision",
            serde_json::json!({
                "approved": input.approved,
                "decided_at_ms": now,
                "metadata": input.metadata,
            }),
        )?;
        diesel::update(
            terminal_ai_action_approvals::table
                .filter(terminal_ai_action_approvals::id.eq(&input.approval_id)),
        )
        .set((
            terminal_ai_action_approvals::state.eq(state),
            terminal_ai_action_approvals::approver_ref_hash
                .eq(input.approver_ref.map(|value| blake3_hash_text(&value))),
            terminal_ai_action_approvals::decided_at_ms.eq(Some(now)),
            terminal_ai_action_approvals::metadata_json.eq(metadata_json),
        ))
        .execute(&mut connection)?;
        AiActionApprovalRecord::try_from(load_ai_action_approval(
            &mut connection,
            &input.approval_id,
        )?)
    }

    pub fn vacuum_into_backup(
        &self,
        target_path: impl AsRef<Path>,
    ) -> Result<BackupRecord, TerminalPersistenceV2Error> {
        let target_path = prepare_vacuum_backup_target(&self.path, target_path.as_ref())?;

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
        let now = self.config.clock.now_ms();

        let latest_topology = load_latest_valid_topology_snapshot(
            &mut connection,
            session_id,
            now,
            "hydrate_pane_history",
        )?;
        let topology_pane_high_water = latest_topology
            .as_ref()
            .map(|topology| parse_pane_high_water_json(&topology.pane_high_water_json))
            .transpose()?;
        let latest_screen_snapshot = load_latest_valid_screen_snapshot(
            &mut connection,
            session_id,
            Some(pane_id),
            topology_pane_high_water.as_ref(),
            now,
            "hydrate_pane_history",
        )?
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
            if let Some(failure) = stream_segment_hydration_failure(&row) {
                persist_hydration_segment_failure(
                    &mut connection,
                    session_id,
                    &row,
                    &failure,
                    now,
                )?;
                continue;
            }
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

    fn is_session_private(&self, session_id: &str) -> Result<bool, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        session_private_mode(&mut connection, session_id)
    }
}

mod models;
pub use models::*;

mod rows;
pub use rows::TerminalDbIdentityRow;
use rows::*;

mod integrity;
use integrity::*;

mod repository;
use repository::*;

fn collect_outbox_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<OutboxDiagnosticsRecord, TerminalPersistenceV2Error> {
    let pending_count = count_outbox_state(connection, "pending")?;
    let due_pending_count = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.le(now))
        .count()
        .get_result::<i64>(connection)?;
    let claimed_count = count_outbox_state(connection, "claimed")?;
    let stale_claim_count = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("claimed"))
        .filter(terminal_outbox_messages::claimed_until_ms.le(Some(now)))
        .count()
        .get_result::<i64>(connection)?;
    let done_count = count_outbox_state(connection, "done")?;
    let failed_count = count_outbox_state(connection, "failed")?;
    let quarantined_count = count_outbox_state(connection, "quarantined")?;
    let oldest_due_pending_created_at = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.le(now))
        .select(min(terminal_outbox_messages::created_at_ms))
        .first::<Option<i64>>(connection)?;
    let next_pending_run_at = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.gt(now))
        .select(min(terminal_outbox_messages::next_run_at_ms))
        .first::<Option<i64>>(connection)?;

    Ok(OutboxDiagnosticsRecord {
        generated_at_ms: now,
        pending_count,
        due_pending_count,
        claimed_count,
        stale_claim_count,
        done_count,
        failed_count,
        quarantined_count,
        oldest_due_pending_age_ms: oldest_due_pending_created_at
            .map(|created_at_ms| (now - created_at_ms).max(0)),
        next_pending_due_in_ms: next_pending_run_at.map(|run_at_ms| (run_at_ms - now).max(0)),
    })
}

fn count_outbox_state(
    connection: &mut SqliteConnection,
    state_name: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq(state_name))
        .count()
        .get_result::<i64>(connection)?)
}

fn collect_compression_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<CompressionDiagnosticsRecord, TerminalPersistenceV2Error> {
    let feature_gate_state = terminal_feature_gates::table
        .filter(
            terminal_feature_gates::feature_name
                .eq(FeatureGateName::SegmentCompressionZstd.as_str()),
        )
        .select(terminal_feature_gates::state)
        .first::<String>(connection)
        .optional()?
        .unwrap_or_else(|| FeatureGateState::Disabled.as_str().to_string());
    let raw_segment_count = count_stream_segments_by_compression(connection, "none")?;
    let zstd_segment_count = count_stream_segments_by_compression(connection, "zstd")?;
    let unsupported_segment_count = terminal_stream_segments::table
        .filter(terminal_stream_segments::compression.ne("none"))
        .filter(terminal_stream_segments::compression.ne("zstd"))
        .count()
        .get_result::<i64>(connection)?;
    let rewrite_candidate_count = if feature_gate_state == FeatureGateState::Enabled.as_str() {
        raw_segment_count
    } else {
        0
    };
    let action_taken = if feature_gate_state == FeatureGateState::Enabled.as_str() {
        "skipped_restore_drill_guard"
    } else {
        "skipped_feature_disabled"
    }
    .to_string();

    Ok(CompressionDiagnosticsRecord {
        generated_at_ms: now,
        feature_gate_state,
        raw_segment_count,
        zstd_segment_count,
        unsupported_segment_count,
        rewrite_candidate_count,
        segments_rewritten: 0,
        restore_drill_required: true,
        action_taken,
    })
}

fn count_stream_segments_by_compression(
    connection: &mut SqliteConnection,
    compression: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_stream_segments::table
        .filter(terminal_stream_segments::compression.eq(compression))
        .count()
        .get_result::<i64>(connection)?)
}

#[derive(Debug, Clone)]
struct InsertedAiContextItem {
    id: String,
    content_preview: String,
}

fn insert_ai_context_items_from_command_history(
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

fn insert_ai_context_items_from_search_documents(
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

fn insert_prompt_injection_findings_for_items(
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

fn collect_restore_replay_safety(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<RestoreReplaySafetyRecord, TerminalPersistenceV2Error> {
    let segments = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .select(terminal_stream_segments::payload)
        .load::<Vec<u8>>(connection)?;
    let mut record = RestoreReplaySafetyRecord {
        session_id: session_id.to_string(),
        scanned_segment_count: i64::try_from(segments.len()).unwrap_or(i64::MAX),
        osc52_clipboard_count: 0,
        title_sequence_count: 0,
        hyperlink_sequence_count: 0,
        cwd_sequence_count: 0,
        shell_marker_sequence_count: 0,
        bel_byte_count: 0,
        side_effects_suppressed: true,
        prompt_injection_text_is_data: true,
    };
    for payload in segments {
        record.osc52_clipboard_count += count_byte_pattern(&payload, b"\x1b]52;");
        record.title_sequence_count +=
            count_byte_pattern(&payload, b"\x1b]0;") + count_byte_pattern(&payload, b"\x1b]2;");
        record.hyperlink_sequence_count += count_byte_pattern(&payload, b"\x1b]8;");
        record.cwd_sequence_count += count_byte_pattern(&payload, b"\x1b]7;");
        record.shell_marker_sequence_count +=
            count_byte_pattern(&payload, b"\x1b]133;") + count_byte_pattern(&payload, b"\x1b]633;");
        record.bel_byte_count += payload.iter().filter(|byte| **byte == 0x07).count() as i64;
    }
    Ok(record)
}

fn count_byte_pattern(haystack: &[u8], needle: &[u8]) -> i64 {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    haystack.windows(needle.len()).filter(|window| *window == needle).count() as i64
}

fn collect_retention_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
    selected_policy_id: Option<&str>,
) -> Result<RetentionDiagnosticsRecord, TerminalPersistenceV2Error> {
    let policy_id = selected_policy_id.unwrap_or(DEFAULT_RETENTION_POLICY_ID);
    let (policy_id, policy_kind, pressure_behavior, raw_history_prune_behavior) =
        terminal_retention_policies::table
            .filter(terminal_retention_policies::id.eq(policy_id))
            .select((
                terminal_retention_policies::id,
                terminal_retention_policies::policy_kind,
                terminal_retention_policies::pressure_behavior,
                terminal_retention_policies::raw_history_prune_behavior,
            ))
            .first::<(String, String, String, String)>(connection)?;
    let sessions_scanned = terminal_sessions::table
        .filter(terminal_sessions::retention_policy_id.eq(&policy_id))
        .count()
        .get_result::<i64>(connection)?;
    let action_taken = match (pressure_behavior.as_str(), raw_history_prune_behavior.as_str()) {
        ("warn_only", "never_silent") => "warn_only_no_delete",
        (_, "request_only") => "warn_only_delete_request_required",
        _ => "warn_only_no_silent_delete",
    }
    .to_string();

    Ok(RetentionDiagnosticsRecord {
        generated_at_ms: now,
        policy_id,
        policy_kind,
        pressure_behavior,
        raw_history_prune_behavior,
        sessions_scanned,
        scan_mode: "warn_only".to_string(),
        maintenance_deletes_raw_history: false,
        action_taken,
    })
}

fn build_support_bundle_diagnostics(
    connection: &mut SqliteConnection,
    db_path: &Path,
    config: &TerminalPersistenceV2Config,
    bundle: &SupportBundleRow,
    now: i64,
) -> Result<SupportBundleDiagnosticsRecord, TerminalPersistenceV2Error> {
    let scope_hash = blake3_hash_text(&bundle.scope_json);
    let feature_gates = terminal_feature_gates::table
        .select((
            terminal_feature_gates::feature_name,
            terminal_feature_gates::state,
            terminal_feature_gates::rollout_scope,
        ))
        .order(terminal_feature_gates::feature_name.asc())
        .load::<(String, String, String)>(connection)?
        .into_iter()
        .map(|(feature_name, state, scope)| {
            serde_json::json!({
                "feature_name": feature_name,
                "state": state,
                "scope": scope,
            })
        })
        .collect::<Vec<_>>();
    let open_health_count = terminal_data_health_records::table
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .count()
        .get_result::<i64>(connection)?;
    let open_critical_health_count = terminal_data_health_records::table
        .filter(terminal_data_health_records::severity.eq("critical"))
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .count()
        .get_result::<i64>(connection)?;
    let restore_drill_passed_count = terminal_restore_drills::table
        .filter(terminal_restore_drills::result.eq("passed"))
        .count()
        .get_result::<i64>(connection)?;
    let restore_drill_failed_count = terminal_restore_drills::table
        .filter(terminal_restore_drills::result.eq("failed"))
        .count()
        .get_result::<i64>(connection)?;
    let latest_restore_drill_status = terminal_restore_drills::table
        .order(terminal_restore_drills::checked_at_ms.desc())
        .select(terminal_restore_drills::result)
        .first::<String>(connection)
        .optional()?;
    let outbox = collect_outbox_diagnostics(connection, now)?;
    let compression = collect_compression_diagnostics(connection, now)?;
    let retention = collect_retention_diagnostics(connection, now, None)?;
    let encryption = encryption_capability_state_for_connection(connection, config)?;
    let session_count = terminal_sessions::table.count().get_result::<i64>(connection)?;
    let pane_count = terminal_panes::table.count().get_result::<i64>(connection)?;
    let stream_segment_count =
        terminal_stream_segments::table.count().get_result::<i64>(connection)?;
    let command_history_count =
        terminal_command_history_entries::table.count().get_result::<i64>(connection)?;
    let search_document_count =
        terminal_search_documents::table.count().get_result::<i64>(connection)?;
    let external_artifact_count =
        terminal_external_artifacts::table.count().get_result::<i64>(connection)?;
    let db_file_bytes = file_len_i64(db_path)?;
    let wal_file_bytes = file_len_i64(&sqlite_sidecar_path(db_path, "-wal"))?;

    let include_raw = bundle.include_raw != 0;
    let manifest = serde_json::json!({
        "support_bundle_id": bundle.id,
        "generated_at_ms": now,
        "scope_hash": scope_hash,
        "scope_value_stored_in_bundle_row_only": true,
        "redaction_profile_id": bundle.redaction_profile_id,
        "include_raw": include_raw,
        "raw_content_included": include_raw,
        "raw_content_included_by_default": false,
        "excluded_classes": if include_raw {
            vec!["class_secret_material"]
        } else {
            vec!["class_sensitive_content", "class_secret_material"]
        },
        "included_classes": if include_raw {
            vec!["class_public_diagnostic", "class_local_metadata", "class_user_context", "class_sensitive_content"]
        } else {
            vec!["class_public_diagnostic", "class_local_metadata", "class_user_context_redacted"]
        },
        "raw_terminal_output_rows_serialized": false,
        "raw_command_text_rows_serialized": false,
        "raw_paths_serialized": false,
        "crypto_key_refs_serialized": false,
        "db_path_hash": path_hash(db_path),
        "wal_path_hash": path_hash(&sqlite_sidecar_path(db_path, "-wal")),
        "storage": {
            "db_file_bytes": db_file_bytes,
            "wal_file_bytes": wal_file_bytes,
        },
        "counts": {
            "sessions": session_count,
            "panes": pane_count,
            "stream_segments": stream_segment_count,
            "command_history_entries": command_history_count,
            "search_documents": search_document_count,
            "external_artifacts": external_artifact_count,
        },
        "data_health": {
            "open_record_count": open_health_count,
            "open_critical_record_count": open_critical_health_count,
        },
        "restore_drills": {
            "passed_count": restore_drill_passed_count,
            "failed_count": restore_drill_failed_count,
            "latest_status": latest_restore_drill_status,
        },
        "feature_gates": feature_gates,
        "outbox": outbox,
        "compression": compression,
        "retention": retention,
        "encryption": encryption,
        "prompt_injection_text_is_data": true,
        "historical_replay_side_effects_suppressed": true,
    });

    Ok(SupportBundleDiagnosticsRecord {
        support_bundle_id: bundle.id.clone(),
        generated_at_ms: now,
        include_raw,
        raw_content_included: include_raw,
        manifest_json: manifest,
    })
}

fn recover_expired_maintenance_leases(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<MaintenanceRecoverySummary, TerminalPersistenceV2Error> {
    connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
        let retryable_outbox = diesel::update(
            terminal_outbox_messages::table
                .filter(terminal_outbox_messages::state.eq("claimed"))
                .filter(terminal_outbox_messages::claimed_until_ms.le(Some(now)))
                .filter(
                    terminal_outbox_messages::attempts.lt(terminal_outbox_messages::max_attempts),
                ),
        )
        .set((
            terminal_outbox_messages::state.eq("pending"),
            terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
            terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
            terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
            terminal_outbox_messages::next_run_at_ms.eq(now),
            terminal_outbox_messages::last_error
                .eq(Some("outbox lease expired during maintenance recovery".to_string())),
            terminal_outbox_messages::updated_at_ms.eq(now),
        ))
        .execute(connection)?;

        let exhausted_outbox = diesel::update(
            terminal_outbox_messages::table
                .filter(terminal_outbox_messages::state.eq("claimed"))
                .filter(terminal_outbox_messages::claimed_until_ms.le(Some(now)))
                .filter(
                    terminal_outbox_messages::attempts.ge(terminal_outbox_messages::max_attempts),
                ),
        )
        .set((
            terminal_outbox_messages::state.eq("quarantined"),
            terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
            terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
            terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
            terminal_outbox_messages::next_run_at_ms.eq(now),
            terminal_outbox_messages::last_error
                .eq(Some("outbox lease expired after max attempts".to_string())),
            terminal_outbox_messages::updated_at_ms.eq(now),
        ))
        .execute(connection)?;

        let stale_writer_ids = terminal_writer_generations::table
            .filter(terminal_writer_generations::state.eq("active"))
            .filter(terminal_writer_generations::lease_expires_at_ms.le(now))
            .select(terminal_writer_generations::id)
            .load::<String>(connection)?;
        let stale_writers = diesel::update(
            terminal_writer_generations::table
                .filter(terminal_writer_generations::state.eq("active"))
                .filter(terminal_writer_generations::lease_expires_at_ms.le(now)),
        )
        .set((
            terminal_writer_generations::state.eq("stale"),
            terminal_writer_generations::released_at_ms.eq(Some(now)),
        ))
        .execute(connection)?;

        for writer_generation in stale_writer_ids.iter().take(stale_writers) {
            insert_clock_anchor(connection, writer_generation, now, "writer_stale_recovery")?;
        }

        Ok(MaintenanceRecoverySummary {
            stale_outbox_claims_requeued: retryable_outbox,
            stale_outbox_claims_quarantined: exhausted_outbox,
            stale_writer_generations_marked: stale_writers,
        })
    })
}

fn map_writer_generation_insert_error(error: DieselError) -> TerminalPersistenceV2Error {
    match error {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            TerminalPersistenceV2Error::WriterAlreadyActive
        }
        other => other.into(),
    }
}

mod domain;
pub use domain::shell_metadata_profile;
use domain::*;

#[cfg(test)]
mod tests;
