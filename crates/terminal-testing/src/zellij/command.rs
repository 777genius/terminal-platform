use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
fn resolve_windows_executable_path(program: &str) -> Option<PathBuf> {
    let has_path_separator = program.contains('\\') || program.contains('/');
    if has_path_separator {
        let path = PathBuf::from(program);
        return path.is_file().then_some(path);
    }

    let candidates = if program.to_ascii_lowercase().ends_with(".exe") {
        vec![program.to_string()]
    } else {
        vec![program.to_string(), format!("{program}.exe")]
    };

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for candidate in &candidates {
                let path = dir.join(candidate);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }

    None
}
#[cfg(any(unix, windows))]
pub(super) fn run_zellij(args: &[&str]) -> Result<String, String> {
    let output = run_zellij_with_timeout(args, zellij_command_timeout())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid zellij utf8 output: {error}"))
}

#[cfg(any(unix, windows))]
pub(super) fn run_zellij_with_timeout(args: &[&str], timeout: Duration) -> Result<Output, String> {
    let mut child = Command::new(zellij_command_path())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn zellij: {error}"))?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| format!("failed to collect zellij output: {error}"));
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let output = child.wait_with_output().map_err(|error| {
                    format!("failed to collect timed-out zellij output: {error}")
                })?;
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "zellij command timed out after {}ms: zellij {}; stderr: {}",
                    timeout.as_millis(),
                    args.join(" "),
                    stderr.trim()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(format!("failed while waiting for zellij: {error}")),
        }
    }
}

#[cfg(any(unix, windows))]
pub(super) fn zellij_command_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(10) } else { Duration::from_secs(5) }
}

#[cfg(unix)]
pub(super) fn zellij_create_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(20) } else { Duration::from_secs(10) }
}

#[cfg(unix)]
pub(super) fn is_headless_zellij_spawn_error(stderr: &str) -> bool {
    stderr.contains("could not get terminal attribute")
        || stderr.contains("could not enable raw mode")
        || stderr.contains("No such device or address")
        || stderr.contains("The handle is invalid")
}

#[cfg(any(unix, windows))]
pub(super) fn wait_for_zellij_session(session_name: &str) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < zellij_session_wait_timeout() {
        match run_zellij(&["list-sessions", "--short", "--no-formatting"]) {
            Ok(sessions) => {
                if sessions.lines().map(str::trim).any(|line| line == session_name)
                    && is_zellij_session_control_ready(session_name)?
                {
                    return Ok(());
                }
            }
            Err(error) if is_transient_zellij_session_wait_error(&error) => {}
            Err(error) => return Err(error),
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(format!(
        "zellij session never appeared within {}ms: {session_name}",
        zellij_session_wait_timeout().as_millis()
    ))
}

#[cfg(any(unix, windows))]
fn zellij_session_wait_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(30) } else { Duration::from_secs(20) }
}

#[cfg(any(unix, windows))]
fn is_transient_zellij_session_wait_error(error: &str) -> bool {
    error.contains("No active zellij sessions found")
        || error.contains("There is no active session")
        || error.contains("Session '") && error.contains("' not found")
}

#[cfg(any(unix, windows))]
fn is_legacy_zellij_action_error(error: &str) -> bool {
    error.contains("The subcommand 'list-tabs' wasn't recognized")
        || error.contains("The subcommand 'list-panes' wasn't recognized")
}

#[cfg(any(unix, windows))]
fn run_zellij_in_session(session_name: &str, args: &[&str]) -> Result<String, String> {
    let mut command_args = Vec::with_capacity(args.len() + 2);
    command_args.push("--session");
    command_args.push(session_name);
    command_args.extend_from_slice(args);

    let output = run_zellij_with_timeout(&command_args, zellij_command_timeout())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid zellij utf8 output: {error}"))
}

#[cfg(any(unix, windows))]
fn is_zellij_session_control_ready(session_name: &str) -> Result<bool, String> {
    match run_zellij_in_session(session_name, &["action", "list-tabs", "--json"]) {
        Ok(output) if output.trim_start().starts_with('[') => {}
        Ok(_) => return Ok(false),
        Err(error) if is_transient_zellij_session_wait_error(&error) => return Ok(false),
        Err(error) if is_legacy_zellij_action_error(&error) => return Ok(true),
        Err(error) => return Err(error),
    }

    match run_zellij_in_session(session_name, &["action", "list-panes", "--json"]) {
        Ok(output) if output.trim_start().starts_with('[') => Ok(true),
        Ok(_) => Ok(false),
        Err(error) if is_transient_zellij_session_wait_error(&error) => Ok(false),
        Err(error) if is_legacy_zellij_action_error(&error) => Ok(true),
        Err(error) => Err(error),
    }
}
pub(super) fn zellij_command_path() -> String {
    if let Some(path) = std::env::var_os("TERMINAL_PLATFORM_ZELLIJ_BIN")
        && !path.as_os_str().is_empty()
    {
        return PathBuf::from(path).display().to_string();
    }

    #[cfg(windows)]
    {
        if let Some(path) = resolve_windows_executable_path("zellij") {
            return path.display().to_string();
        }

        if let Some(path) = workspace_zellij_command_path() {
            return path.display().to_string();
        }
    }

    "zellij".to_string()
}

#[cfg(windows)]
fn workspace_zellij_command_path() -> Option<PathBuf> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()?.parent()?.to_path_buf();
    let candidate = repo_root
        .join("apps")
        .join("terminal-demo")
        .join(".generated")
        .join("tools")
        .join("zellij")
        .join("zellij.exe");
    candidate.is_file().then_some(candidate)
}

#[cfg(windows)]
pub(super) fn windows_powershell_path() -> PathBuf {
    let windows_root = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .unwrap_or_else(|| "C:\\Windows".into());
    PathBuf::from(windows_root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
}
