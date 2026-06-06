use terminal_runtime::TerminalRuntimeBuildError;
use thiserror::Error;

use crate::backend_registry::TerminalDaemonBackendBuildError;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalDaemonBootstrapConfigError {
    #[error("terminal-daemon bootstrap config listed no backends")]
    EmptyBackendList,
    #[error("terminal-daemon bootstrap config backend '{value}' is not recognized")]
    UnknownBackend { value: String },
    #[error("terminal-daemon bootstrap env var {env_var} contains non-utf8 data")]
    InvalidEnvironmentEncoding { env_var: &'static str },
}

#[derive(Debug, Error)]
pub enum TerminalDaemonBootstrapBuildError {
    #[error(transparent)]
    Backend(#[from] TerminalDaemonBackendBuildError),
    #[error(transparent)]
    Persistence(#[from] terminal_persistence::PersistenceError),
    #[error(transparent)]
    Runtime(#[from] TerminalRuntimeBuildError),
}
