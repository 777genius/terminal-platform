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
async fn installs_terminal_capi_package_into_prefix_layout() {
    let fixture = daemon_fixture("capi-install").expect("daemon fixture should start");
    let stage_dir = support::unique_temp_dir("terminal-capi-package");
    let prefix_dir = support::unique_temp_dir("terminal-capi-prefix");
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root should resolve")
        .to_path_buf();

    let stage_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "stage-capi-package", "--out"])
        .arg(&stage_dir)
        .current_dir(&workspace_root)
        .output()
        .expect("xtask stage-capi-package should launch");
    assert!(
        stage_output.status.success(),
        "xtask stage-capi-package failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&stage_output.stdout),
        String::from_utf8_lossy(&stage_output.stderr)
    );

    let install_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "install-capi-package", "--package-dir"])
        .arg(&stage_dir)
        .arg("--prefix")
        .arg(&prefix_dir)
        .current_dir(&workspace_root)
        .output()
        .expect("xtask install-capi-package should launch");
    assert!(
        install_output.status.success(),
        "xtask install-capi-package failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install_output.stdout),
        String::from_utf8_lossy(&install_output.stderr)
    );

    let verify_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "verify-capi-install", "--prefix"])
        .arg(&prefix_dir)
        .current_dir(&workspace_root)
        .output()
        .expect("xtask verify-capi-install should launch");
    assert!(
        verify_output.status.success(),
        "xtask verify-capi-install failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify_output.stdout),
        String::from_utf8_lossy(&verify_output.stderr)
    );

    let installed_manifest =
        support::read_json(&prefix_dir.join("share/terminal-capi/manifest.json"))
            .expect("installed manifest should parse");
    let exports =
        installed_manifest["exports"].as_object().expect("installed exports should be an object");
    let header_path = prefix_dir
        .join(exports["header"].as_str().expect("installed exports.header should be a string"));
    let cdylib_path = prefix_dir
        .join(exports["cdylib"].as_str().expect("installed exports.cdylib should be a string"));
    let staticlib_path = prefix_dir.join(
        exports["staticlib"].as_str().expect("installed exports.staticlib should be a string"),
    );
    let pkgconfig_path = prefix_dir.join(
        exports["pkgConfig"].as_str().expect("installed exports.pkgConfig should be a string"),
    );
    let readme_path = prefix_dir
        .join(exports["readme"].as_str().expect("installed exports.readme should be a string"));
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference_consumer.c");
    let binary_path = support::compile_reference_consumer_with_pkg_config(
        &source_path,
        &prefix_dir,
        "terminal-platform-capi",
    )
    .expect("reference consumer should compile against installed prefix via pkg-config");
    let mut consumer = Command::new(&binary_path);

    support::configure_runtime_library_path(&mut consumer, &cdylib_path);

    match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => {
            consumer.arg("namespaced").arg(value);
        }
        LocalSocketAddress::Filesystem(path) => {
            consumer.arg("filesystem").arg(path);
        }
    }
    consumer.arg("native");

    let consumer_output = consumer.output().expect("reference consumer should launch");
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
