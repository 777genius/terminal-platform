use std::{path::PathBuf, process::Command};

use terminal_protocol::LocalSocketAddress;
use terminal_testing::{daemon_fixture, wait_for_daemon_ready};

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
