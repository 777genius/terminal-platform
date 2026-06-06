use terminal_runtime::TerminalRuntime;

use crate::{
    TerminalDaemon,
    backend_registry::TerminalDaemonBackendRegistry,
    bootstrap::{config::TerminalDaemonBootstrapConfig, errors::TerminalDaemonBootstrapBuildError},
};

impl TerminalDaemonBootstrapConfig {
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
