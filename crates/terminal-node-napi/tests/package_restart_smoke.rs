use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};

use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_protocol::LocalSocketAddress;
use terminal_testing::{unique_socket_address, wait_for_daemon_ready};
use tokio::time::{Duration, sleep};

mod support;

#[tokio::test(flavor = "multi_thread")]
async fn recovers_staged_package_client_after_daemon_restart() {
    let addon_path = support::locate_cdylib().expect("node addon should be built");
    let package_dir =
        support::stage_node_package(&addon_path).expect("package should stage successfully");
    support::verify_node_package(&package_dir).expect("package should verify successfully");

    for (script, label) in [
        ("package_restart_smoke.cjs", "node-pkg-restart-cjs"),
        ("package_restart_smoke.mjs", "node-pkg-restart-mjs"),
    ] {
        let address = unique_socket_address(label);
        let initial_client = LocalSocketDaemonClient::new(address.clone());
        let mut server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("initial daemon should bind");
        wait_for_daemon_ready(&initial_client).await;

        let (address_kind, address_value) = match &address {
            LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
            LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
        };
        let initial_ready_file = unique_temp_path("terminal-node-package-restart", "ready");
        let stop_file = unique_temp_path("terminal-node-package-restart", "stop");
        let stale_ready_file = unique_temp_path("terminal-node-package-restart", "stale");
        let restart_file = unique_temp_path("terminal-node-package-restart", "restart");
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/{script}"));

        let child = std::process::Command::new("node")
            .arg(script_path)
            .env("TERMINAL_NODE_PACKAGE", &package_dir)
            .env("TERMINAL_NODE_INITIAL_READY_FILE", &initial_ready_file)
            .env("TERMINAL_NODE_STOP_FILE", &stop_file)
            .env("TERMINAL_NODE_STALE_READY_FILE", &stale_ready_file)
            .env("TERMINAL_NODE_RESTART_FILE", &restart_file)
            .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
            .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("package restart smoke should launch");

        wait_for_file(&initial_ready_file).await;
        server.shutdown().await.expect("initial daemon should stop cleanly");
        std::fs::write(&stop_file, "stopped\n").expect("stop signal file should write");
        wait_for_file(&stale_ready_file).await;

        let restarted_client = LocalSocketDaemonClient::new(address.clone());
        server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("replacement daemon should bind");
        wait_for_daemon_ready(&restarted_client).await;
        std::fs::write(&restart_file, "restart\n").expect("restart signal file should write");

        let output = child.wait_with_output().expect("package restart smoke should collect output");
        server.shutdown().await.expect("replacement daemon should stop cleanly");

        assert!(
            output.status.success(),
            "package restart smoke {script} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\"stale_error_observed\":true"),
            "package restart smoke {script} should confirm stale daemon failure"
        );
        assert!(
            stdout.contains("\"recovered\":true"),
            "package restart smoke {script} should confirm recovery against restarted daemon"
        );

        let _ = std::fs::remove_file(&initial_ready_file);
        let _ = std::fs::remove_file(&stop_file);
        let _ = std::fs::remove_file(&stale_ready_file);
        let _ = std::fs::remove_file(&restart_file);
    }
}

async fn wait_for_file(path: &Path) {
    for _ in 0..600 {
        if path.is_file() {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }

    panic!("restart smoke never observed file: {}", path.display());
}

fn unique_temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}
