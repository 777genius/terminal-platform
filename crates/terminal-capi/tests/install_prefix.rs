#[cfg(unix)]
mod support;

#[cfg(unix)]
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(unix)]
use terminal_daemon::spawn_local_socket_server;
#[cfg(unix)]
use terminal_daemon_client::LocalSocketDaemonClient;
#[cfg(unix)]
use terminal_protocol::LocalSocketAddress;
#[cfg(unix)]
use terminal_testing::{daemon, daemon_fixture, unique_socket_address, wait_for_daemon_ready};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn installs_terminal_capi_package_into_prefix_layout() {
    let fixture = daemon_fixture("capi-install").expect("daemon fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
    let stage_dir = support::unique_temp_dir("terminal-capi-package");
    let prefix_dir = support::unique_temp_dir("terminal-capi-prefix");
    let workspace_root = workspace_root();

    stage_capi_package(&workspace_root, &stage_dir);
    install_capi_package(&workspace_root, &stage_dir, &prefix_dir);
    verify_capi_install(&workspace_root, &prefix_dir);

    let (installed_manifest, cdylib_path, _binary_path) =
        compile_installed_reference_consumer(&prefix_dir);
    let exports =
        installed_manifest["exports"].as_object().expect("installed exports should be an object");
    let header_path = prefix_dir
        .join(exports["header"].as_str().expect("installed exports.header should be a string"));
    let staticlib_path = prefix_dir.join(
        exports["staticlib"].as_str().expect("installed exports.staticlib should be a string"),
    );
    let pkgconfig_path = prefix_dir.join(
        exports["pkgConfig"].as_str().expect("installed exports.pkgConfig should be a string"),
    );
    let readme_path = prefix_dir
        .join(exports["readme"].as_str().expect("installed exports.readme should be a string"));
    let consumer_output =
        run_installed_reference_consumer(&prefix_dir, fixture.client.address(), "native", &[]);
    assert!(
        consumer_output.status.success(),
        "reference consumer against installed prefix failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&consumer_output.stdout),
        String::from_utf8_lossy(&consumer_output.stderr)
    );

    assert_eq!(installed_manifest["layout"], "prefix");
    assert!(header_path.is_file(), "installed header should exist");
    assert!(cdylib_path.is_file(), "installed cdylib should exist");
    assert!(staticlib_path.is_file(), "installed staticlib should exist");
    assert!(pkgconfig_path.is_file(), "installed pkg-config file should exist");
    assert!(readme_path.is_file(), "installed README should exist");

    fixture
        .shutdown()
        .await
        .expect("daemon fixture should stop cleanly after install prefix smoke");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn installed_prefix_handles_shutdown_and_restart_flows() {
    let stage_dir = support::unique_temp_dir("terminal-capi-package");
    let prefix_dir = support::unique_temp_dir("terminal-capi-prefix");
    let workspace_root = workspace_root();

    stage_capi_package(&workspace_root, &stage_dir);
    install_capi_package(&workspace_root, &stage_dir, &prefix_dir);
    verify_capi_install(&workspace_root, &prefix_dir);

    let shutdown_ready_file = support::unique_temp_path("terminal-capi-install-shutdown", "ready");
    let fixture = daemon_fixture("capi-install-shutdown").expect("daemon fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
    let shutdown_child = spawn_installed_reference_consumer(
        &prefix_dir,
        fixture.client.address(),
        "shutdown",
        &[("TERMINAL_CAPI_READY_FILE", shutdown_ready_file.as_path())],
    );
    support::wait_for_file(&shutdown_ready_file, "installed shutdown consumer ready file").await;
    fixture
        .shutdown()
        .await
        .expect("daemon fixture should stop cleanly before install shutdown consumer exits");
    let shutdown_output = shutdown_child
        .wait_with_output()
        .expect("installed prefix shutdown consumer should collect output");
    assert!(
        shutdown_output.status.success(),
        "installed prefix shutdown consumer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&shutdown_output.stdout),
        String::from_utf8_lossy(&shutdown_output.stderr)
    );

    let address = unique_socket_address("capi-install-restart");
    let initial_client = LocalSocketDaemonClient::new(address.clone());
    let initial_ready_file = support::unique_temp_path("terminal-capi-install-restart", "ready");
    let stale_ready_file = support::unique_temp_path("terminal-capi-install-restart", "stale");
    let restart_file = support::unique_temp_path("terminal-capi-install-restart", "restart");
    let mut server =
        spawn_local_socket_server(daemon(), address.clone()).expect("initial daemon should bind");
    wait_for_daemon_ready(&initial_client).await;
    let restart_child = spawn_installed_reference_consumer(
        &prefix_dir,
        &address,
        "restart",
        &[
            ("TERMINAL_CAPI_INITIAL_READY_FILE", initial_ready_file.as_path()),
            ("TERMINAL_CAPI_STALE_READY_FILE", stale_ready_file.as_path()),
            ("TERMINAL_CAPI_RESTART_FILE", restart_file.as_path()),
        ],
    );
    let restart_child = support::wait_for_file_or_child_exit(
        &initial_ready_file,
        restart_child,
        "installed restart consumer initial ready",
    )
    .await;
    server.shutdown().await.expect("initial daemon should stop cleanly");
    let restart_child = support::wait_for_file_or_child_exit(
        &stale_ready_file,
        restart_child,
        "installed restart consumer stale signal",
    )
    .await;

    let restarted_client = LocalSocketDaemonClient::new(address.clone());
    server = spawn_local_socket_server(daemon(), address.clone())
        .expect("replacement daemon should bind");
    wait_for_daemon_ready(&restarted_client).await;
    std::fs::write(&restart_file, "restart\n").expect("restart signal file should write");

    let restart_output = restart_child
        .wait_with_output()
        .expect("installed prefix restart consumer should collect output");
    server.shutdown().await.expect("replacement daemon should stop cleanly");
    let restart_stdout = String::from_utf8_lossy(&restart_output.stdout);

    assert!(
        restart_output.status.success(),
        "installed prefix restart consumer failed\nstdout:\n{}\nstderr:\n{}",
        restart_stdout,
        String::from_utf8_lossy(&restart_output.stderr)
    );
    assert!(
        restart_stdout.contains("\"stale_error_observed\":true"),
        "installed prefix restart consumer should confirm stale daemon failure\nstdout:\n{}\nstderr:\n{}",
        restart_stdout,
        String::from_utf8_lossy(&restart_output.stderr)
    );
    assert!(
        restart_stdout.contains("\"recovered\":true"),
        "installed prefix restart consumer should confirm recovery against restarted daemon\nstdout:\n{}\nstderr:\n{}",
        restart_stdout,
        String::from_utf8_lossy(&restart_output.stderr)
    );

    let _ = std::fs::remove_file(&shutdown_ready_file);
    let _ = std::fs::remove_file(&initial_ready_file);
    let _ = std::fs::remove_file(&stale_ready_file);
    let _ = std::fs::remove_file(&restart_file);
}

#[cfg(unix)]
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should resolve")
        .to_path_buf()
}

#[cfg(unix)]
fn stage_capi_package(workspace_root: &Path, stage_dir: &Path) {
    let stage_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "stage-capi-package", "--out"])
        .arg(stage_dir)
        .current_dir(workspace_root)
        .output()
        .expect("xtask stage-capi-package should launch");
    assert!(
        stage_output.status.success(),
        "xtask stage-capi-package failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&stage_output.stdout),
        String::from_utf8_lossy(&stage_output.stderr)
    );
}

