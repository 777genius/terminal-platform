use terminal_domain::BackendKind;

use super::{TerminalDaemonBootstrapConfig, TerminalDaemonBootstrapConfigError};
use crate::backend_registry::compiled_backend_kinds;

#[test]
fn default_backends_track_compiled_backends() {
    let config = TerminalDaemonBootstrapConfig::default();

    assert_eq!(config.enabled_backends, compiled_backend_kinds());
}

#[test]
fn parses_backend_csv_into_sorted_unique_backends() {
    let config = TerminalDaemonBootstrapConfig::from_backend_csv("zellij,native,zellij")
        .expect("backend csv should parse");

    assert_eq!(config.enabled_backends, vec![BackendKind::Native, BackendKind::Zellij]);
}

#[test]
fn rejects_unknown_backend_names() {
    let error = TerminalDaemonBootstrapConfig::from_backend_csv("native,screen")
        .expect_err("unknown backend name should fail");

    assert_eq!(
        error,
        TerminalDaemonBootstrapConfigError::UnknownBackend { value: "screen".to_string() }
    );
}

#[test]
fn converts_enabled_backends_into_backend_config() {
    let config = TerminalDaemonBootstrapConfig::from_backend_csv("native,zellij")
        .expect("backend csv should parse");
    let backend_config = config.backend_config();

    assert!(backend_config.native);
    assert!(!backend_config.tmux);
    assert!(backend_config.zellij);
}
