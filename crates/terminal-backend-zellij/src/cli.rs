use std::path::PathBuf;

use terminal_backend_api::BackendError;
use terminal_domain::DegradedModeReason;

pub(crate) fn is_transient_zellij_error(message: &str) -> bool {
    message.contains("No active zellij sessions found")
        || message.contains("There is no active session")
        || message.contains("Session '") && message.contains("' not found")
}

pub(crate) fn is_transient_zellij_backend_error(error: &BackendError) -> bool {
    is_transient_zellij_error(&error.message)
        || error.message.contains("invalid zellij list-tabs json: EOF while parsing a value")
        || error.message.contains("invalid zellij list-tabs json: expected value")
        || error.message.contains("invalid zellij list-panes json: EOF while parsing a value")
        || error.message.contains("invalid zellij list-panes json: expected value")
        || error.message.contains("invalid zellij list-panes json: missing field")
        || error
            .message
            .contains("unexpected zellij list-tabs payload while the session was settling")
        || error
            .message
            .contains("unexpected zellij list-panes payload while the session was settling")
        || error.message.contains(
            "zellij snapshot commands returned empty output while the session was still settling",
        )
        || error.message.contains("exposed no importable panes")
}

pub(crate) fn zellij_focus_actions_supported() -> bool {
    !cfg!(windows)
}

pub(crate) fn zellij_focus_unsupported_error() -> BackendError {
    BackendError::unsupported(
        "zellij imported routes cannot focus tabs or panes on Windows because CLI focus actions are scoped to the transient action client",
        DegradedModeReason::UnsupportedByBackend,
    )
}

pub(crate) fn zellij_command_path() -> PathBuf {
    if let Some(path) = non_empty_env_path("TERMINAL_PLATFORM_ZELLIJ_BIN") {
        return path;
    }

    #[cfg(windows)]
    {
        if let Some(path) = resolve_windows_executable("zellij") {
            return path;
        }

        if let Some(path) = workspace_zellij_command_path() {
            return path;
        }
    }

    PathBuf::from("zellij")
}

fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).and_then(|value| {
        if value.as_os_str().is_empty() { None } else { Some(PathBuf::from(value)) }
    })
}

#[cfg(windows)]
fn resolve_windows_executable(program: &str) -> Option<PathBuf> {
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

    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            candidates.iter().map(|candidate| dir.join(candidate)).find(|path| path.is_file())
        })
    })
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
