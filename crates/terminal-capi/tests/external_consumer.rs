#[cfg(unix)]
mod support;

#[cfg(unix)]
use std::{path::PathBuf, process::Command};

#[cfg(unix)]
use terminal_protocol::LocalSocketAddress;
#[cfg(unix)]
use terminal_testing::{
    DaemonFixture, TmuxServerGuard, daemon_fixture, daemon_fixture_with_state, tmux_daemon_state,
    unique_tmux_session_name, unique_tmux_socket_name,
};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn external_c_consumer_roundtrips_against_terminal_capi_cdylib() {
    let fixture =
        daemon_fixture("capi-ext").expect("daemon fixture should start for external c consumer");
    run_reference_consumer(&fixture, "native");

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
    run_reference_consumer(&fixture, "tmux");

    fixture
        .shutdown()
        .await
        .expect("daemon fixture should stop cleanly after external tmux c consumer");
}

#[cfg(unix)]
fn run_reference_consumer(fixture: &DaemonFixture, mode: &str) {
    let header_path =
        support::generate_header().expect("c api header should generate for reference consumer");
    let cdylib_path = support::locate_cdylib().expect("terminal-capi cdylib should be built");
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference_consumer.c");
    let binary_path = support::compile_reference_consumer(&source_path, &header_path, &cdylib_path)
        .expect("reference c consumer should compile");
    let mut command = Command::new(&binary_path);

    support::configure_runtime_library_path(&mut command, &cdylib_path);

    match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => {
            command.arg("namespaced").arg(value);
        }
        LocalSocketAddress::Filesystem(path) => {
            command.arg("filesystem").arg(path);
        }
    }
    command.arg(mode);

    let output = command.output().expect("reference c consumer should launch");
    if !output.status.success() {
        panic!(
            "reference c consumer failed in mode {mode}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
