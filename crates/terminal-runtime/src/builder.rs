use terminal_persistence::SqliteSessionStore;
use thiserror::Error;

use crate::{BackendCatalog, TerminalRuntime};

#[derive(Default)]
pub struct TerminalRuntimeBuilder {
    backends: Option<BackendCatalog>,
    persistence: Option<SqliteSessionStore>,
}

impl TerminalRuntimeBuilder {
    #[must_use]
    pub fn with_backends(mut self, backends: BackendCatalog) -> Self {
        self.backends = Some(backends);
        self
    }

    #[must_use]
    pub fn with_persistence(mut self, persistence: SqliteSessionStore) -> Self {
        self.persistence = Some(persistence);
        self
    }

    pub fn with_default_persistence(
        mut self,
    ) -> Result<Self, terminal_persistence::PersistenceError> {
        self.persistence = Some(SqliteSessionStore::open_default()?);
        Ok(self)
    }

    pub fn build(self) -> Result<TerminalRuntime, TerminalRuntimeBuildError> {
        let backends = self.backends.ok_or(TerminalRuntimeBuildError::MissingBackends)?;
        let persistence = self.persistence.ok_or(TerminalRuntimeBuildError::MissingPersistence)?;
        Ok(TerminalRuntime::with_persistence(backends, persistence))
    }
}

#[derive(Debug, Error)]
pub enum TerminalRuntimeBuildError {
    #[error("terminal runtime builder requires a backend catalog")]
    MissingBackends,
    #[error("terminal runtime builder requires a persistence store")]
    MissingPersistence,
}
