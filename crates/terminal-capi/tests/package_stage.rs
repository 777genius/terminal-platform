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
async fn stages_and_verifies_terminal_capi_package() {
    let fixture = daemon_fixture("capi-stage").expect("daemon fixture should start");
    let stage_dir = support::unique_temp_dir("terminal-capi-package");
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

    let verify_output = Command::new("cargo")
        .args(["run", "-p", "xtask", "--", "verify-capi-package", "--package-dir"])
        .arg(&stage_dir)
        .current_dir(&workspace_root)
        .output()
        .expect("xtask verify-capi-package should launch");

    assert!(
        verify_output.status.success(),
        "xtask verify-capi-package failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&verify_output.stdout),
        String::from_utf8_lossy(&verify_output.stderr)
    );

    let manifest = support::read_json(&stage_dir.join("manifest.json"))
        .expect("package manifest should parse");
    let exports = manifest["exports"].as_object().expect("exports should be an object");
    let header_path = stage_dir
        .join(exports["header"].as_str().expect("exports.header should be a relative path"));
    let cdylib_path = stage_dir
        .join(exports["cdylib"].as_str().expect("exports.cdylib should be a relative path"));
    let staticlib_path = stage_dir
        .join(exports["staticlib"].as_str().expect("exports.staticlib should be a relative path"));
    let pkgconfig_path = stage_dir
        .join(exports["pkgConfig"].as_str().expect("exports.pkgConfig should be a relative path"));
    let readme_path = stage_dir.join("README.md");
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference_consumer.c");
    let binary_path = support::compile_reference_consumer_with_pkg_config(
        &source_path,
        &stage_dir,
        "terminal-platform-capi",
    )
    .expect("reference consumer should compile against staged package via pkg-config");
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
