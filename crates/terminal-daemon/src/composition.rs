use std::sync::Arc;

use terminal_backend_api::MuxBackendPort;
use terminal_backend_native::NativeBackend;
use terminal_backend_tmux::TmuxBackend;
use terminal_backend_zellij::ZellijBackend;
use terminal_persistence::SqliteSessionStore;
use terminal_runtime::{BackendCatalog, TerminalRuntime};

pub(crate) fn default_backend_catalog() -> BackendCatalog {
    BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(TmuxBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(ZellijBackend) as Arc<dyn MuxBackendPort>,
    ])
}

pub(crate) fn runtime_with_persistence(persistence: SqliteSessionStore) -> TerminalRuntime {
    TerminalRuntime::with_persistence(default_backend_catalog(), persistence)
}

pub(crate) fn default_runtime() -> TerminalRuntime {
    TerminalRuntime::new(default_backend_catalog())
}
