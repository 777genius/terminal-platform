use std::{
    fmt, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::{M, Migrations};
use serde::{Deserialize, Serialize};
use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::{SavedSessionManifest, SessionId, SessionRoute};
use terminal_mux_domain::PaneTreeNode;
use terminal_projection::{ScreenSnapshot, TopologySnapshot};
use thiserror::Error;
use uuid::Uuid;

use crate::db::executor::PersistenceExecutor;
use crate::v2::{
    BackendCapabilityReportInput, CommandHistoryEntryRecord, HistoryGapEventInput,
    PaneHistoryHydrationRecord, RestorePlan, ScreenSnapshotEventInput, TerminalOutputEventInput,
    TerminalPersistenceV2, TerminalPersistenceV2Config, TerminalPersistenceV2Error,
    TopologySnapshotEventInput, UiInputEventInput,
};

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
            "
            CREATE TABLE IF NOT EXISTS native_saved_sessions (
                session_id TEXT PRIMARY KEY,
                route_json TEXT NOT NULL,
                title TEXT,
                launch_json TEXT,
                manifest_json TEXT NOT NULL DEFAULT '{\"format_version\":1,\"binary_version\":\"0.1.0-dev\",\"protocol_major\":0,\"protocol_minor\":1}',
                topology_json TEXT NOT NULL,
                screens_json TEXT NOT NULL,
                saved_at_ms INTEGER NOT NULL
            );
            ",
        ),
        // Keep migration cardinality stable for existing local stores that already advanced
        // to migration index 2 in earlier development builds.
        M::up("SELECT 1;"),
        M::up(
            "
            CREATE TABLE IF NOT EXISTS session_routes (
                session_id TEXT PRIMARY KEY,
                route_json TEXT NOT NULL,
                route_fingerprint TEXT NOT NULL UNIQUE
            );
            ",
        ),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedNativeSession {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub manifest: SavedSessionManifest,
    pub topology: TopologySnapshot,
    pub screens: Vec<ScreenSnapshot>,
    pub saved_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedSessionSummary {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub saved_at_ms: i64,
    pub manifest: SavedSessionManifest,
    pub has_launch: bool,
    pub tab_count: usize,
    pub pane_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrunedSavedSessions {
    pub deleted_count: usize,
    pub kept_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRouteRecord {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub route_fingerprint: String,
}

#[derive(Clone)]
pub struct SqliteSessionStore {
    path: PathBuf,
    v2_executor: Arc<Mutex<Option<Arc<PersistenceExecutor>>>>,
}

impl fmt::Debug for SqliteSessionStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("SqliteSessionStore").field("path", &self.path).finish()
    }
}

type SavedSessionSummaryRow = (String, String, Option<String>, String, String, String, i64);

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("persistence home path unavailable")]
    NoProjectDir,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid persisted data: {0}")]
    InvalidData(String),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("time: {0}")]
    Time(#[from] std::time::SystemTimeError),
}

