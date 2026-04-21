//! Shared testing helpers and fixtures for daemon transport smoke coverage.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::sync::Arc;
#[cfg(windows)]
use std::{
    io::Read as _,
    sync::{Arc, Mutex},
};
#[cfg(any(unix, windows))]
use std::{
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
#[cfg(unix)]
use terminal_application::BackendCatalog;
#[cfg(unix)]
use terminal_backend_api::MuxBackendPort;
use terminal_backend_api::ShellLaunchSpec;
#[cfg(unix)]
use terminal_backend_native::NativeBackend;
#[cfg(unix)]
use terminal_backend_tmux::TmuxBackend;
#[cfg(unix)]
use terminal_backend_zellij::ZellijBackend;
use terminal_daemon::{
    LocalSocketServerHandle, TerminalDaemon, TerminalDaemonState, spawn_local_socket_server,
};
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_persistence::SqliteSessionStore;
use terminal_protocol::LocalSocketAddress;

#[must_use]
pub fn daemon_state() -> TerminalDaemonState {
    TerminalDaemonState::default()
}

#[must_use]
pub fn isolated_daemon_state(label: &str) -> TerminalDaemonState {
    let store = SqliteSessionStore::open(unique_sqlite_path(label))
        .expect("isolated sqlite session store should open");
    TerminalDaemonState::with_default_persistence(store)
}

pub struct DaemonFixture {
    pub client: LocalSocketDaemonClient,
    server: LocalSocketServerHandle,
}

impl DaemonFixture {
    pub async fn shutdown(self) -> std::io::Result<()> {
        self.server.shutdown().await
    }
}

#[must_use]
pub fn unique_socket_address(label: &str) -> LocalSocketAddress {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let slug = format!("terminal-platform-{label}-{}-{nanos}.sock", std::process::id());

    LocalSocketAddress::from_runtime_slug(slug)
}

#[must_use]
pub fn unique_sqlite_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir()
        .join(format!("terminal-platform-{label}-{}-{nanos}.sqlite3", std::process::id()))
}

pub fn daemon_fixture(label: &str) -> std::io::Result<DaemonFixture> {
    daemon_fixture_with_state(label, TerminalDaemonState::default())
}

pub fn daemon_fixture_with_state(
    label: &str,
    state: TerminalDaemonState,
) -> std::io::Result<DaemonFixture> {
    let address = unique_socket_address(label);
    let server = spawn_local_socket_server(TerminalDaemon::new(state), address.clone())?;
    let client = LocalSocketDaemonClient::new(address);

    Ok(DaemonFixture { client, server })
}

pub async fn wait_for_daemon_ready(client: &LocalSocketDaemonClient) {
    for _ in 0..100 {
        if client.handshake().await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    panic!("daemon fixture never became ready for handshake");
}

#[must_use]
pub fn echo_shell_launch_spec() -> ShellLaunchSpec {
    #[cfg(unix)]
    {
        ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
    }

    #[cfg(windows)]
    {
        let program = std::env::var("COMSPEC")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "cmd.exe".to_string());

        ShellLaunchSpec::new(program).with_args(["/D", "/Q", "/K", "echo ready & more"])
    }
}

#[cfg(unix)]
#[must_use]
pub fn tmux_daemon_state(socket_name: &str) -> TerminalDaemonState {
    TerminalDaemonState::new(BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(TmuxBackend::with_socket_name(socket_name)) as Arc<dyn MuxBackendPort>,
        Arc::new(ZellijBackend) as Arc<dyn MuxBackendPort>,
    ]))
}

#[cfg(unix)]
#[must_use]
pub fn unique_tmux_socket_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("terminal-platform-{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
#[must_use]
pub fn unique_tmux_session_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
#[derive(Debug)]
pub struct TmuxServerGuard {
    socket_name: String,
}

#[cfg(unix)]
impl TmuxServerGuard {
    pub fn spawn(socket_name: &str, session_name: &str) -> Result<Self, String> {
        run_tmux(
            socket_name,
            &[
                "new-session",
                "-d",
                "-s",
                session_name,
                "sh",
                "-lc",
                "printf 'hello from tmux\\n'; exec cat",
            ],
        )?;
        run_tmux(
            socket_name,
            &[
                "new-window",
                "-d",
                "-t",
                session_name,
                "-n",
                "logs",
                "sh",
                "-lc",
                "printf 'logs ready\\n'; exec cat",
            ],
        )?;

        Ok(Self { socket_name: socket_name.to_string() })
    }
}

#[cfg(unix)]
impl Drop for TmuxServerGuard {
    fn drop(&mut self) {
        let _ = run_tmux(&self.socket_name, &["kill-server"]);
    }
}

#[cfg(any(unix, windows))]
#[must_use]
pub fn unique_zellij_session_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let entropy = (nanos & 0xffff_ffff) as u64;
    format!("tp-{}-{:x}", label.chars().take(8).collect::<String>(), entropy)
}

