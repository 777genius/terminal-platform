use std::{
    io::{Read as _, Write},
    sync::{Arc, Mutex},
    thread,
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use super::command::zellij_command_path;

pub(super) struct WindowsZellijPtyGuard {
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
    pub(super) fn output_tail(&self) -> String {
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
pub(super) fn spawn_windows_zellij_pty(
    session_name: &str,
) -> Result<WindowsZellijPtyGuard, String> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|error| format!("failed to open zellij test pty: {error}"))?;

    let mut command = CommandBuilder::new(zellij_command_path());
    command.args(["attach", "--create", session_name]);
    command.env("TERM", "xterm-256color");

    let child = pty_pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("failed to spawn zellij in test pty: {error}"))?;
    let mut reader = pty_pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("failed to clone zellij test pty reader: {error}"))?;
    let mut writer = pty_pair
        .master
        .take_writer()
        .map_err(|error| format!("failed to open zellij test pty writer: {error}"))?;
    let output = Arc::new(Mutex::new(Vec::new()));
    let output_reader = Arc::clone(&output);

    thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    if chunk[..read].windows(4).any(|window| window == b"\x1b[6n") {
                        let _ = writer.write_all(b"\x1b[1;1R");
                        let _ = writer.flush();
                    }
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
