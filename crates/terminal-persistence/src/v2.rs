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

mod session_catalog;

mod feature_gates;

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
