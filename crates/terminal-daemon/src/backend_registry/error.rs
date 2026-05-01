use terminal_domain::BackendKind;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalDaemonBackendBuildError {
    #[error("terminal-daemon backend config enables no backends")]
    NoBackendsEnabled,
    #[error(
        "terminal-daemon backend {backend:?} was requested but is not compiled in. Compiled backends - {compiled_backends:?}"
    )]
    BackendNotCompiled { backend: BackendKind, compiled_backends: Vec<BackendKind> },
}
