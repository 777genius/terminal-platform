//! Shared testing helpers and fixtures will live here.

use terminal_daemon::TerminalDaemonState;
use terminal_daemon_client::InProcessDaemonClient;

#[must_use]
pub fn daemon_state() -> TerminalDaemonState {
    TerminalDaemonState::default()
}

#[must_use]
pub fn daemon_client() -> InProcessDaemonClient {
    InProcessDaemonClient::default()
}