impl SqliteSessionStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, PersistenceError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let store = Self { path, v2_executor: Arc::new(Mutex::new(None)) };
        retry_persistence_operation(|| store.ensure_schema())?;
        Ok(store)
    }

    pub fn save_native_session_v2_snapshot(
        &self,
        session: &SavedNativeSession,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let session = session.clone();
        retry_v2_write(|| {
            let session = session.clone();
            self.with_v2_store_serialized(move |store| {
                store.import_saved_native_session_snapshot(&session)?;
                store.run_restore_drill(&session.session_id.0.to_string())?;
                store.restore_plan(&session.session_id.0.to_string())
            })
        })
    }

    pub fn native_session_v2_restore_plan(
        &self,
        session_id: SessionId,
    ) -> Result<Option<RestorePlan>, TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            self.with_v2_store_serialized(move |store| {
                match store.restore_plan(&session_id.0.to_string()) {
                    Ok(plan) => Ok(Some(plan)),
                    Err(TerminalPersistenceV2Error::Query(diesel::result::Error::NotFound)) => {
                        Ok(None)
                    }
                    Err(error) => Err(error),
                }
            })
        })
    }

    pub fn record_v2_backend_capability_report(
        &self,
        input: BackendCapabilityReportInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_backend_capability_report(input)
            })
        })
    }

    pub fn record_v2_ui_input(
        &self,
        input: UiInputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| store.record_ui_input_event(input))
        })
    }

    pub fn record_v2_terminal_output(
        &self,
        input: TerminalOutputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_terminal_output_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_history_gap(
        &self,
        input: HistoryGapEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_history_gap_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_screen_snapshot(
        &self,
        input: ScreenSnapshotEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_screen_snapshot_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_topology_snapshot(
        &self,
        input: TopologySnapshotEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_topology_snapshot_event(input)?;
                Ok(())
            })
        })
    }

    pub fn hydrate_v2_pane_history(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, TerminalPersistenceV2Error> {
        let session_id = session_id.to_string();
        let pane_id = pane_id.to_string();
        retry_v2_write(|| {
            let session_id = session_id.clone();
            let pane_id = pane_id.clone();
            self.with_v2_store_serialized(move |store| {
                store.hydrate_pane_history(
                    &session_id,
                    &pane_id,
                    from_event_seq,
                    max_segments,
                    max_bytes,
                )
            })
        })
    }

    pub fn list_v2_command_history(
        &self,
        session_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CommandHistoryEntryRecord>, TerminalPersistenceV2Error> {
        let session_id = session_id.map(ToOwned::to_owned);
        retry_v2_write(|| {
            let session_id = session_id.clone();
            self.with_v2_store_serialized(move |store| {
                store.list_command_history(session_id.as_deref(), limit)
            })
        })
    }

    fn with_v2_store_serialized<T>(
        &self,
        operation: impl FnOnce(TerminalPersistenceV2) -> Result<T, TerminalPersistenceV2Error>
        + Send
        + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let path = self.path.clone();
        self.execute_v2_serialized(move || {
            let store = TerminalPersistenceV2::open_with_config(
                &path,
                TerminalPersistenceV2Config::default(),
            )?;
            operation(store)
        })
    }

    fn execute_v2_serialized<T>(
        &self,
        operation: impl FnOnce() -> Result<T, TerminalPersistenceV2Error> + Send + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let executor = self.v2_executor()?;
        executor.execute(move |_connection| operation())
    }

    fn v2_executor(&self) -> Result<Arc<PersistenceExecutor>, TerminalPersistenceV2Error> {
        let mut guard = self.v2_executor.lock().map_err(|_| {
            TerminalPersistenceV2Error::InvalidData(
                "terminal persistence v2 executor lock poisoned".to_string(),
            )
        })?;
        if let Some(executor) = guard.as_ref() {
            return Ok(Arc::clone(executor));
        }

        let executor = Arc::new(PersistenceExecutor::start(
            &self.path,
            TerminalPersistenceV2Config::default(),
        )?);
        *guard = Some(Arc::clone(&executor));
        Ok(executor)
    }

    pub fn open_default() -> Result<Self, PersistenceError> {
        let project_dirs = ProjectDirs::from("dev", "terminal-platform", "terminal-platform")
            .ok_or(PersistenceError::NoProjectDir)?;
        let path = project_dirs.data_local_dir().join("session-store.sqlite3");
        Self::open(path)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_native_session(
        &self,
        session: &SavedNativeSession,
    ) -> Result<(), PersistenceError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "
            INSERT INTO native_saved_sessions (
                session_id,
                route_json,
                title,
                launch_json,
                manifest_json,
                topology_json,
                screens_json,
                saved_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(session_id) DO UPDATE SET
                route_json = excluded.route_json,
                title = excluded.title,
                launch_json = excluded.launch_json,
                manifest_json = excluded.manifest_json,
                topology_json = excluded.topology_json,
                screens_json = excluded.screens_json,
                saved_at_ms = excluded.saved_at_ms
            ",
            params![
                session.session_id.0.to_string(),
                serde_json::to_string(&session.route)?,
                session.title,
                serde_json::to_string(&session.launch)?,
                serde_json::to_string(&session.manifest)?,
                serde_json::to_string(&session.topology)?,
                serde_json::to_string(&session.screens)?,
                session.saved_at_ms,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_native_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<SavedNativeSession>, PersistenceError> {
        let connection = self.open_connection()?;
        let row = connection
            .query_row(
                "
                SELECT route_json, title, launch_json, manifest_json, topology_json, screens_json, saved_at_ms
                FROM native_saved_sessions
                WHERE session_id = ?1
                ",
                params![session_id.0.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .optional()?;

        row.map_or(
            Ok(None),
            |(
                route_json,
                title,
                launch_json,
                manifest_json,
                topology_json,
                screens_json,
                saved_at_ms,
            )| {
                Ok(Some(SavedNativeSession {
                    session_id,
                    route: serde_json::from_str(&route_json)?,
                    title,
                    launch: serde_json::from_str(&launch_json)?,
                    manifest: serde_json::from_str(&manifest_json)?,
                    topology: serde_json::from_str(&topology_json)?,
                    screens: serde_json::from_str(&screens_json)?,
                    saved_at_ms,
                }))
            },
        )
    }

    pub fn delete_native_session(&self, session_id: SessionId) -> Result<bool, PersistenceError> {
        let connection = self.open_connection()?;
        let deleted = connection.execute(
            "
            DELETE FROM native_saved_sessions
            WHERE session_id = ?1
            ",
            params![session_id.0.to_string()],
        )?;

        Ok(deleted > 0)
    }

    pub fn upsert_session_route(
        &self,
        record: &SessionRouteRecord,
    ) -> Result<(), PersistenceError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "
            INSERT INTO session_routes (
                session_id,
                route_json,
                route_fingerprint
            ) VALUES (?1, ?2, ?3)
            ON CONFLICT(session_id) DO UPDATE SET
                route_json = excluded.route_json,
                route_fingerprint = excluded.route_fingerprint
            ",
            params![
                record.session_id.0.to_string(),
                serde_json::to_string(&record.route)?,
                record.route_fingerprint,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_session_route(
        &self,
        session_id: SessionId,
    ) -> Result<Option<SessionRouteRecord>, PersistenceError> {
        let connection = self.open_connection()?;
        let row = connection
            .query_row(
                "
                SELECT route_json, route_fingerprint
                FROM session_routes
                WHERE session_id = ?1
                ",
                params![session_id.0.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        row.map_or(Ok(None), |(route_json, route_fingerprint)| {
            Ok(Some(SessionRouteRecord {
                session_id,
                route: serde_json::from_str(&route_json)?,
                route_fingerprint,
            }))
        })
    }

    pub fn load_session_route_by_fingerprint(
        &self,
        route_fingerprint: &str,
    ) -> Result<Option<SessionRouteRecord>, PersistenceError> {
        let connection = self.open_connection()?;
        let row = connection
            .query_row(
                "
                SELECT session_id, route_json
                FROM session_routes
                WHERE route_fingerprint = ?1
                ",
                params![route_fingerprint],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        row.map_or(Ok(None), |(session_id, route_json)| {
            Ok(Some(SessionRouteRecord {
                session_id: SessionId::from(Uuid::parse_str(&session_id).map_err(|error| {
                    PersistenceError::InvalidData(format!(
                        "invalid session route id `{session_id}` - {error}"
                    ))
                })?),
                route: serde_json::from_str(&route_json)?,
                route_fingerprint: route_fingerprint.to_string(),
            }))
        })
    }

    pub fn prune_native_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, PersistenceError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        let deleted_count = transaction.execute(
            "
            DELETE FROM native_saved_sessions
            WHERE session_id IN (
                SELECT session_id
                FROM native_saved_sessions
                ORDER BY saved_at_ms DESC, session_id DESC
                LIMIT -1 OFFSET ?1
            )
            ",
            params![keep_latest as i64],
        )?;
        let kept_count =
            transaction.query_row("SELECT COUNT(*) FROM native_saved_sessions", [], |row| {
                row.get::<_, i64>(0)
            })? as usize;
        transaction.commit()?;

        Ok(PrunedSavedSessions { deleted_count, kept_count })
    }

    pub fn list_native_sessions(&self) -> Result<Vec<SavedSessionSummary>, PersistenceError> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "
            SELECT session_id, route_json, title, launch_json, manifest_json, topology_json, saved_at_ms
            FROM native_saved_sessions
            ORDER BY saved_at_ms DESC
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok::<SavedSessionSummaryRow, rusqlite::Error>((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            let row = row?;
            if let Ok(session) = decode_saved_session_summary_row(row) {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }

    pub fn save_timestamp_ms() -> Result<i64, PersistenceError> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64)
    }

    fn ensure_schema(&self) -> Result<(), PersistenceError> {
        let _guard = legacy_schema_lock().lock().map_err(|_| {
            PersistenceError::InvalidData("legacy schema lock poisoned".to_string())
        })?;
        let mut connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_millis(5_000))?;
        migrations().to_latest(&mut connection)?;
        ensure_manifest_column(&connection)?;
        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, PersistenceError> {
        retry_persistence_operation(|| self.ensure_schema())?;
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_millis(5_000))?;
        Ok(connection)
    }
}

fn legacy_schema_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

fn ensure_manifest_column(connection: &Connection) -> Result<(), PersistenceError> {
    let mut statement = connection.prepare("PRAGMA table_info(native_saved_sessions)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == "manifest_json" {
            return Ok(());
        }
    }

    let alter = connection.execute(
        "
        ALTER TABLE native_saved_sessions
        ADD COLUMN manifest_json TEXT NOT NULL DEFAULT '{\"format_version\":1,\"binary_version\":\"0.1.0-dev\",\"protocol_major\":0,\"protocol_minor\":1}';
        ",
        [],
    );
    match alter {
        Ok(_) => Ok(()),
        Err(error) if duplicate_column_error(&error) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn duplicate_column_error(error: &rusqlite::Error) -> bool {
    matches!(error, rusqlite::Error::SqliteFailure(_, Some(message)) if message.contains("duplicate column name"))
}

fn decode_saved_session_summary_row(
    (
        session_id,
        route_json,
        title,
        launch_json,
        manifest_json,
        topology_json,
        saved_at_ms,
    ): SavedSessionSummaryRow,
) -> Result<SavedSessionSummary, PersistenceError> {
    let route: SessionRoute = serde_json::from_str(&route_json)?;
    let launch: Option<ShellLaunchSpec> = serde_json::from_str(&launch_json)?;
    let manifest: SavedSessionManifest = serde_json::from_str(&manifest_json)?;
    let topology: TopologySnapshot = serde_json::from_str(&topology_json)?;
    Ok(SavedSessionSummary {
        session_id: SessionId::from(Uuid::parse_str(&session_id).map_err(|error| {
            PersistenceError::InvalidData(format!(
                "invalid saved session id `{session_id}` - {error}"
            ))
        })?),
        route,
        title,
        saved_at_ms,
        manifest,
        has_launch: launch.is_some(),
        tab_count: topology.tabs.len(),
        pane_count: topology.tabs.iter().map(|tab| pane_count(&tab.root)).sum(),
    })
}

fn pane_count(root: &PaneTreeNode) -> usize {
    match root {
        PaneTreeNode::Leaf { .. } => 1,
        PaneTreeNode::Split(split) => pane_count(&split.first) + pane_count(&split.second),
    }
}

fn retry_persistence_operation<T>(
    mut operation: impl FnMut() -> Result<T, PersistenceError>,
) -> Result<T, PersistenceError> {
    let mut last_error = None;
    for attempt in 0..80 {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if is_retryable_persistence_error(&error) => {
                last_error = Some(error);
                let backoff_ms = 10 + i64::from(attempt.min(20)) * 5;
                thread::sleep(Duration::from_millis(backoff_ms as u64));
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        PersistenceError::InvalidData("sqlite operation retry exhausted".to_string())
    }))
}

fn is_retryable_persistence_error(error: &PersistenceError) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("database is locked")
        || text.contains("database table is locked")
        || text.contains("database busy")
        || text.contains("locking protocol")
}

fn retry_v2_write<T>(
    mut operation: impl FnMut() -> Result<T, TerminalPersistenceV2Error>,
) -> Result<T, TerminalPersistenceV2Error> {
    let mut last_error = None;
    for attempt in 0..80 {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if is_retryable_v2_write_error(&error) => {
                last_error = Some(error);
                let backoff_ms = 10 + i64::from(attempt.min(20)) * 5;
                thread::sleep(Duration::from_millis(backoff_ms as u64));
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        TerminalPersistenceV2Error::InvalidData("v2 write retry exhausted".to_string())
    }))
}

fn is_retryable_v2_write_error(error: &TerminalPersistenceV2Error) -> bool {
    if matches!(
        error,
        TerminalPersistenceV2Error::WriterAlreadyActive | TerminalPersistenceV2Error::Connection(_)
    ) {
        return true;
    }

    let text = error.to_string().to_ascii_lowercase();
    text.contains("database is locked")
        || text.contains("database table is locked")
        || text.contains("database busy")
        || text.contains("locking protocol")
        || text.contains("active terminal writer generation")
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use rusqlite::{Connection, params};
    use terminal_backend_api::ShellLaunchSpec;
    use terminal_domain::{
        BackendKind, CURRENT_BINARY_VERSION, PaneId, RouteAuthority, SavedSessionManifest,
        SessionId, SessionRoute, TabId,
    };
    use terminal_projection::{
        ProjectionSource, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
    };

    use crate::v2::{RestoreGuaranteeLevel, TerminalOutputEventInput};

    use super::{PersistenceError, SavedNativeSession, SessionRouteRecord, SqliteSessionStore};

    fn sample_snapshot(session_id: SessionId, title: &str, line: &str) -> SavedNativeSession {
        SavedNativeSession {
            session_id,
            route: SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            },
            title: Some(title.to_string()),
            launch: Some(ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "exec cat"])),
            manifest: SavedSessionManifest::current(),
            topology: TopologySnapshot {
                session_id,
                backend_kind: BackendKind::Native,
                focused_tab: Some(TabId::new()),
                tabs: Vec::new(),
            },
            screens: vec![ScreenSnapshot {
                pane_id: PaneId::new(),
                sequence: 1,
                rows: 24,
                cols: 80,
                source: ProjectionSource::NativeEmulator,
                surface: ScreenSurface {
                    title: Some(title.to_string()),
                    cursor: None,
                    lines: vec![ScreenLine { text: line.to_string() }],
                },
            }],
            saved_at_ms: SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve"),
        }
    }

    #[test]
    fn saves_and_loads_native_session_snapshot() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path = std::env::temp_dir().join(format!("terminal-platform-test-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let session_id = SessionId::new();
        let snapshot = sample_snapshot(session_id, "shell", "ready");

        store.save_native_session(&snapshot).expect("save should succeed");
        let loaded = store
            .load_native_session(session_id)
            .expect("load should succeed")
            .expect("saved session should exist");

        assert_eq!(loaded, snapshot);
        assert_eq!(loaded.manifest.format_version, 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn upserts_existing_native_session_snapshot() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-upsert-test-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let session_id = SessionId::new();
        let first = sample_snapshot(session_id, "shell", "ready");
        let second = sample_snapshot(session_id, "shell-renamed", "ready again");

        store.save_native_session(&first).expect("first save should succeed");
        store.save_native_session(&second).expect("second save should succeed");

        let loaded = store
            .load_native_session(session_id)
            .expect("load should succeed")
            .expect("saved session should exist");

        assert_eq!(loaded.title.as_deref(), Some("shell-renamed"));
        assert_eq!(
            loaded.screens[0].surface.lines.first().map(|line| line.text.as_str()),
            Some("ready again")
        );
        assert!(loaded.saved_at_ms >= first.saved_at_ms);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn deletes_saved_native_session_snapshot() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-delete-test-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let session_id = SessionId::new();
        let snapshot = sample_snapshot(session_id, "shell", "ready");

        store.save_native_session(&snapshot).expect("save should succeed");

        assert!(store.delete_native_session(session_id).expect("delete should succeed"));
        assert!(store.load_native_session(session_id).expect("load should succeed").is_none());
        assert!(!store.delete_native_session(session_id).expect("delete should succeed"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn prunes_saved_native_sessions_to_latest_count() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-prune-test-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let oldest_session = SessionId::new();
        let middle_session = SessionId::new();
        let newest_session = SessionId::new();
        let oldest = sample_snapshot(oldest_session, "older", "first");
        let mut middle = sample_snapshot(middle_session, "middle", "second");
        middle.saved_at_ms = oldest.saved_at_ms + 1;
        let mut newest = sample_snapshot(newest_session, "newest", "third");
        newest.saved_at_ms = oldest.saved_at_ms + 2;

        store.save_native_session(&oldest).expect("oldest save should succeed");
        store.save_native_session(&middle).expect("middle save should succeed");
        store.save_native_session(&newest).expect("newest save should succeed");

        let pruned = store.prune_native_sessions(1).expect("prune should succeed");
        let listed = store.list_native_sessions().expect("list should succeed");

        assert_eq!(pruned.deleted_count, 2);
        assert_eq!(pruned.kept_count, 1);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].session_id, newest_session);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn lists_saved_native_sessions_in_descending_timestamp_order() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-list-test-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let older_session = SessionId::new();
        let newer_session = SessionId::new();
        let older = sample_snapshot(older_session, "older", "first");
        let mut newer = sample_snapshot(newer_session, "newer", "second");
        newer.saved_at_ms = older.saved_at_ms + 1;

        store.save_native_session(&older).expect("older save should succeed");
        store.save_native_session(&newer).expect("newer save should succeed");

        let listed = store.list_native_sessions().expect("list should succeed");

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].session_id, newer_session);
        assert_eq!(listed[0].title.as_deref(), Some("newer"));
        assert_eq!(listed[0].tab_count, 0);
        assert_eq!(listed[0].pane_count, 0);
        assert!(listed[0].has_launch);
        assert_eq!(listed[0].manifest.format_version, 1);
        assert_eq!(listed[1].session_id, older_session);
        assert_eq!(listed[1].manifest.format_version, 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn lists_saved_native_sessions_ignores_corrupted_rows() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path = std::env::temp_dir()
            .join(format!("terminal-platform-corrupt-list-test-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let valid_session_id = SessionId::new();
        let corrupt_session_id = SessionId::new();
        let snapshot = sample_snapshot(valid_session_id, "shell", "ready");

        store.save_native_session(&snapshot).expect("valid save should succeed");

        let connection = Connection::open(&path).expect("raw sqlite should open");
        connection
            .execute(
                "
                INSERT INTO native_saved_sessions (
                    session_id,
                    route_json,
                    title,
                    launch_json,
                    manifest_json,
                    topology_json,
                    screens_json,
                    saved_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
                params![
                    corrupt_session_id.0.to_string(),
                    serde_json::to_string(&snapshot.route).expect("route json should serialize"),
                    "corrupt",
                    serde_json::to_string(&snapshot.launch).expect("launch json should serialize"),
                    serde_json::to_string(&snapshot.manifest)
                        .expect("manifest json should serialize"),
                    "{not-json",
                    serde_json::to_string(&snapshot.screens)
                        .expect("screens json should serialize"),
                    snapshot.saved_at_ms + 1,
                ],
            )
            .expect("corrupted row should insert");

        let listed = store.list_native_sessions().expect("list should succeed");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].session_id, valid_session_id);
        assert_eq!(listed[0].title.as_deref(), Some("shell"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_native_session_reports_corrupted_row_for_targeted_lookup() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path = std::env::temp_dir()
            .join(format!("terminal-platform-corrupt-load-test-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let session_id = SessionId::new();
        let snapshot = sample_snapshot(session_id, "shell", "ready");

        let connection = Connection::open(&path).expect("raw sqlite should open");
        connection
            .execute(
                "
                INSERT INTO native_saved_sessions (
                    session_id,
                    route_json,
                    title,
                    launch_json,
                    manifest_json,
                    topology_json,
                    screens_json,
                    saved_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
                params![
                    session_id.0.to_string(),
                    serde_json::to_string(&snapshot.route).expect("route json should serialize"),
                    "corrupt",
                    serde_json::to_string(&snapshot.launch).expect("launch json should serialize"),
                    serde_json::to_string(&snapshot.manifest)
                        .expect("manifest json should serialize"),
                    "{not-json",
                    serde_json::to_string(&snapshot.screens)
                        .expect("screens json should serialize"),
                    snapshot.saved_at_ms,
                ],
            )
            .expect("corrupted row should insert");

        let error = store
            .load_native_session(session_id)
            .expect_err("targeted lookup should fail for corrupted row");

        assert!(matches!(error, PersistenceError::Serde(_) | PersistenceError::InvalidData(_)));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn upgrades_legacy_saved_session_schema_with_manifest_column() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-legacy-schema-{nonce}.sqlite3"));
        let connection = Connection::open(&path).expect("legacy db should open");
        connection
            .execute_batch(
                "
                CREATE TABLE native_saved_sessions (
                    session_id TEXT PRIMARY KEY,
                    route_json TEXT NOT NULL,
                    title TEXT,
                    launch_json TEXT,
                    topology_json TEXT NOT NULL,
                    screens_json TEXT NOT NULL,
                    saved_at_ms INTEGER NOT NULL
                );
                ",
            )
            .expect("legacy schema should be created");
        drop(connection);

        let store = SqliteSessionStore::open(&path).expect("store should upgrade legacy schema");
        let session_id = SessionId::new();
        let snapshot = sample_snapshot(session_id, "shell", "ready");

        store.save_native_session(&snapshot).expect("save should succeed after upgrade");
        let loaded = store
            .load_native_session(session_id)
            .expect("load should succeed")
            .expect("saved session should exist");

        assert_eq!(loaded.manifest.format_version, 1);
        assert_eq!(loaded.manifest.binary_version, CURRENT_BINARY_VERSION);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn saves_and_loads_session_route_registry_records() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-route-registry-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let record = SessionRouteRecord {
            session_id: SessionId::new(),
            route: SessionRoute {
                backend: BackendKind::Tmux,
                authority: RouteAuthority::ImportedForeign,
                external: Some(terminal_domain::ExternalSessionRef {
                    namespace: "tmux_session".to_string(),
                    value: "demo".to_string(),
                }),
            },
            route_fingerprint: "tmux/import/demo".to_string(),
        };

        store.upsert_session_route(&record).expect("route record should save");

        assert_eq!(
            store.load_session_route(record.session_id).expect("lookup by id should succeed"),
            Some(record.clone())
        );
        assert_eq!(
            store
                .load_session_route_by_fingerprint(&record.route_fingerprint)
                .expect("lookup by fingerprint should succeed"),
            Some(record)
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn saves_native_session_snapshot_into_v2_visual_history() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-v2-visual-save-{nonce}.sqlite3"));
        let store = SqliteSessionStore::open(&path).expect("store should open");
        let session_id = SessionId::new();
        let snapshot = sample_snapshot(session_id, "shell", "visible history");

        let plan = store
            .save_native_session_v2_snapshot(&snapshot)
            .expect("v2 visual snapshot should save");

        assert_eq!(plan.session_id, session_id.0.to_string());
        assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::VisualSnapshotOnly);
        assert!(plan.latest_screen_snapshot_id.is_some());
        assert!(plan.latest_topology_snapshot_id.is_some());
        assert_eq!(plan.latest_restore_drill_status.as_deref(), Some("passed"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn v2_facade_serializes_concurrent_output_capture() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-v2-serialized-{nonce}.sqlite3"));
        let store = Arc::new(SqliteSessionStore::open(&path).expect("store should open"));
        let session_id = SessionId::new();
        let pane_id = PaneId::new();
        let route = SessionRoute {
            backend: BackendKind::Native,
            authority: RouteAuthority::LocalDaemon,
            external: None,
        };

        let mut handles = Vec::new();
        for index in 0..12 {
            let store = Arc::clone(&store);
            let route = route.clone();
            let input = TerminalOutputEventInput {
                session_id: session_id.0.to_string(),
                route,
                title: Some("serialized shell".to_string()),
                launch: None,
                pane_id: pane_id.0.to_string(),
                tab_id: None,
                payload: format!("serialized-line-{index}\n").into_bytes(),
                rows: Some(24),
                cols: Some(80),
                source_sequence: Some(index),
                occurred_at_ms: None,
                capture_semantics: Some("raw_vt_stream".to_string()),
            };
            handles.push(thread::spawn(move || {
                store
                    .record_v2_terminal_output(input)
                    .expect("serialized v2 output capture should persist");
            }));
        }

        for handle in handles {
            handle.join().expect("capture thread should finish");
        }

        let history = store
            .hydrate_v2_pane_history(
                &session_id.0.to_string(),
                &pane_id.0.to_string(),
                Some(1),
                Some(32),
                Some(16 * 1024),
            )
            .expect("history should hydrate after concurrent capture");

        assert_eq!(history.segments.len(), 12);
        let mut expected_event_seq = 1;
        let mut payload_text = String::new();
        for segment in &history.segments {
            assert_eq!(segment.event_seq_low, expected_event_seq);
            assert_eq!(segment.event_seq_high, expected_event_seq);
            expected_event_seq += 1;
            payload_text.push_str(&String::from_utf8_lossy(&segment.payload));
        }
        for index in 0..12 {
            assert!(payload_text.contains(&format!("serialized-line-{index}")));
        }

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn upgrades_legacy_saved_session_schema_without_session_routes_table() {
        let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
        let path =
            std::env::temp_dir().join(format!("terminal-platform-legacy-routes-{nonce}.sqlite3"));
        let connection = Connection::open(&path).expect("legacy db should open");
        connection
            .execute_batch(
                "
                CREATE TABLE native_saved_sessions (
                    session_id TEXT PRIMARY KEY,
                    route_json TEXT NOT NULL,
                    title TEXT,
                    launch_json TEXT,
                    topology_json TEXT NOT NULL,
                    screens_json TEXT NOT NULL,
                    saved_at_ms INTEGER NOT NULL
                );
                ",
            )
            .expect("legacy schema should be created");
        drop(connection);

        let store = SqliteSessionStore::open(&path).expect("store should upgrade legacy schema");
        let record = SessionRouteRecord {
            session_id: SessionId::new(),
            route: SessionRoute {
                backend: BackendKind::Tmux,
                authority: RouteAuthority::ImportedForeign,
                external: Some(terminal_domain::ExternalSessionRef {
                    namespace: "tmux_session".to_string(),
                    value: "after-upgrade".to_string(),
                }),
            },
            route_fingerprint: "tmux/import/after-upgrade".to_string(),
        };

        store.upsert_session_route(&record).expect("route record should save after upgrade");
        assert_eq!(
            store
                .load_session_route_by_fingerprint(&record.route_fingerprint)
                .expect("lookup by fingerprint should succeed"),
            Some(record)
        );

        let _ = std::fs::remove_file(path);
    }
}
