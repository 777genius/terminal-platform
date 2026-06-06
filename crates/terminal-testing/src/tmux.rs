#[cfg(all(unix, feature = "tmux-backend"))]
use std::sync::Arc;
#[cfg(unix)]
use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(all(unix, feature = "tmux-backend"))]
use terminal_backend_api::MuxBackendPort;
#[cfg(all(unix, feature = "tmux-backend", feature = "native-backend"))]
use terminal_backend_native::NativeBackend;
#[cfg(all(unix, feature = "tmux-backend"))]
use terminal_backend_tmux::TmuxBackend;
#[cfg(all(unix, feature = "tmux-backend", feature = "zellij-backend"))]
use terminal_backend_zellij::ZellijBackend;
#[cfg(all(unix, feature = "tmux-backend"))]
use terminal_daemon::TerminalDaemon;
#[cfg(all(unix, feature = "tmux-backend"))]
use terminal_runtime::{BackendCatalog, TerminalRuntime};

#[cfg(all(unix, feature = "tmux-backend"))]
pub fn tmux_daemon(socket_name: &str) -> TerminalDaemon {
    let mut backends = Vec::<Arc<dyn MuxBackendPort>>::new();

    #[cfg(feature = "native-backend")]
    backends.push(Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>);

    backends.push(Arc::new(TmuxBackend::with_socket_name(socket_name)) as Arc<dyn MuxBackendPort>);

    #[cfg(feature = "zellij-backend")]
    backends.push(Arc::new(ZellijBackend) as Arc<dyn MuxBackendPort>);

    TerminalDaemon::new(TerminalRuntime::new(BackendCatalog::new(backends)))
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
