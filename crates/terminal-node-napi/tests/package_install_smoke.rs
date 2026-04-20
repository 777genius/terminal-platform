use std::{fs, path::PathBuf, process::Command};

use terminal_protocol::LocalSocketAddress;
use terminal_testing::daemon_fixture;

mod support;

const INSTALL_SMOKE_CJS: &str = r#"const { runPackageWatchSmoke, runSmoke } = require(process.env.TERMINAL_NODE_SMOKE_FLOW);

function createClient(sdk) {
  const kind = process.env.TERMINAL_NODE_ADDRESS_KIND;
  const value = process.env.TERMINAL_NODE_ADDRESS_VALUE;

  if (kind === "namespaced") {
    return sdk.TerminalNodeClient.fromNamespacedAddress(value);
  }

  if (kind === "filesystem") {
    return sdk.TerminalNodeClient.fromFilesystemPath(value);
  }

  throw new Error(`Unsupported address kind: ${kind}`);
}

async function main() {
  const sdk = require("terminal-platform-node");
  await runSmoke(() => createClient(sdk));
  await runPackageWatchSmoke(() => createClient(sdk), sdk);
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
"#;

const INSTALL_SMOKE_MJS: &str = r#"import sdk from "terminal-platform-node";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { runPackageWatchSmoke, runSmoke } = require(process.env.TERMINAL_NODE_SMOKE_FLOW);

function createClient(binding) {
  const kind = process.env.TERMINAL_NODE_ADDRESS_KIND;
  const value = process.env.TERMINAL_NODE_ADDRESS_VALUE;

  if (kind === "namespaced") {
    return binding.TerminalNodeClient.fromNamespacedAddress(value);
  }

  if (kind === "filesystem") {
    return binding.TerminalNodeClient.fromFilesystemPath(value);
  }

  throw new Error(`Unsupported address kind: ${kind}`);
}

async function main() {
  await runSmoke(() => createClient(sdk));
  await runPackageWatchSmoke(() => createClient(sdk), sdk);
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
"#;

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_installed_tarball_through_cjs_and_esm() {
    let fixture = daemon_fixture("node-npm-install").expect("fixture should start");
    let addon_path = support::locate_cdylib().expect("node addon should be built");
    let package_dir =
        support::stage_node_package(&addon_path).expect("package should stage successfully");
    support::verify_node_package(&package_dir).expect("package should verify successfully");
    let tarball_path = support::pack_node_package(&package_dir).expect("package should pack");
    let install_dir = support::install_node_package_tarball(&tarball_path)
        .expect("packed tarball should install into temp project");
    let installed_package_dir = install_dir.join("node_modules/terminal-platform-node");
    let installed_manifest =
        support::read_json(&installed_package_dir.join("package.json")).expect("manifest exists");
    let native_manifest = support::read_json(&installed_package_dir.join("native/manifest.json"))
        .expect("native manifest exists");
    let native_file =
        native_manifest["targets"][0]["file"].as_str().expect("native target file should exist");
    let smoke_flow_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/smoke_flow.cjs");

    assert_eq!(installed_manifest["name"], "terminal-platform-node");
    assert_eq!(installed_manifest["main"], "./index.cjs");
    assert_eq!(installed_manifest["module"], "./index.mjs");
    assert!(installed_package_dir.join("index.cjs").is_file());
    assert!(installed_package_dir.join("index.mjs").is_file());
    assert!(installed_package_dir.join("index.d.ts").is_file());
    assert!(installed_package_dir.join("bindings").is_dir());
    assert!(
        installed_package_dir.join("native").join(native_file).is_file(),
        "installed package should contain manifest-selected addon"
    );

    fs::write(install_dir.join("install_smoke.cjs"), INSTALL_SMOKE_CJS)
        .expect("cjs install smoke should write");
    fs::write(install_dir.join("install_smoke.mjs"), INSTALL_SMOKE_MJS)
        .expect("esm install smoke should write");

    let (address_kind, address_value) = match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };

    for script in ["install_smoke.cjs", "install_smoke.mjs"] {
        let output = Command::new("node")
            .arg(install_dir.join(script))
            .current_dir(&install_dir)
            .env("TERMINAL_NODE_SMOKE_FLOW", &smoke_flow_path)
            .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
            .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
            .output()
            .expect("installed package smoke should launch");

        assert!(
            output.status.success(),
            "installed package smoke {script} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("\"session_id\""),
            "installed package smoke {script} should emit structured confirmation"
        );
    }

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
