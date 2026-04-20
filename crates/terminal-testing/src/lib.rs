//! Shared testing helpers and fixtures for daemon transport smoke coverage.

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{process::Command, sync::Arc};

#[cfg(unix)]
use terminal_application::BackendCatalog;
#[cfg(unix)]
use terminal_backend_api::MuxBackendPort;
#[cfg(unix)]
use terminal_backend_native::NativeBackend;
#[cfg(unix)]
use terminal_backend_tmux::TmuxBackend;
#[cfg(unix)]
use terminal_backend_zellij::ZellijBackend;
use terminal_daemon::{
    LocalSocketServerHandle, TerminalDaemon, TerminalDaemonState, spawn_local_socket_server,
};
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_persistence::SqliteSessionStore;
use terminal_protocol::LocalSocketAddress;

#[must_use]
pub fn daemon_state() -> TerminalDaemonState {
    TerminalDaemonState::default()
}

#[must_use]
pub fn isolated_daemon_state(label: &str) -> TerminalDaemonState {
    let store = SqliteSessionStore::open(unique_sqlite_path(label))
        .expect("isolated sqlite session store should open");
    TerminalDaemonState::with_default_persistence(store)
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

#[must_use]
pub fn unique_sqlite_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir()
        .join(format!("terminal-platform-{label}-{}-{nanos}.sqlite3", std::process::id()))
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

#[cfg(unix)]
#[must_use]
pub fn tmux_daemon_state(socket_name: &str) -> TerminalDaemonState {
    TerminalDaemonState::new(BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(TmuxBackend::with_socket_name(socket_name)) as Arc<dyn MuxBackendPort>,
        Arc::new(ZellijBackend) as Arc<dyn MuxBackendPort>,
    ]))
}

#[cfg(unix)]
#[must_use]
pub fn unique_tmux_socket_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("terminal-platform-{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
#[must_use]
pub fn unique_tmux_session_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
#[derive(Debug)]
pub struct TmuxServerGuard {
    socket_name: String,
}

#[cfg(unix)]
impl TmuxServerGuard {
    pub fn spawn(socket_name: &str, session_name: &str) -> Result<Self, String> {
        run_tmux(
            socket_name,
            &[
                "new-session",
                "-d",
                "-s",
                session_name,
                "sh",
                "-lc",
                "printf 'hello from tmux\\n'; exec cat",
            ],
        )?;
        run_tmux(
            socket_name,
            &[
                "new-window",
                "-d",
                "-t",
                session_name,
                "-n",
                "logs",
                "sh",
                "-lc",
                "printf 'logs ready\\n'; exec cat",
            ],
        )?;

        Ok(Self { socket_name: socket_name.to_string() })
    }
}

#[cfg(unix)]
impl Drop for TmuxServerGuard {
    fn drop(&mut self) {
        let _ = run_tmux(&self.socket_name, &["kill-server"]);
    }
}

#[cfg(unix)]
fn run_tmux(socket_name: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .arg("-L")
        .arg(socket_name)
        .args(args)
        .output()
        .map_err(|error| format!("failed to spawn tmux: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid tmux utf8 output: {error}"))
}
