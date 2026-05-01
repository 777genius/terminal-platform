use terminal_persistence::SqliteSessionStore;

use super::TerminalRuntime;
use crate::{BackendCatalog, TerminalRuntimeBuilder, sessions::SessionService};

impl TerminalRuntime {
    #[must_use]
    pub fn builder() -> TerminalRuntimeBuilder {
        TerminalRuntimeBuilder::default()
    }

    #[must_use]
    pub fn new(backends: BackendCatalog) -> Self {
        Self::builder()
            .with_backends(backends)
            .with_default_persistence()
            .expect("default sqlite session store should open")
            .build()
            .expect("terminal runtime builder should have backends configured")
    }

    #[must_use]
    pub fn with_persistence(backends: BackendCatalog, persistence: SqliteSessionStore) -> Self {
        Self { sessions: SessionService::with_persistence(backends, persistence) }
    }
}
