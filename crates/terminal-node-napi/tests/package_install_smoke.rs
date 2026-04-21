use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_protocol::LocalSocketAddress;
#[cfg(not(windows))]
use terminal_testing::ZellijTestLock;
use terminal_testing::{daemon_fixture, unique_socket_address, wait_for_daemon_ready};
use tokio::time::{Duration, sleep};

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

const INSTALL_SHUTDOWN_SMOKE_CJS: &str = r#"const fs = require("node:fs/promises");
const { runShutdownSmoke } = require(process.env.TERMINAL_NODE_SMOKE_FLOW);
const sdk = require("terminal-platform-node");

function createClient() {
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
  const result = await runShutdownSmoke(() => createClient(), {
    onReady: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_READY_FILE, "ready\n");
    },
  });
  process.stdout.write(JSON.stringify(result));
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
"#;

const INSTALL_SHUTDOWN_SMOKE_MJS: &str = r#"import fs from "node:fs/promises";
import sdk from "terminal-platform-node";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { runShutdownSmoke } = require(process.env.TERMINAL_NODE_SMOKE_FLOW);

function createClient() {
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
  const result = await runShutdownSmoke(() => createClient(), {
    onReady: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_READY_FILE, "ready\n");
    },
  });
  process.stdout.write(JSON.stringify(result));
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
"#;

const INSTALL_RESTART_SMOKE_CJS: &str = r#"const fs = require("node:fs/promises");
const { runRestartRecoverySmoke } = require(process.env.TERMINAL_NODE_SMOKE_FLOW);
const sdk = require("terminal-platform-node");

function createClient() {
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

async function waitForFile(path, label) {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    try {
      await fs.access(path);
      return;
    } catch (_error) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  throw new Error(`Timed out waiting for ${label}: ${path}`);
}

async function main() {
  const result = await runRestartRecoverySmoke(() => createClient(), {
    onInitialReady: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_INITIAL_READY_FILE, "ready\n");
    },
    waitForStop: async () => {
      await waitForFile(process.env.TERMINAL_NODE_STOP_FILE, "daemon stop signal");
    },
    onStaleObserved: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_STALE_READY_FILE, "stale\n");
    },
    waitForRestart: async () => {
      await waitForFile(process.env.TERMINAL_NODE_RESTART_FILE, "daemon restart signal");
    },
  });
  process.stdout.write(JSON.stringify(result));
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
"#;

const INSTALL_RESTART_SMOKE_MJS: &str = r#"import fs from "node:fs/promises";
import sdk from "terminal-platform-node";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { runRestartRecoverySmoke } = require(process.env.TERMINAL_NODE_SMOKE_FLOW);

