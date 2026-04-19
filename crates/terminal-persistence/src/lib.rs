use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::{M, Migrations};
use serde::{Deserialize, Serialize};
use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::{SessionId, SessionRoute};
use terminal_mux_domain::PaneTreeNode;
use terminal_projection::{ScreenSnapshot, TopologySnapshot};
use thiserror::Error;
use uuid::Uuid;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(
        "
        CREATE TABLE IF NOT EXISTS native_saved_sessions (
            session_id TEXT PRIMARY KEY,
            route_json TEXT NOT NULL,
            title TEXT,
            launch_json TEXT,
            topology_json TEXT NOT NULL,
            screens_json TEXT NOT NULL,
            saved_at_ms INTEGER NOT NULL
        );
        ",
    )])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedNativeSession {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
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
    pub has_launch: bool,
    pub tab_count: usize,
    pub pane_count: usize,
}

#[derive(Debug, Clone)]
pub struct SqliteSessionStore {
    path: PathBuf,
}

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

        let store = Self { path };
        store.ensure_schema()?;
        Ok(store)
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
                topology_json,
                screens_json,
                saved_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(session_id) DO UPDATE SET
                route_json = excluded.route_json,
                title = excluded.title,
                launch_json = excluded.launch_json,
                topology_json = excluded.topology_json,
                screens_json = excluded.screens_json,
                saved_at_ms = excluded.saved_at_ms
            ",
            params![
                session.session_id.0.to_string(),
                serde_json::to_string(&session.route)?,
                session.title,
                serde_json::to_string(&session.launch)?,
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
                SELECT route_json, title, launch_json, topology_json, screens_json, saved_at_ms
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
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .optional()?;

        row.map_or(
            Ok(None),
            |(route_json, title, launch_json, topology_json, screens_json, saved_at_ms)| {
                Ok(Some(SavedNativeSession {
                    session_id,
                    route: serde_json::from_str(&route_json)?,
                    title,
                    launch: serde_json::from_str(&launch_json)?,
                    topology: serde_json::from_str(&topology_json)?,
                    screens: serde_json::from_str(&screens_json)?,
                    saved_at_ms,
                }))
            },
        )
    }

    pub fn list_native_sessions(&self) -> Result<Vec<SavedSessionSummary>, PersistenceError> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "
            SELECT session_id, route_json, title, launch_json, topology_json, saved_at_ms
            FROM native_saved_sessions
            ORDER BY saved_at_ms DESC
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            let (session_id, route_json, title, launch_json, topology_json, saved_at_ms) = row?;
            let route: SessionRoute = serde_json::from_str(&route_json)?;
            let launch: Option<ShellLaunchSpec> = serde_json::from_str(&launch_json)?;
            let topology: TopologySnapshot = serde_json::from_str(&topology_json)?;
            sessions.push(SavedSessionSummary {
                session_id: SessionId::from(Uuid::parse_str(&session_id).map_err(|error| {
                    PersistenceError::InvalidData(format!(
                        "invalid saved session id `{session_id}` - {error}"
                    ))
                })?),
                route,
                title,
                saved_at_ms,
                has_launch: launch.is_some(),
                tab_count: topology.tabs.len(),
                pane_count: topology.tabs.iter().map(|tab| pane_count(&tab.root)).sum(),
            });
        }

        Ok(sessions)
    }

    pub fn save_timestamp_ms() -> Result<i64, PersistenceError> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64)
    }

    fn ensure_schema(&self) -> Result<(), PersistenceError> {
        let mut connection = Connection::open(&self.path)?;
        migrations().to_latest(&mut connection)?;
        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, PersistenceError> {
        self.ensure_schema()?;
        Ok(Connection::open(&self.path)?)
    }
}

fn pane_count(root: &PaneTreeNode) -> usize {
    match root {
        PaneTreeNode::Leaf { .. } => 1,
        PaneTreeNode::Split(split) => pane_count(&split.first) + pane_count(&split.second),
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::ShellLaunchSpec;
    use terminal_domain::{BackendKind, PaneId, RouteAuthority, SessionId, SessionRoute, TabId};
    use terminal_projection::{
        ProjectionSource, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
    };

    use super::{SavedNativeSession, SqliteSessionStore};

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
        assert_eq!(listed[1].session_id, older_session);

        let _ = std::fs::remove_file(path);
    }
}