#[cfg(unix)]
fn install_capi_package(workspace_root: &Path, stage_dir: &Path, prefix_dir: &Path) {
    let install_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "install-capi-package", "--package-dir"])
        .arg(stage_dir)
        .arg("--prefix")
        .arg(prefix_dir)
        .current_dir(workspace_root)
        .output()
        .expect("xtask install-capi-package should launch");
    assert!(
        install_output.status.success(),
        "xtask install-capi-package failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install_output.stdout),
        String::from_utf8_lossy(&install_output.stderr)
    );
}

#[cfg(unix)]
fn verify_capi_install(workspace_root: &Path, prefix_dir: &Path) {
    let verify_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "verify-capi-install", "--prefix"])
        .arg(prefix_dir)
        .current_dir(workspace_root)
        .output()
        .expect("xtask verify-capi-install should launch");
    assert!(
        verify_output.status.success(),
        "xtask verify-capi-install failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify_output.stdout),
        String::from_utf8_lossy(&verify_output.stderr)
    );
}

#[cfg(unix)]
fn compile_installed_reference_consumer(
    prefix_dir: &Path,
) -> (serde_json::Value, PathBuf, PathBuf) {
    let installed_manifest =
        support::read_json(&prefix_dir.join("share/terminal-capi/manifest.json"))
            .expect("installed manifest should parse");
    let exports =
        installed_manifest["exports"].as_object().expect("installed exports should be an object");
    let cdylib_path = prefix_dir
        .join(exports["cdylib"].as_str().expect("installed exports.cdylib should be a string"));
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference_consumer.c");
    let binary_path = support::compile_reference_consumer_with_pkg_config(
        &source_path,
        prefix_dir,
        "terminal-platform-capi",
    )
    .expect("reference consumer should compile against installed prefix via pkg-config");

    (installed_manifest, cdylib_path, binary_path)
}

#[cfg(unix)]
fn run_installed_reference_consumer(
    prefix_dir: &Path,
    address: &LocalSocketAddress,
    mode: &str,
    envs: &[(&str, &Path)],
) -> std::process::Output {
    spawn_installed_reference_consumer(prefix_dir, address, mode, envs)
        .wait_with_output()
        .expect("installed prefix reference consumer should collect output")
}

#[cfg(unix)]
fn spawn_installed_reference_consumer(
    prefix_dir: &Path,
    address: &LocalSocketAddress,
    mode: &str,
    envs: &[(&str, &Path)],
) -> std::process::Child {
    let (_manifest, cdylib_path, binary_path) = compile_installed_reference_consumer(prefix_dir);
    let mut consumer = Command::new(&binary_path);

    support::configure_runtime_library_path(&mut consumer, &cdylib_path);

    match address {
        LocalSocketAddress::Namespaced(value) => {
            consumer.arg("namespaced").arg(value);
        }
        LocalSocketAddress::Filesystem(path) => {
            consumer.arg("filesystem").arg(path);
        }
    }
    consumer.arg(mode);
    for (key, value) in envs {
        consumer.env(key, value);
    }
    consumer.stdout(Stdio::piped());
    consumer.stderr(Stdio::piped());

    consumer.spawn().expect("installed prefix reference consumer should launch")
}
