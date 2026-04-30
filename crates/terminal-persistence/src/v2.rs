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

mod core;
pub use core::*;

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

mod restore;

mod capture;

mod models;
pub use models::*;

mod rows;
pub use rows::TerminalDbIdentityRow;
use rows::*;

mod integrity;
use integrity::*;

mod repository;
use repository::*;

mod diagnostics;
use diagnostics::*;

mod maintenance;
use maintenance::*;

mod domain;
pub use domain::shell_metadata_profile;
use domain::*;

#[cfg(test)]
mod tests;
