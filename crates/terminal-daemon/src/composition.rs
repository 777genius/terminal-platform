use terminal_persistence::SqliteSessionStore;
use terminal_runtime::{BackendCatalog, TerminalRuntime};

use crate::{
    TerminalDaemonBackendConfig, TerminalDaemonBackendRegistry,
    backend_registry::TerminalDaemonBackendBuildError,
};

pub(crate) fn default_backend_catalog() -> BackendCatalog {
    backend_catalog(TerminalDaemonBackendConfig::default())
        .expect("compiled default terminal-daemon backend catalog should build")
}

pub(crate) fn backend_catalog(
    backend_config: TerminalDaemonBackendConfig,
) -> Result<BackendCatalog, TerminalDaemonBackendBuildError> {
    TerminalDaemonBackendRegistry::compiled_default().build_catalog(backend_config)
}

pub(crate) fn runtime_with_persistence(persistence: SqliteSessionStore) -> TerminalRuntime {
    TerminalRuntime::with_persistence(default_backend_catalog(), persistence)
}

pub(crate) fn default_runtime() -> TerminalRuntime {
    TerminalRuntime::new(default_backend_catalog())
}
