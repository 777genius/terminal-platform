//! Shared testing helpers and fixtures will live here.

use terminal_daemon::TerminalDaemonState;

#[must_use]
pub fn daemon_state() -> TerminalDaemonState {
    TerminalDaemonState::default()
}
