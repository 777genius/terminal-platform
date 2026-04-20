use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use terminal_protocol::LocalSocketAddress;
use terminal_testing::daemon_fixture;

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_node_addon_against_daemon_fixture() {
    let fixture = daemon_fixture("terminal-node-napi-smoke").expect("fixture should start");
    let addon_path = materialize_node_addon().expect("node addon should be materialized");
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/node_smoke.cjs");
    let (address_kind, address_value) = match fixture.client.address() {
        LocalSocketAddress::Namespaced(value) => ("namespaced", value.clone()),
        LocalSocketAddress::Filesystem(path) => ("filesystem", path.display().to_string()),
    };

    let output = Command::new("node")
        .arg(script_path)
        .env("TERMINAL_NODE_ADDON", &addon_path)
        .env("TERMINAL_NODE_ADDRESS_KIND", address_kind)
        .env("TERMINAL_NODE_ADDRESS_VALUE", &address_value)
        .output()
        .expect("node smoke should launch");

    fixture.shutdown().await.expect("fixture should stop cleanly");

    assert!(
        output.status.success(),
        "node smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("\"session_id\""),
        "node smoke should emit structured confirmation"
    );
}

fn materialize_node_addon() -> std::io::Result<PathBuf> {
    let source = locate_cdylib()?;
    let target = unique_node_addon_path();
    fs::copy(&source, &target)?;
    Ok(target)
}

fn locate_cdylib() -> std::io::Result<PathBuf> {
    let test_binary = std::env::current_exe()?;
    let deps_dir = test_binary
        .parent()
        .ok_or_else(|| std::io::Error::other("test binary should have a parent dir"))?;
    let target_dir = deps_dir
        .parent()
        .ok_or_else(|| std::io::Error::other("deps dir should have a parent dir"))?;

    for dir in [deps_dir, target_dir] {
        for name in candidate_cdylib_names() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("could not find terminal-node-napi cdylib near {}", test_binary.display()),
    ))
}

fn unique_node_addon_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("terminal-node-napi-{}-{nanos}.node", std::process::id()))
}

fn candidate_cdylib_names() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    {
        &["libterminal_node_napi.dylib"]
    }

    #[cfg(target_os = "linux")]
    {
        &["libterminal_node_napi.so"]
    }

    #[cfg(target_os = "windows")]
    {
        &["terminal_node_napi.dll"]
    }
}