#[cfg(any(unix, windows))]
#[derive(Debug)]
pub struct ZellijSessionGuard {
    session_name: String,
    _lock: ZellijTestLock,
    #[cfg(windows)]
    _pty: Option<WindowsZellijPtyGuard>,
}

#[cfg(any(unix, windows))]
impl ZellijSessionGuard {
    pub fn spawn(session_name: &str) -> Result<Self, String> {
        let lock = ZellijTestLock::acquire()?;
        let _ = run_zellij(&["kill-session", session_name]);

        spawn_zellij_session_with_lock(session_name, lock)
    }
}

#[cfg(unix)]
fn spawn_zellij_session_with_lock(
    session_name: &str,
    lock: ZellijTestLock,
) -> Result<ZellijSessionGuard, String> {
    let mut last_error = None;

    for _ in 0..3 {
        match run_zellij_with_timeout(
            &["attach", "--create-background", session_name],
            zellij_create_timeout(),
        ) {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !is_headless_zellij_spawn_error(&stderr) && !stderr.trim().is_empty() {
                    last_error = Some(stderr.trim().to_string());
                }
            }
            Err(error) => {
                last_error = Some(error);
            }
        }

        let wait_result = wait_for_zellij_session(session_name);
        if wait_result.is_ok() {
            return Ok(ZellijSessionGuard { session_name: session_name.to_string(), _lock: lock });
        }

        let wait_error = wait_result
            .expect_err("wait_result should be an error once zellij session discovery fails");
        if last_error.is_none() {
            last_error = Some(wait_error);
        }

        let _ = run_zellij(&["kill-session", session_name]);
        thread::sleep(Duration::from_millis(200));
    }

    Err(last_error.unwrap_or_else(|| format!("zellij session never stabilized for {session_name}")))
}

#[cfg(windows)]
fn spawn_zellij_session_with_lock(
    session_name: &str,
    lock: ZellijTestLock,
) -> Result<ZellijSessionGuard, String> {
    let mut last_error = None;

    for _ in 0..3 {
        match spawn_windows_zellij_pty(session_name) {
            Ok(pty) => {
                let wait_result = wait_for_zellij_session(session_name);
                if wait_result.is_ok() {
                    return Ok(ZellijSessionGuard {
                        session_name: session_name.to_string(),
                        _lock: lock,
                        _pty: Some(pty),
                    });
                }

                let wait_error = wait_result.expect_err(
                    "wait_result should be an error once zellij session discovery fails",
                );
                last_error = Some(format!("{wait_error}; zellij pty tail: {}", pty.output_tail()));
                drop(pty);
            }
            Err(error) => last_error = Some(error),
        }

        let _ = run_zellij(&["kill-session", session_name]);
        thread::sleep(Duration::from_millis(200));
    }

    for _ in 0..2 {
        match run_zellij_with_timeout(
            &["attach", "--create-background", session_name],
            zellij_create_timeout(),
        ) {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !is_headless_zellij_spawn_error(&stderr) && !stderr.trim().is_empty() {
                    last_error = Some(stderr.trim().to_string());
                }
            }
            Err(error) => last_error = Some(error),
        }

        let wait_result = wait_for_zellij_session(session_name);
        if wait_result.is_ok() {
            return Ok(ZellijSessionGuard {
                session_name: session_name.to_string(),
                _lock: lock,
                _pty: None,
            });
        }

        let wait_error = wait_result
            .expect_err("wait_result should be an error once zellij session discovery fails");
        if last_error.is_none() {
            last_error = Some(wait_error);
        }

        let _ = run_zellij(&["kill-session", session_name]);
        thread::sleep(Duration::from_millis(200));
    }

    Err(last_error.unwrap_or_else(|| format!("zellij session never stabilized for {session_name}")))
}

#[cfg(windows)]
struct WindowsZellijPtyGuard {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _master: Box<dyn portable_pty::MasterPty + Send>,
    output: Arc<Mutex<Vec<u8>>>,
}

#[cfg(windows)]
impl std::fmt::Debug for WindowsZellijPtyGuard {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("WindowsZellijPtyGuard").finish_non_exhaustive()
    }
}

