pub mod backend_registry;
pub mod bootstrap;
#[cfg(not(any(feature = "native-backend", feature = "tmux-backend", feature = "zellij-backend")))]
compile_error!("terminal-daemon requires at least one backend feature to be enabled");

pub mod service;
pub mod state;
pub mod transport;

pub use backend_registry::{TerminalDaemonBackendProvider, TerminalDaemonBackendRegistry};
pub use bootstrap::{TerminalDaemonBootstrapConfig, TerminalDaemonBootstrapConfigError};
pub use service::TerminalDaemon;
pub use state::{
    TerminalDaemonBackendConfig, TerminalDaemonState, TerminalDaemonStateBuildError,
    TerminalDaemonStateBuilder,
};
pub use transport::{LocalSocketServerHandle, spawn_local_socket_server};
