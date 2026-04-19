//! Shared testing helpers and fixtures for daemon transport smoke coverage.

use std::time::{SystemTime, UNIX_EPOCH};

use terminal_daemon::{
    LocalSocketServerHandle, TerminalDaemon, TerminalDaemonState, spawn_local_socket_server,
};
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_protocol::LocalSocketAddress;

#[must_use]
pub fn daemon_state() -> TerminalDaemonState {
    TerminalDaemonState::default()
}

pub struct DaemonFixture {
    pub client: LocalSocketDaemonClient,
    server: LocalSocketServerHandle,
}

impl DaemonFixture {
    pub async fn shutdown(self) -> std::io::Result<()> {
        self.server.shutdown().await
    }
}

#[must_use]
pub fn unique_socket_address(label: &str) -> LocalSocketAddress {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let slug = format!("terminal-platform-{label}-{}-{nanos}.sock", std::process::id());

    LocalSocketAddress::from_runtime_slug(slug)
}

pub fn daemon_fixture(label: &str) -> std::io::Result<DaemonFixture> {
    daemon_fixture_with_state(label, TerminalDaemonState::default())
}

pub fn daemon_fixture_with_state(
    label: &str,
    state: TerminalDaemonState,
) -> std::io::Result<DaemonFixture> {
    let address = unique_socket_address(label);
    let server = spawn_local_socket_server(TerminalDaemon::new(state), address.clone())?;
    let client = LocalSocketDaemonClient::new(address);

    Ok(DaemonFixture { client, server })
}