#[cfg(windows)]
impl WindowsZellijPtyGuard {
    fn output_tail(&self) -> String {
        let output = self.output.lock().ok().map_or_else(Vec::new, |buffer| buffer.clone());
        let text = String::from_utf8_lossy(&output);
        let mut lines = text.lines().rev().take(8).collect::<Vec<_>>();
        lines.reverse();
        let tail = lines.join(" | ");
        if tail.trim().is_empty() { "<empty>".to_string() } else { tail }
    }
}

#[cfg(windows)]
impl Drop for WindowsZellijPtyGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(windows)]
fn spawn_windows_zellij_pty(session_name: &str) -> Result<WindowsZellijPtyGuard, String> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|error| format!("failed to open zellij test pty: {error}"))?;

    let mut command = CommandBuilder::new("zellij");
    command.args(["--session", session_name]);
    command.env("TERM", "xterm-256color");

    let child = pty_pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("failed to spawn zellij in test pty: {error}"))?;
    let mut reader = pty_pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("failed to clone zellij test pty reader: {error}"))?;
    let output = Arc::new(Mutex::new(Vec::new()));
    let output_reader = Arc::clone(&output);

    thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    if let Ok(mut output) = output_reader.lock() {
                        output.extend_from_slice(&chunk[..read]);
                        let overflow = output.len().saturating_sub(16 * 1024);
                        if overflow > 0 {
                            output.drain(..overflow);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });

    Ok(WindowsZellijPtyGuard { child, _master: pty_pair.master, output })
}

#[cfg(any(unix, windows))]
impl Drop for ZellijSessionGuard {
    fn drop(&mut self) {
        let _ = run_zellij(&["kill-session", &self.session_name]);
    }
}

#[cfg(unix)]
fn run_tmux(socket_name: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .arg("-L")
        .arg(socket_name)
        .args(args)
        .output()
        .map_err(|error| format!("failed to spawn tmux: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid tmux utf8 output: {error}"))
}

#[cfg(any(unix, windows))]
fn run_zellij(args: &[&str]) -> Result<String, String> {
    let output = run_zellij_with_timeout(args, zellij_command_timeout())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid zellij utf8 output: {error}"))
}

#[cfg(any(unix, windows))]
fn run_zellij_with_timeout(args: &[&str], timeout: Duration) -> Result<Output, String> {
    let mut child = Command::new("zellij")
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
fn zellij_command_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(10) } else { Duration::from_secs(5) }
}

#[cfg(any(unix, windows))]
fn zellij_create_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(20) } else { Duration::from_secs(10) }
}

#[cfg(any(unix, windows))]
fn is_headless_zellij_spawn_error(stderr: &str) -> bool {
    stderr.contains("could not get terminal attribute")
        || stderr.contains("could not enable raw mode")
        || stderr.contains("No such device or address")
        || stderr.contains("The handle is invalid")
}

#[cfg(any(unix, windows))]
fn wait_for_zellij_session(session_name: &str) -> Result<(), String> {
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
    if cfg!(windows) { Duration::from_secs(45) } else { Duration::from_secs(20) }
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

#[cfg(any(unix, windows))]
#[derive(Debug)]
pub struct ZellijTestLock {
    path: PathBuf,
}

#[cfg(any(unix, windows))]
impl ZellijTestLock {
    pub fn acquire() -> Result<Self, String> {
        let path = std::env::temp_dir().join("terminal-platform-zellij-test.lock");
        for _ in 0..9000 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    writeln!(file, "pid={}", std::process::id())
                        .map_err(|error| format!("failed to write zellij test lock: {error}"))?;
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if clear_stale_zellij_test_lock(&path) {
                        continue;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(format!("failed to acquire zellij test lock: {error}")),
            }
        }

        Err(format!("timed out acquiring zellij test lock at {}", path.display()))
    }
}

#[cfg(any(unix, windows))]
impl Drop for ZellijTestLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(any(unix, windows))]
fn clear_stale_zellij_test_lock(path: &PathBuf) -> bool {
    let pid = fs::read_to_string(path).ok().and_then(parse_zellij_test_lock_pid);
    if let Some(pid) = pid {
        if pid == std::process::id() || !is_zellij_test_lock_pid_alive(pid) {
            return fs::remove_file(path).is_ok();
        }
        return false;
    }

    false
}

#[cfg(any(unix, windows))]
fn parse_zellij_test_lock_pid(contents: String) -> Option<u32> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|value| value.trim().parse::<u32>().ok())
}

#[cfg(unix)]
fn is_zellij_test_lock_pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(true)
}

#[cfg(windows)]
fn is_zellij_test_lock_pid_alive(pid: u32) -> bool {
    Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!(
                "if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 0 }} else {{ exit 1 }}"
            ),
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(true)
}
