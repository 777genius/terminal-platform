use terminal_domain::BackendKind;

use crate::bootstrap::errors::TerminalDaemonBootstrapConfigError;

pub(super) fn normalize_backends(mut backends: Vec<BackendKind>) -> Vec<BackendKind> {
    backends.sort_by_key(backend_sort_order);
    backends.dedup();
    backends
}

pub(super) fn parse_backend_kind(
    value: &str,
) -> Result<BackendKind, TerminalDaemonBootstrapConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "native" => Ok(BackendKind::Native),
        "tmux" => Ok(BackendKind::Tmux),
        "zellij" => Ok(BackendKind::Zellij),
        _ => Err(TerminalDaemonBootstrapConfigError::UnknownBackend {
            value: value.trim().to_string(),
        }),
    }
}

fn backend_sort_order(kind: &BackendKind) -> u8 {
    match kind {
        BackendKind::Native => 0,
        BackendKind::Tmux => 1,
        BackendKind::Zellij => 2,
    }
}
