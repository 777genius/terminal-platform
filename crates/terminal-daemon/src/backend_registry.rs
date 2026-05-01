mod config;
mod error;
mod ordering;
mod providers;
mod registry;

pub use config::TerminalDaemonBackendConfig;
pub use error::TerminalDaemonBackendBuildError;
pub use providers::TerminalDaemonBackendProvider;
pub use registry::TerminalDaemonBackendRegistry;

#[must_use]
pub fn compiled_backend_kinds() -> Vec<terminal_domain::BackendKind> {
    TerminalDaemonBackendRegistry::compiled_default().compiled_backends()
}
