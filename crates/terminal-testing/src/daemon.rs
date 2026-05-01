use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use terminal_backend_api::ShellLaunchSpec;
use terminal_daemon::{LocalSocketServerHandle, TerminalDaemon, spawn_local_socket_server};
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_persistence::SqliteSessionStore;
use terminal_protocol::LocalSocketAddress;

pub fn daemon() -> TerminalDaemon {
    TerminalDaemon::default()
}

#[must_use]
pub fn isolated_daemon(label: &str) -> TerminalDaemon {
    let store = SqliteSessionStore::open(unique_sqlite_path(label))
        .expect("isolated sqlite session store should open");
    TerminalDaemon::with_persistence(store)
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
    daemon_fixture_with_daemon(label, TerminalDaemon::default())
}

pub fn daemon_fixture_with_daemon(
    label: &str,
    daemon: TerminalDaemon,
) -> std::io::Result<DaemonFixture> {
    let address = unique_socket_address(label);
    let server = spawn_local_socket_server(daemon, address.clone())?;
    let client = LocalSocketDaemonClient::new(address);

    Ok(DaemonFixture { client, server })
}

pub async fn wait_for_daemon_ready(client: &LocalSocketDaemonClient) {
    for _ in 0..100 {
        if client.handshake().await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    panic!("daemon fixture never became ready for handshake");
}

#[must_use]
pub fn echo_shell_launch_spec() -> ShellLaunchSpec {
    #[cfg(unix)]
    {
        ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
    }

    #[cfg(windows)]
    {
        let program = std::env::var("COMSPEC")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "cmd.exe".to_string());

        // Hosted Windows has been more reliable with a plain `cmd` bootstrap than with a
        // prompt mutation or a separate Node echo loop.
        ShellLaunchSpec::new(program).with_args(["/D", "/Q", "/K", "echo ready"])
    }
}
