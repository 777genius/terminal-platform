use std::{path::PathBuf, process::Command};

use terminal_protocol::LocalSocketAddress;
use terminal_testing::{daemon_fixture, wait_for_daemon_ready};

mod support;

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_staged_package_through_cjs_and_esm() {
    let fixture = daemon_fixture("terminal-node-package-smoke").expect("fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
    let addon_path = support::locate_cdylib().expect("node addon should be built");
    let package_dir =
        support::stage_node_package(&addon_path).expect("package should stage successfully");
    support::verify_node_package(&package_dir).expect("package should verify successfully");
    let native_manifest = support::read_json(&package_dir.join("native/manifest.json"))
        .expect("manifest should parse");
    let native_file =
        native_manifest["targets"][0]["file"].as_str().expect("manifest target file should exist");
    let tarball_path = support::pack_node_package(&package_dir).expect("package should pack");
    let archive_entries = support::tar_list(&tarball_path).expect("tarball should be readable");
    let (address_kind, address_value) = match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };

    for required_entry in [
        "package/package.json",
        "package/README.md",
        "package/index.cjs",
        "package/index.mjs",
        "package/index.d.ts",
        "package/native/manifest.json",
    ] {
        assert!(
            archive_entries.iter().any(|entry| entry == required_entry),
            "tarball should contain {required_entry}"
        );
    }
    assert!(
        archive_entries.iter().any(|entry| entry == &format!("package/native/{native_file}")),
        "tarball should contain manifest-selected native file"
    );

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
