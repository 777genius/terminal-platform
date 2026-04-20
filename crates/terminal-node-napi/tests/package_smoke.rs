use std::{path::PathBuf, process::Command};

use terminal_protocol::LocalSocketAddress;
use terminal_testing::daemon_fixture;

mod support;

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_staged_package_through_cjs_and_esm() {
    let fixture = daemon_fixture("terminal-node-package-smoke").expect("fixture should start");
    let addon_path = support::locate_cdylib().expect("node addon should be built");
    let package_dir =
        support::stage_node_package(&addon_path).expect("package should stage successfully");
    let (address_kind, address_value) = match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };

    for script in ["package_smoke.cjs", "package_smoke.mjs"] {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/{script}"));
        let output = Command::new("node")
            .arg(script_path)
            .env("TERMINAL_NODE_PACKAGE", &package_dir)
            .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
            .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
            .output()
            .expect("package smoke should launch");

        assert!(
            output.status.success(),
            "package smoke {script} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("\"session_id\""),
            "package smoke {script} should emit structured confirmation"
        );
    }

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
