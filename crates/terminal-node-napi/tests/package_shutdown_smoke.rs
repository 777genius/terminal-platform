use std::{
    path::PathBuf,
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};

use terminal_protocol::LocalSocketAddress;
use terminal_testing::daemon_fixture;
use tokio::time::{Duration, sleep};

mod support;

#[tokio::test(flavor = "multi_thread")]
async fn closes_staged_package_subscriptions_when_daemon_stops() {
    let addon_path = support::locate_cdylib().expect("node addon should be built");
    let package_dir =
        support::stage_node_package(&addon_path).expect("package should stage successfully");
    support::verify_node_package(&package_dir).expect("package should verify successfully");

    for (script, fixture_label) in [
        ("package_shutdown_smoke.cjs", "node-pkg-close-cjs"),
        ("package_shutdown_smoke.mjs", "node-pkg-close-mjs"),
    ] {
        let fixture = daemon_fixture(fixture_label).expect("fixture should start");
        let (address_kind, address_value) = match fixture.client.address() {
            LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
            LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
        };
        let ready_file = unique_temp_path("terminal-node-package-close", "ready");
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/{script}"));

        let child = std::process::Command::new("node")
            .arg(script_path)
            .env("TERMINAL_NODE_PACKAGE", &package_dir)
            .env("TERMINAL_NODE_READY_FILE", &ready_file)
            .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
            .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("package shutdown smoke should launch");

        wait_for_ready_file(&ready_file).await;
        fixture.shutdown().await.expect("fixture should stop cleanly");

        let output =
            child.wait_with_output().expect("package shutdown smoke should collect output");

        assert!(
            output.status.success(),
            "package shutdown smoke {script} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\"subscription_closed\":true"),
            "package shutdown smoke {script} should confirm subscription closure"
        );
        assert!(
            stdout.contains("\"watch_closed\":true"),
            "package shutdown smoke {script} should confirm watch closure"
        );

        let _ = std::fs::remove_file(&ready_file);
    }
}

async fn wait_for_ready_file(path: &std::path::Path) {
    for _ in 0..120 {
        if path.is_file() {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }

    panic!("node package shutdown smoke never became ready: {}", path.display());
}

fn unique_temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}
