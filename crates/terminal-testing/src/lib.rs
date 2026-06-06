//! Shared testing helpers and fixtures for daemon transport smoke coverage.

mod daemon;
#[cfg(unix)]
mod tmux;
#[cfg(any(unix, windows))]
mod zellij;

pub use daemon::{
    DaemonFixture, daemon, daemon_fixture, daemon_fixture_with_daemon, echo_shell_launch_spec,
    isolated_daemon, unique_socket_address, unique_sqlite_path, wait_for_daemon_ready,
};

#[cfg(all(unix, feature = "tmux-backend"))]
pub use tmux::tmux_daemon;
#[cfg(unix)]
pub use tmux::{TmuxServerGuard, unique_tmux_session_name, unique_tmux_socket_name};

#[cfg(any(unix, windows))]
pub use zellij::{ZellijSessionGuard, ZellijTestLock, unique_zellij_session_name};
