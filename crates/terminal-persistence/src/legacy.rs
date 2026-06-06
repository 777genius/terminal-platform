use std::{
    fmt, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::{SavedSessionManifest, SessionId, SessionRoute};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};
use thiserror::Error;

use crate::db::executor::PersistenceExecutor;
use crate::v2::TerminalPersistenceV2Config;

use self::retry::retry_persistence_operation;

mod retry;
mod routes;
mod saved_sessions;
mod schema;
mod summary;
mod v2_facade;

#[cfg(test)]
mod tests;

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
    v2_config: TerminalPersistenceV2Config,
    v2_executor: Arc<Mutex<Option<Arc<PersistenceExecutor>>>>,
}

impl fmt::Debug for SqliteSessionStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("SqliteSessionStore").field("path", &self.path).finish()
    }
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
        Self::open_with_v2_config(path, TerminalPersistenceV2Config::default())
    }

    pub fn open_with_v2_config(
        path: impl Into<PathBuf>,
        v2_config: TerminalPersistenceV2Config,
    ) -> Result<Self, PersistenceError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let store = Self { path, v2_config, v2_executor: Arc::new(Mutex::new(None)) };
        retry_persistence_operation(|| store.ensure_schema())?;
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
}