function createClient() {
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

async function waitForFile(path, label) {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    try {
      await fs.access(path);
      return;
    } catch (_error) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  throw new Error(`Timed out waiting for ${label}: ${path}`);
}

async function main() {
  const result = await runRestartRecoverySmoke(() => createClient(), {
    onInitialReady: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_INITIAL_READY_FILE, "ready\n");
    },
    waitForStop: async () => {
      await waitForFile(process.env.TERMINAL_NODE_STOP_FILE, "daemon stop signal");
    },
    onStaleObserved: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_STALE_READY_FILE, "stale\n");
    },
    waitForRestart: async () => {
      await waitForFile(process.env.TERMINAL_NODE_RESTART_FILE, "daemon restart signal");
    },
  });
  process.stdout.write(JSON.stringify(result));
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
"#;

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_installed_tarball_through_cjs_and_esm() {
    #[cfg(windows)]
    let zellij_smoke = support::windows_zellij_smoke_env("package-installed");
    #[cfg(not(windows))]
    let _zellij_lock = ZellijTestLock::acquire().expect("zellij test lock should acquire");

    let fixture = daemon_fixture("node-npm-install").expect("fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
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
        let mut command = Command::new("node");
        command
            .arg(install_dir.join(script))
            .current_dir(&install_dir)
            .env("TERMINAL_NODE_SMOKE_FLOW", &smoke_flow_path)
            .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
            .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value);
        #[cfg(windows)]
        command
            .env("TERMINAL_NODE_RUN_ZELLIJ_SMOKE", "1")
            .env("TERMINAL_NODE_EXTERNAL_ZELLIJ_SESSION", &zellij_smoke.session_name);
        let output = support::command_output(&mut command, "installed package smoke")
            .expect("installed package smoke should run");

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

#[tokio::test(flavor = "multi_thread")]
async fn installed_tarball_handles_shutdown_and_restart_flows() {
    #[cfg(not(windows))]
    let _zellij_lock = ZellijTestLock::acquire().expect("zellij test lock should acquire");
    let addon_path = support::locate_cdylib().expect("node addon should be built");
    let package_dir =
        support::stage_node_package(&addon_path).expect("package should stage successfully");
    support::verify_node_package(&package_dir).expect("package should verify successfully");
    let tarball_path = support::pack_node_package(&package_dir).expect("package should pack");
    let install_dir = support::install_node_package_tarball(&tarball_path)
        .expect("packed tarball should install into temp project");
    let smoke_flow_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/smoke_flow.cjs");

    fs::write(install_dir.join("install_shutdown_smoke.cjs"), INSTALL_SHUTDOWN_SMOKE_CJS)
        .expect("cjs install shutdown smoke should write");
    fs::write(install_dir.join("install_shutdown_smoke.mjs"), INSTALL_SHUTDOWN_SMOKE_MJS)
        .expect("esm install shutdown smoke should write");
    fs::write(install_dir.join("install_restart_smoke.cjs"), INSTALL_RESTART_SMOKE_CJS)
        .expect("cjs install restart smoke should write");
    fs::write(install_dir.join("install_restart_smoke.mjs"), INSTALL_RESTART_SMOKE_MJS)
        .expect("esm install restart smoke should write");

    for (script, fixture_label) in [
        ("install_shutdown_smoke.cjs", "node-install-close-cjs"),
        ("install_shutdown_smoke.mjs", "node-install-close-mjs"),
    ] {
        let fixture = daemon_fixture(fixture_label).expect("fixture should start");
        wait_for_daemon_ready(&fixture.client).await;
        let (address_kind, address_value) = match fixture.client.address() {
            LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
            LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
        };
        let ready_file = unique_temp_path("terminal-node-install-close", "ready");
        let mut child = spawn_install_script(
            &install_dir,
            script,
            &[
                ("TERMINAL_NODE_SMOKE_FLOW", smoke_flow_path.as_path()),
                ("TERMINAL_NODE_READY_FILE", ready_file.as_path()),
                ("TERMINAL_NODE_ADDRESS_KIND", Path::new(address_kind)),
                ("TERMINAL_NODE_ADDRESS_VALUE", Path::new(&address_value)),
            ],
        );
        if !wait_for_file(&ready_file).await {
            let _ = child.kill();
            let output = support::wait_child_output(child, "installed shutdown smoke")
                .expect("installed shutdown smoke should collect output");
            panic!(
                "installed package shutdown smoke never observed file: {}\nstdout:\n{}\nstderr:\n{}",
                ready_file.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        fixture.shutdown().await.expect("fixture should stop cleanly");
        let output = support::wait_child_output(child, "installed shutdown smoke")
            .expect("installed shutdown smoke should collect output");

        assert!(
            output.status.success(),
            "installed package shutdown smoke {script} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\"subscription_closed\":true"),
            "installed package shutdown smoke {script} should confirm subscription closure"
        );
        assert!(
            stdout.contains("\"watch_closed\":true"),
            "installed package shutdown smoke {script} should confirm watch closure"
        );

        let _ = std::fs::remove_file(&ready_file);
    }

    for (script, label) in [
        ("install_restart_smoke.cjs", "node-install-restart-cjs"),
        ("install_restart_smoke.mjs", "node-install-restart-mjs"),
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
        let initial_ready_file = unique_temp_path("terminal-node-install-restart", "ready");
        let stop_file = unique_temp_path("terminal-node-install-restart", "stop");
        let stale_ready_file = unique_temp_path("terminal-node-install-restart", "stale");
        let restart_file = unique_temp_path("terminal-node-install-restart", "restart");

        let mut child = spawn_install_script(
            &install_dir,
            script,
            &[
                ("TERMINAL_NODE_SMOKE_FLOW", smoke_flow_path.as_path()),
                ("TERMINAL_NODE_INITIAL_READY_FILE", initial_ready_file.as_path()),
                ("TERMINAL_NODE_STOP_FILE", stop_file.as_path()),
                ("TERMINAL_NODE_STALE_READY_FILE", stale_ready_file.as_path()),
                ("TERMINAL_NODE_RESTART_FILE", restart_file.as_path()),
                ("TERMINAL_NODE_ADDRESS_KIND", Path::new(address_kind)),
                ("TERMINAL_NODE_ADDRESS_VALUE", Path::new(&address_value)),
            ],
        );

        if !wait_for_file(&initial_ready_file).await {
            let _ = child.kill();
            let output = support::wait_child_output(child, "installed restart smoke")
                .expect("installed restart smoke should collect output");
            panic!(
                "installed package restart smoke never observed file: {}\nstdout:\n{}\nstderr:\n{}",
                initial_ready_file.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        server.shutdown().await.expect("initial daemon should stop cleanly");
        std::fs::write(&stop_file, "stopped\n").expect("stop signal file should write");
        if !wait_for_file(&stale_ready_file).await {
            let _ = child.kill();
            let output = support::wait_child_output(child, "installed restart smoke")
                .expect("installed restart smoke should collect output");
            panic!(
                "installed package restart smoke never observed file: {}\nstdout:\n{}\nstderr:\n{}",
                stale_ready_file.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let restarted_client = LocalSocketDaemonClient::new(address.clone());
        server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("replacement daemon should bind");
        wait_for_daemon_ready(&restarted_client).await;
        std::fs::write(&restart_file, "restart\n").expect("restart signal file should write");

        let output = support::wait_child_output(child, "installed restart smoke")
            .expect("installed restart smoke should collect output");
        server.shutdown().await.expect("replacement daemon should stop cleanly");

        assert!(
            output.status.success(),
            "installed package restart smoke {script} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\"stale_error_observed\":true"),
            "installed package restart smoke {script} should confirm stale daemon failure"
        );
        assert!(
            stdout.contains("\"recovered\":true"),
            "installed package restart smoke {script} should confirm recovery against restarted daemon"
        );

        let _ = std::fs::remove_file(&initial_ready_file);
        let _ = std::fs::remove_file(&stop_file);
        let _ = std::fs::remove_file(&stale_ready_file);
        let _ = std::fs::remove_file(&restart_file);
    }
}

fn spawn_install_script(
    install_dir: &Path,
    script: &str,
    envs: &[(&str, &Path)],
) -> std::process::Child {
    let mut command = Command::new("node");
    command.arg(install_dir.join(script)).current_dir(install_dir);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    command.spawn().expect("installed package smoke script should launch")
}

async fn wait_for_file(path: &Path) -> bool {
    for _ in 0..600 {
        if path.is_file() {
            return true;
        }
        sleep(Duration::from_millis(50)).await;
    }

    false
}

fn unique_temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}
