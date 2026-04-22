#[cfg(unix)]
mod support;

#[cfg(unix)]
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
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
async fn stages_and_verifies_terminal_capi_package() {
    let fixture = daemon_fixture("capi-stage").expect("daemon fixture should start");
    wait_for_daemon_ready(&fixture.client).await;
    let stage_dir = support::unique_temp_dir("terminal-capi-package");
    let workspace_root = workspace_root();
    stage_capi_package(&workspace_root, &stage_dir);
    verify_staged_capi_package(&workspace_root, &stage_dir);
    let (binary_path, cdylib_path) = compile_staged_reference_consumer(&stage_dir);
    let consumer_output = run_staged_reference_consumer(
        &binary_path,
        &cdylib_path,
        fixture.client.address(),
        "native",
        &[],
    );
    let manifest = support::read_json(&stage_dir.join("manifest.json"))
        .expect("package manifest should parse");
    let exports = manifest["exports"].as_object().expect("exports should be an object");
    let header_path = stage_dir
        .join(exports["header"].as_str().expect("exports.header should be a relative path"));
    let staticlib_path = stage_dir
        .join(exports["staticlib"].as_str().expect("exports.staticlib should be a relative path"));
    let pkgconfig_path = stage_dir
        .join(exports["pkgConfig"].as_str().expect("exports.pkgConfig should be a relative path"));
    let readme_path = stage_dir.join("README.md");

    assert!(
        consumer_output.status.success(),
        "reference consumer against staged package failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&consumer_output.stdout),
        String::from_utf8_lossy(&consumer_output.stderr)
    );

    assert_eq!(manifest["package"], "terminal-capi");
    assert_eq!(manifest["schemaVersion"], 1);
    assert!(header_path.is_file(), "staged header should exist");
    assert!(cdylib_path.is_file(), "staged cdylib should exist");
    assert!(staticlib_path.is_file(), "staged staticlib should exist");
    assert!(pkgconfig_path.is_file(), "staged pkg-config file should exist");
    assert!(readme_path.is_file(), "staged README should exist");

    fixture
        .shutdown()
        .await
        .expect("daemon fixture should stop cleanly after staged package smoke");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn staged_terminal_capi_package_handles_shutdown_and_restart_flows() {
    let stage_dir = support::unique_temp_dir("terminal-capi-package");
    let workspace_root = workspace_root();
    stage_capi_package(&workspace_root, &stage_dir);
    verify_staged_capi_package(&workspace_root, &stage_dir);
    let (binary_path, cdylib_path) = compile_staged_reference_consumer(&stage_dir);

    let shutdown_ready_file = support::unique_temp_path("terminal-capi-stage-shutdown", "ready");
    let shutdown_fixture = daemon_fixture("capi-stage-shutdown")
        .expect("daemon fixture should start for stage shutdown smoke");
    wait_for_daemon_ready(&shutdown_fixture.client).await;
    let shutdown_child = spawn_staged_reference_consumer(
        &binary_path,
        &cdylib_path,
        shutdown_fixture.client.address(),
        "shutdown",
        &[("TERMINAL_CAPI_READY_FILE", shutdown_ready_file.as_path())],
    );
    support::wait_for_file(&shutdown_ready_file, "staged shutdown consumer ready file").await;
    shutdown_fixture
        .shutdown()
        .await
        .expect("daemon fixture should stop cleanly before staged shutdown consumer exits");
    let shutdown_output =
        shutdown_child.wait_with_output().expect("staged shutdown consumer should collect output");
    assert!(
        shutdown_output.status.success(),
        "staged shutdown consumer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&shutdown_output.stdout),
        String::from_utf8_lossy(&shutdown_output.stderr)
    );

    let restart_address = unique_socket_address("capi-stage-restart");
    let restart_client = LocalSocketDaemonClient::new(restart_address.clone());
    let initial_ready_file = support::unique_temp_path("terminal-capi-stage-restart", "ready");
    let stale_ready_file = support::unique_temp_path("terminal-capi-stage-restart", "stale");
    let restart_file = support::unique_temp_path("terminal-capi-stage-restart", "restart");
    let initial_server = spawn_local_socket_server(daemon(), restart_address.clone())
        .expect("initial daemon should start for staged restart consumer");
    wait_for_daemon_ready(&restart_client).await;
    let restart_child = spawn_staged_reference_consumer(
        &binary_path,
        &cdylib_path,
        &restart_address,
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
        "staged restart consumer initial ready",
    )
    .await;
    initial_server
        .shutdown()
        .await
        .expect("initial daemon should stop cleanly for staged restart consumer");
    let restart_child = support::wait_for_file_or_child_exit(
        &stale_ready_file,
        restart_child,
        "staged restart consumer stale signal",
    )
    .await;

    let replacement_client = LocalSocketDaemonClient::new(restart_address.clone());
    let replacement_server = spawn_local_socket_server(daemon(), restart_address)
        .expect("replacement daemon should start for staged restart consumer");
    wait_for_daemon_ready(&replacement_client).await;
    std::fs::write(&restart_file, "restart\n").expect("restart signal file should write");
    let restart_output =
        restart_child.wait_with_output().expect("staged restart consumer should collect output");
    replacement_server
        .shutdown()
        .await
        .expect("replacement daemon should stop cleanly for staged restart consumer");
    let restart_stdout = String::from_utf8_lossy(&restart_output.stdout);

    assert!(
        restart_output.status.success(),
        "staged restart consumer failed\nstdout:\n{}\nstderr:\n{}",
        restart_stdout,
        String::from_utf8_lossy(&restart_output.stderr)
    );
    assert!(
        restart_stdout.contains("\"stale_error_observed\":true"),
        "staged restart consumer should confirm stale daemon failure\nstdout:\n{}\nstderr:\n{}",
        restart_stdout,
        String::from_utf8_lossy(&restart_output.stderr)
    );
    assert!(
        restart_stdout.contains("\"recovered\":true"),
        "staged restart consumer should confirm recovery against restarted daemon\nstdout:\n{}\nstderr:\n{}",
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
fn verify_staged_capi_package(workspace_root: &Path, stage_dir: &Path) {
    let verify_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "verify-capi-package", "--package-dir"])
        .arg(stage_dir)
        .current_dir(workspace_root)
        .output()
        .expect("xtask verify-capi-package should launch");

    assert!(
        verify_output.status.success(),
        "xtask verify-capi-package failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify_output.stdout),
        String::from_utf8_lossy(&verify_output.stderr)
    );
}

#[cfg(unix)]
fn compile_staged_reference_consumer(stage_dir: &Path) -> (PathBuf, PathBuf) {
    let manifest = support::read_json(&stage_dir.join("manifest.json"))
        .expect("package manifest should parse");
    let exports = manifest["exports"].as_object().expect("exports should be an object");
    let cdylib_path = stage_dir
        .join(exports["cdylib"].as_str().expect("exports.cdylib should be a relative path"));
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference_consumer.c");
    let binary_path = support::compile_reference_consumer_with_pkg_config(
        &source_path,
        stage_dir,
        "terminal-platform-capi",
    )
    .expect("reference consumer should compile against staged package via pkg-config");

    (binary_path, cdylib_path)
}

#[cfg(unix)]
fn run_staged_reference_consumer(
    binary_path: &Path,
    cdylib_path: &Path,
    address: &LocalSocketAddress,
    mode: &str,
    envs: &[(&str, &Path)],
) -> std::process::Output {
    let child = spawn_staged_reference_consumer(binary_path, cdylib_path, address, mode, envs);
    child.wait_with_output().expect("staged reference consumer should collect output")
}

#[cfg(unix)]
fn spawn_staged_reference_consumer(
    binary_path: &Path,
    cdylib_path: &Path,
    address: &LocalSocketAddress,
    mode: &str,
    envs: &[(&str, &Path)],
) -> Child {
    let mut consumer = Command::new(binary_path);
    support::configure_runtime_library_path(&mut consumer, cdylib_path);

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

    consumer.spawn().expect("staged reference consumer should launch")
}
