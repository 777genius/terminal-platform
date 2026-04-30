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
}

mod history_read;

mod operations;

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
