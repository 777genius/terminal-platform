#[cfg(unix)]
mod support;

#[cfg(unix)]
use std::{path::{Path, PathBuf}, process::{Child, Command, Stdio}, time::Duration};

#[cfg(unix)]
use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
#[cfg(unix)]
use terminal_daemon_client::LocalSocketDaemonClient;
#[cfg(unix)]
use terminal_protocol::LocalSocketAddress;
#[cfg(unix)]
use terminal_testing::{
    TmuxServerGuard, daemon_fixture, daemon_fixture_with_state, daemon_state, tmux_daemon_state,
    unique_socket_address, unique_tmux_session_name, unique_tmux_socket_name, wait_for_daemon_ready,
};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn external_c_consumer_roundtrips_against_terminal_capi_cdylib() {
    let fixture =
        daemon_fixture("capi-ext").expect("daemon fixture should start for external c consumer");
    wait_for_daemon_ready(&fixture.client).await;
    run_reference_consumer(fixture.client.address(), "native");

    fixture.shutdown().await.expect("daemon fixture should stop cleanly after external c consumer");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn external_c_consumer_discovers_and_imports_tmux_sessions() {
    let socket_name = unique_tmux_socket_name("capi-ext");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux server should start");
    let fixture = daemon_fixture_with_state("capi-ext-tmux", tmux_daemon_state(&socket_name))
        .expect("daemon fixture should start for external tmux c consumer");
    wait_for_daemon_ready(&fixture.client).await;
    run_reference_consumer(fixture.client.address(), "tmux");

    fixture
        .shutdown()
        .await
        .expect("daemon fixture should stop cleanly after external tmux c consumer");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn external_c_consumer_closes_subscriptions_when_daemon_shuts_down() {
    let ready_file = support::unique_temp_path("terminal-capi-shutdown", "ready");
    let fixture =
        daemon_fixture("capi-ext-shutdown").expect("daemon fixture should start for shutdown smoke");
    wait_for_daemon_ready(&fixture.client).await;

    let child = spawn_reference_consumer(
        fixture.client.address(),
        "shutdown",
        &[("TERMINAL_CAPI_READY_FILE", ready_file.as_path())],
    );
    wait_for_file(&ready_file).await;
    fixture.shutdown().await.expect("daemon fixture should stop cleanly before shutdown consumer exits");
    let output = child
        .wait_with_output()
        .expect("shutdown reference consumer should collect output");

    assert!(
        output.status.success(),
        "shutdown reference consumer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&ready_file);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn external_c_consumer_recovers_after_daemon_restart() {
    let address = unique_socket_address("capi-ext-restart");
    let initial_client = LocalSocketDaemonClient::new(address.clone());
    let initial_ready_file = support::unique_temp_path("terminal-capi-restart", "ready");
    let stale_ready_file = support::unique_temp_path("terminal-capi-restart", "stale");
    let restart_file = support::unique_temp_path("terminal-capi-restart", "restart");

    let server = spawn_local_socket_server(
        TerminalDaemon::new(daemon_state()),
        address.clone(),
    )
    .expect("initial daemon should start for restart consumer");
    wait_for_daemon_ready(&initial_client).await;
    let child = spawn_reference_consumer(
        &address,
        "restart",
        &[
            ("TERMINAL_CAPI_INITIAL_READY_FILE", initial_ready_file.as_path()),
            ("TERMINAL_CAPI_STALE_READY_FILE", stale_ready_file.as_path()),
            ("TERMINAL_CAPI_RESTART_FILE", restart_file.as_path()),
        ],
    );

    wait_for_file(&initial_ready_file).await;
    server.shutdown().await.expect("initial daemon should stop cleanly");
    wait_for_file(&stale_ready_file).await;

    let restarted_client = LocalSocketDaemonClient::new(address.clone());
    let replacement =
        spawn_local_socket_server(TerminalDaemon::new(daemon_state()), address)
            .expect("replacement daemon should start for restart consumer");
    wait_for_daemon_ready(&restarted_client).await;
    std::fs::write(&restart_file, "restart\n").expect("restart signal file should write");

    let output = child
        .wait_with_output()
        .expect("restart reference consumer should collect output");
    replacement.shutdown().await.expect("replacement daemon should stop cleanly");

    assert!(
        output.status.success(),
        "restart reference consumer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"stale_error_observed\":true"),
        "restart reference consumer should confirm stale daemon failure\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("\"recovered\":true"),
        "restart reference consumer should confirm recovery against restarted daemon\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = std::fs::remove_file(&initial_ready_file);
    let _ = std::fs::remove_file(&stale_ready_file);
    let _ = std::fs::remove_file(&restart_file);
}

#[cfg(unix)]
fn run_reference_consumer(address: &LocalSocketAddress, mode: &str) {
    let output = spawn_reference_consumer(address, mode, &[])
        .wait_with_output()
        .expect("reference c consumer should collect output");

    if !output.status.success() {
        panic!(
            "reference c consumer failed in mode {mode}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(unix)]
fn spawn_reference_consumer(
    address: &LocalSocketAddress,
    mode: &str,
    envs: &[(&str, &Path)],
) -> Child {
    let header_path =
        support::generate_header().expect("c api header should generate for reference consumer");
    let cdylib_path = support::locate_cdylib().expect("terminal-capi cdylib should be built");
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference_consumer.c");
    let binary_path = support::compile_reference_consumer(&source_path, &header_path, &cdylib_path)
        .expect("reference c consumer should compile");
    let mut command = Command::new(&binary_path);

    support::configure_runtime_library_path(&mut command, &cdylib_path);

    match address {
        LocalSocketAddress::Namespaced(value) => {
            command.arg("namespaced").arg(value);
        }
        LocalSocketAddress::Filesystem(path) => {
            command.arg("filesystem").arg(path);
        }
    }
    command.arg(mode);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    command.spawn().expect("reference c consumer should launch")
}

#[cfg(unix)]
async fn wait_for_file(path: &Path) {
    for _ in 0..200 {
        if path.is_file() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    panic!("reference c consumer never observed file: {}", path.display());
}
