mod command;
mod lock;
#[cfg(windows)]
mod windows;

use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use command::{is_headless_zellij_spawn_error, run_zellij_with_timeout, zellij_create_timeout};
use command::{run_zellij, wait_for_zellij_session};
pub use lock::ZellijTestLock;
#[cfg(windows)]
use windows::{WindowsZellijPtyGuard, spawn_windows_zellij_pty};

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

    for _ in 0..1 {
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

    Err(last_error.unwrap_or_else(|| format!("zellij session never stabilized for {session_name}")))
}

#[cfg(any(unix, windows))]
impl Drop for ZellijSessionGuard {
    fn drop(&mut self) {
        let _ = run_zellij(&["kill-session", &self.session_name]);
    }
}
