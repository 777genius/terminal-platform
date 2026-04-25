mod adapters;
mod application;
pub mod backend_registry;
pub mod bootstrap;
mod composition;
pub mod service;
pub mod transport;

#[cfg(not(any(feature = "native-backend", feature = "tmux-backend", feature = "zellij-backend")))]
compile_error!("terminal-daemon requires at least one backend feature to be enabled");

pub use backend_registry::{
    TerminalDaemonBackendBuildError, TerminalDaemonBackendConfig, TerminalDaemonBackendProvider,
    TerminalDaemonBackendRegistry,
};
pub use bootstrap::{
    TerminalDaemonBootstrapBuildError, TerminalDaemonBootstrapConfig,
    TerminalDaemonBootstrapConfigError,
};
pub use service::TerminalDaemon;
pub use transport::{LocalSocketServerHandle, spawn_local_socket_server};
