use std::{path::PathBuf, process::Command};

mod support;

#[test]
fn rejects_incompatible_native_manifest_targets() {
    let addon_path = support::locate_cdylib().expect("node addon should be built");
    let package_dir =
        support::stage_node_package(&addon_path).expect("package should stage successfully");
    let manifest_path = package_dir.join("native/manifest.json");
    let mut manifest = support::read_json(&manifest_path).expect("manifest should parse");
    manifest["targets"][0]["platform"] = serde_json::Value::String("definitely-not-this-os".into());
    manifest["targets"][0]["arch"] = serde_json::Value::String("definitely-not-this-arch".into());
    support::write_json(&manifest_path, &manifest).expect("manifest should rewrite");

    let script_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/package_incompatible_smoke.cjs");
    let mut command = Command::new("node");
    command.arg(script_path).env("TERMINAL_NODE_PACKAGE", &package_dir);
    let output = support::command_output(&mut command, "incompatible package smoke")
        .expect("incompatible package smoke should run");

    assert!(
        output.status.success(),
        "incompatible package smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("incompatible-target-rejected"),
        "incompatible package smoke should confirm manifest rejection"
    );
}
