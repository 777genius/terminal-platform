use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    thread,
    time::Duration,
};

use std::process::Command;

#[cfg(windows)]
use super::command::windows_powershell_path;

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
    Command::new(windows_powershell_path())
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
