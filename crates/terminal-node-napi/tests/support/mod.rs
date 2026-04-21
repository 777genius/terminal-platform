#![allow(dead_code)]

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use terminal_testing::{ZellijSessionGuard, unique_zellij_session_name};

#[cfg(windows)]
pub struct WindowsZellijSmokeEnv {
    pub session_name: String,
    _guard: ZellijSessionGuard,
}

#[cfg(windows)]
pub fn windows_zellij_smoke_env(label: &str) -> WindowsZellijSmokeEnv {
    let session_name = unique_zellij_session_name(label);
    let guard = ZellijSessionGuard::spawn(&session_name)
        .expect("windows zellij smoke session should start");

    WindowsZellijSmokeEnv { session_name, _guard: guard }
}

pub fn locate_cdylib() -> std::io::Result<PathBuf> {
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

pub fn materialize_node_addon() -> std::io::Result<PathBuf> {
    let source = locate_cdylib()?;
    let target = unique_temp_path("terminal-node-napi", "node");
    fs::copy(&source, &target)?;
    Ok(target)
}

pub fn stage_node_package(addon_source: &Path) -> std::io::Result<PathBuf> {
    let stage_dir = unique_temp_dir("terminal-node-package");
    let script_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("package/scripts/stage-package.mjs");
    let mut command = Command::new("node");
    command.arg(script_path).arg("--out").arg(&stage_dir).arg("--addon").arg(addon_source);
    let output = command_output(&mut command, "package staging")?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "package staging failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(stage_dir)
}

pub fn verify_node_package(package_dir: &Path) -> std::io::Result<()> {
    let script_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("package/scripts/verify-package.mjs");
    let mut command = Command::new("node");
    command.arg(script_path).arg("--package-dir").arg(package_dir);
    let output = command_output(&mut command, "package verification")?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "package verification failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

pub fn pack_node_package(package_dir: &Path) -> std::io::Result<PathBuf> {
    let mut command = Command::new(node_package_manager());
    command.arg("pack").arg("--json").current_dir(package_dir);
    let output = command_output(&mut command, "npm pack")?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "npm pack failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        std::io::Error::other(format!("failed to parse npm pack output - {error}"))
    })?;
    let filename = payload
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("filename"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| std::io::Error::other("npm pack did not return a filename"))?;

    Ok(package_dir.join(filename))
}

pub fn install_node_package_tarball(tarball_path: &Path) -> std::io::Result<PathBuf> {
    let project_dir = unique_temp_dir("terminal-node-install");
    fs::create_dir_all(&project_dir)?;
    fs::write(
        project_dir.join("package.json"),
        concat!(
            "{\n",
            "  \"name\": \"terminal-node-install-smoke\",\n",
            "  \"private\": true\n",
            "}\n"
        ),
    )?;

    let mut command = Command::new(node_package_manager());
    command
        .args(["install", "--ignore-scripts", "--no-audit", "--no-fund", "--no-package-lock"])
        .arg(tarball_path)
        .current_dir(&project_dir);
    let output = command_output(&mut command, "npm install")?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "npm install failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(project_dir)
}

pub fn tar_list(archive_path: &Path) -> std::io::Result<Vec<String>> {
    let mut command = Command::new("tar");
    command.arg("-tf").arg(archive_path);
    let output = command_output(&mut command, "tar listing")?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "tar listing failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).lines().map(ToOwned::to_owned).collect())
}

pub fn read_json(path: &Path) -> std::io::Result<serde_json::Value> {
    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(|error| {
        std::io::Error::other(format!("failed to parse json {} - {error}", path.display()))
    })
}

pub fn write_json(path: &Path, value: &serde_json::Value) -> std::io::Result<()> {
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).map_err(|error| {
                std::io::Error::other(format!(
                    "failed to serialize json {} - {error}",
                    path.display()
                ))
            })?
        ),
    )
}

pub fn command_output(command: &mut Command, label: &str) -> io::Result<Output> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = command.spawn().map_err(|error| {
        io::Error::new(error.kind(), format!("{label} failed to launch - {error}"))
    })?;

    wait_child_output(child, label)
}

pub fn wait_child_output(child: Child, label: &str) -> io::Result<Output> {
    wait_child_output_with_timeout(child, process_timeout(), label)
}

pub fn wait_child_output_with_timeout(
    mut child: Child,
    timeout: Duration,
    label: &str,
) -> io::Result<Output> {
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().map_err(|error| {
                io::Error::new(error.kind(), format!("{label} output collection failed - {error}"))
            });
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output().map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("{label} timed out and output collection failed - {error}"),
                )
            })?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{label} timed out after {}ms\nstdout:\n{}\nstderr:\n{}",
                    timeout.as_millis(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        thread::sleep(Duration::from_millis(50));
    }
}

pub fn process_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(120) } else { Duration::from_secs(180) }
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    unique_temp_path(prefix, "dir")
}

fn unique_temp_path(prefix: &str, suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}.{suffix}", std::process::id()))
}

fn node_package_manager() -> &'static str {
    #[cfg(windows)]
    {
        "npm.cmd"
    }

    #[cfg(not(windows))]
    {
        "npm"
    }
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
