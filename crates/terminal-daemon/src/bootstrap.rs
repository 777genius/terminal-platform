use std::env;

use serde::{Deserialize, Serialize};
use terminal_domain::BackendKind;
use terminal_runtime::{TerminalRuntime, TerminalRuntimeBuildError};
use thiserror::Error;

use crate::{
    TerminalDaemon,
    backend_registry::{
        TerminalDaemonBackendBuildError, TerminalDaemonBackendConfig,
        TerminalDaemonBackendRegistry, compiled_backend_kinds,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalDaemonBootstrapConfig {
    #[serde(default = "compiled_backend_kinds")]
    pub enabled_backends: Vec<BackendKind>,
}

impl Default for TerminalDaemonBootstrapConfig {
    fn default() -> Self {
        Self { enabled_backends: compiled_backend_kinds() }
    }
}

impl TerminalDaemonBootstrapConfig {
    pub const BACKENDS_ENV: &str = "TERMINAL_DAEMON_BACKENDS";

    pub fn from_env() -> Result<Self, TerminalDaemonBootstrapConfigError> {
        match env::var(Self::BACKENDS_ENV) {
            Ok(value) => Self::from_backend_csv(&value),
            Err(env::VarError::NotPresent) => Ok(Self::default()),
            Err(env::VarError::NotUnicode(_)) => {
                Err(TerminalDaemonBootstrapConfigError::InvalidEnvironmentEncoding {
                    env_var: Self::BACKENDS_ENV,
                })
            }
        }
    }

    pub fn from_backend_csv(value: &str) -> Result<Self, TerminalDaemonBootstrapConfigError> {
        let mut enabled_backends = Vec::new();

        for candidate in value.split(',').map(str::trim).filter(|candidate| !candidate.is_empty()) {
            enabled_backends.push(parse_backend_kind(candidate)?);
        }

        if enabled_backends.is_empty() {
            return Err(TerminalDaemonBootstrapConfigError::EmptyBackendList);
        }

        Ok(Self { enabled_backends: normalize_backends(enabled_backends) })
    }

    #[must_use]
    pub fn enable_backend(mut self, backend: BackendKind, enabled: bool) -> Self {
        self.enabled_backends.retain(|candidate| *candidate != backend);
        if enabled {
            self.enabled_backends.push(backend);
        }
        self.enabled_backends = normalize_backends(self.enabled_backends);
        self
    }

    #[must_use]
    pub fn backend_config(&self) -> TerminalDaemonBackendConfig {
        self.enabled_backends
            .iter()
            .copied()
            .fold(TerminalDaemonBackendConfig::none(), |config, backend| {
                config.enable(backend, true)
            })
    }

    pub fn build_runtime(&self) -> Result<TerminalRuntime, TerminalDaemonBootstrapBuildError> {
        let catalog = TerminalDaemonBackendRegistry::compiled_default()
            .build_catalog(self.backend_config())
            .map_err(TerminalDaemonBootstrapBuildError::Backend)?;

        TerminalRuntime::builder()
            .with_backends(catalog)
            .with_default_persistence()
            .map_err(TerminalDaemonBootstrapBuildError::Persistence)?
            .build()
            .map_err(TerminalDaemonBootstrapBuildError::Runtime)
    }

    pub fn build_daemon(&self) -> Result<TerminalDaemon, TerminalDaemonBootstrapBuildError> {
        self.build_runtime().map(TerminalDaemon::new)
    }
}

impl TerminalDaemon {
    pub fn from_bootstrap(
        config: &TerminalDaemonBootstrapConfig,
    ) -> Result<Self, TerminalDaemonBootstrapBuildError> {
        config.build_daemon()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalDaemonBootstrapConfigError {
    #[error("terminal-daemon bootstrap config listed no backends")]
    EmptyBackendList,
    #[error("terminal-daemon bootstrap config backend '{value}' is not recognized")]
    UnknownBackend { value: String },
    #[error("terminal-daemon bootstrap env var {env_var} contains non-utf8 data")]
    InvalidEnvironmentEncoding { env_var: &'static str },
}

#[derive(Debug, Error)]
pub enum TerminalDaemonBootstrapBuildError {
    #[error(transparent)]
    Backend(#[from] TerminalDaemonBackendBuildError),
    #[error(transparent)]
    Persistence(#[from] terminal_persistence::PersistenceError),
    #[error(transparent)]
    Runtime(#[from] TerminalRuntimeBuildError),
}

fn normalize_backends(mut backends: Vec<BackendKind>) -> Vec<BackendKind> {
    backends.sort_by_key(|kind| match kind {
        BackendKind::Native => 0,
        BackendKind::Tmux => 1,
        BackendKind::Zellij => 2,
    });
    backends.dedup();
    backends
}

fn parse_backend_kind(value: &str) -> Result<BackendKind, TerminalDaemonBootstrapConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "native" => Ok(BackendKind::Native),
        "tmux" => Ok(BackendKind::Tmux),
        "zellij" => Ok(BackendKind::Zellij),
        _ => Err(TerminalDaemonBootstrapConfigError::UnknownBackend {
            value: value.trim().to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use terminal_domain::BackendKind;

    use super::{
        TerminalDaemonBootstrapConfig, TerminalDaemonBootstrapConfigError, compiled_backend_kinds,
    };

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
}
