use serde::{Deserialize, Serialize};
use terminal_domain::BackendKind;

use crate::{
    backend_registry::{TerminalDaemonBackendConfig, compiled_backend_kinds},
    bootstrap::parser::normalize_backends,
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
}
