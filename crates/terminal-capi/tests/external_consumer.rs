#[cfg(unix)]
mod support;

#[cfg(unix)]
use std::{path::PathBuf, process::Command};

#[cfg(unix)]
use terminal_protocol::LocalSocketAddress;
#[cfg(unix)]
use terminal_testing::daemon_fixture;

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn external_c_consumer_roundtrips_against_terminal_capi_cdylib() {
    let fixture = daemon_fixture("terminal-capi-external-consumer")
        .expect("daemon fixture should start for external c consumer");
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

    let output = command.output().expect("reference c consumer should launch");
    if !output.status.success() {
        panic!(
            "reference c consumer failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fixture.shutdown().await.expect("daemon fixture should stop cleanly after external c consumer");
}
