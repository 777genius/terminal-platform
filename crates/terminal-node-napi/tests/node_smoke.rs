use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_protocol::LocalSocketAddress;
use terminal_testing::{daemon_fixture, unique_socket_address, wait_for_daemon_ready};
use tokio::time::{Duration, sleep};

mod support;

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_node_addon_against_daemon_fixture() {
    let fixture = daemon_fixture("terminal-node-napi-smoke").expect("fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
    let addon_path = support::materialize_node_addon().expect("node addon should be materialized");
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/node_smoke.cjs");
    let (address_kind, address_value) = match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };

    let output = Command::new("node")
        .arg(script_path)
        .env("TERMINAL_NODE_ADDON", &addon_path)
        .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
        .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
        .output()
        .expect("node smoke should launch");

    fixture.shutdown().await.expect("fixture should stop cleanly");

    assert!(
        output.status.success(),
        "node smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("\"session_id\""),
        "node smoke should emit structured confirmation"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn closes_node_addon_subscriptions_when_daemon_stops() {
    let fixture = daemon_fixture("terminal-node-napi-addon-close").expect("fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
    let addon_path = support::materialize_node_addon().expect("node addon should be materialized");
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/node_smoke.cjs");
    let (address_kind, address_value) = match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };
    let ready_file = unique_temp_path("terminal-node-addon-close", "ready");

    let child = Command::new("node")
        .arg(script_path)
        .env("TERMINAL_NODE_ADDON", &addon_path)
        .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
        .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
        .env("TERMINAL_NODE_SMOKE_MODE", "shutdown")
        .env("TERMINAL_NODE_READY_FILE", &ready_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("node shutdown smoke should launch");

    wait_for_file(&ready_file).await;
    fixture.shutdown().await.expect("fixture should stop cleanly");

    let output = child.wait_with_output().expect("node shutdown smoke should collect output");

    assert!(
        output.status.success(),
        "node shutdown smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"subscription_closed\":true"),
        "node shutdown smoke should confirm subscription closure"
    );

    let _ = std::fs::remove_file(&ready_file);
}

#[tokio::test(flavor = "multi_thread")]
async fn repeatedly_reopens_subscriptions_through_node_addon() {
    let fixture = daemon_fixture("terminal-node-napi-repeat").expect("fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
    let addon_path = support::materialize_node_addon().expect("node addon should be materialized");
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/node_smoke.cjs");
    let (address_kind, address_value) = match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };

    let output = Command::new("node")
        .arg(script_path)
        .env("TERMINAL_NODE_ADDON", &addon_path)
        .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
        .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
        .env("TERMINAL_NODE_SMOKE_MODE", "repeat-subscriptions")
        .output()
        .expect("node repeat smoke should launch");

    fixture.shutdown().await.expect("fixture should stop cleanly");

    assert!(
        output.status.success(),
        "node repeat smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"cycles\":24"),
        "node repeat smoke should confirm subscription cycle count"
    );
    assert!(
        stdout.contains("\"observed_markers\":4"),
        "node repeat smoke should confirm live pane updates across cycles"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn recovers_node_addon_client_after_daemon_restart() {
    let addon_path = support::materialize_node_addon().expect("node addon should be materialized");
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/node_smoke.cjs");
    let address = unique_socket_address("terminal-node-addon-restart");
    let initial_client = LocalSocketDaemonClient::new(address.clone());
    let mut server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("initial daemon should bind");
    wait_for_daemon_ready(&initial_client).await;

    let (address_kind, address_value) = match &address {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };
    let initial_ready_file = unique_temp_path("terminal-node-addon-restart", "ready");
    let stop_file = unique_temp_path("terminal-node-addon-restart", "stop");
    let stale_ready_file = unique_temp_path("terminal-node-addon-restart", "stale");
    let restart_file = unique_temp_path("terminal-node-addon-restart", "restart");

    let child = Command::new("node")
        .arg(script_path)
        .env("TERMINAL_NODE_ADDON", &addon_path)
        .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
        .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
        .env("TERMINAL_NODE_SMOKE_MODE", "restart")
        .env("TERMINAL_NODE_INITIAL_READY_FILE", &initial_ready_file)
        .env("TERMINAL_NODE_STOP_FILE", &stop_file)
        .env("TERMINAL_NODE_STALE_READY_FILE", &stale_ready_file)
        .env("TERMINAL_NODE_RESTART_FILE", &restart_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("node restart smoke should launch");

    wait_for_file(&initial_ready_file).await;
    server.shutdown().await.expect("initial daemon should stop cleanly");
    std::fs::write(&stop_file, "stopped\n").expect("stop signal file should write");
    wait_for_file(&stale_ready_file).await;

    let restarted_client = LocalSocketDaemonClient::new(address.clone());
    server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("replacement daemon should bind");
    wait_for_daemon_ready(&restarted_client).await;
    std::fs::write(&restart_file, "restart\n").expect("restart signal file should write");

    let output = child.wait_with_output().expect("node restart smoke should collect output");
    server.shutdown().await.expect("replacement daemon should stop cleanly");

    assert!(
        output.status.success(),
        "node restart smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"stale_error_observed\":true"),
        "node restart smoke should confirm stale daemon failure"
    );
    assert!(
        stdout.contains("\"recovered\":true"),
        "node restart smoke should confirm recovery against restarted daemon"
    );
    assert!(
        stdout.contains("\"recovered_subscription_ok\":true"),
        "node restart smoke should confirm recovered subscription health"
    );

    let _ = std::fs::remove_file(&initial_ready_file);
    let _ = std::fs::remove_file(&stop_file);
    let _ = std::fs::remove_file(&stale_ready_file);
    let _ = std::fs::remove_file(&restart_file);
}

async fn wait_for_file(path: &Path) {
    for _ in 0..120 {
        if path.is_file() {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }

    panic!("node addon smoke never observed file: {}", path.display());
}

fn unique_temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}
