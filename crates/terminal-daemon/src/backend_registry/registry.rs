use std::{collections::HashMap, fmt, sync::Arc};

use terminal_domain::BackendKind;
use terminal_runtime::BackendCatalog;

use super::{
    config::TerminalDaemonBackendConfig, error::TerminalDaemonBackendBuildError,
    ordering::sort_backends, providers::TerminalDaemonBackendProvider,
};

#[derive(Clone, Default)]
pub struct TerminalDaemonBackendRegistry {
    providers: HashMap<BackendKind, Arc<dyn TerminalDaemonBackendProvider>>,
}

impl fmt::Debug for TerminalDaemonBackendRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalDaemonBackendRegistry")
            .field("compiled_backends", &self.compiled_backends())
            .finish()
    }
}

impl TerminalDaemonBackendRegistry {
    #[must_use]
    pub fn compiled_default() -> Self {
        let mut registry = Self::default();

        #[cfg(feature = "native-backend")]
        registry.add_provider(Arc::new(super::providers::NativeBackendProvider));
        #[cfg(feature = "tmux-backend")]
        registry.add_provider(Arc::new(super::providers::TmuxBackendProvider));
        #[cfg(feature = "zellij-backend")]
        registry.add_provider(Arc::new(super::providers::ZellijBackendProvider));

        registry
    }

    #[must_use]
    pub fn with_provider(mut self, provider: Arc<dyn TerminalDaemonBackendProvider>) -> Self {
        self.add_provider(provider);
        self
    }

    pub fn add_provider(&mut self, provider: Arc<dyn TerminalDaemonBackendProvider>) {
        self.providers.insert(provider.kind(), provider);
    }

    #[must_use]
    pub fn compiled_backends(&self) -> Vec<BackendKind> {
        sort_backends(self.providers.keys().copied().collect())
    }

    pub fn build_catalog(
        &self,
        backend_config: TerminalDaemonBackendConfig,
    ) -> Result<BackendCatalog, TerminalDaemonBackendBuildError> {
        let compiled_backends = self.compiled_backends();
        let enabled_backends = backend_config.enabled_backends();

        if enabled_backends.is_empty() {
            return Err(TerminalDaemonBackendBuildError::NoBackendsEnabled);
        }

        let mut backends = Vec::with_capacity(enabled_backends.len());
        for backend in enabled_backends {
            let provider = self.providers.get(&backend).ok_or_else(|| {
                TerminalDaemonBackendBuildError::BackendNotCompiled {
                    backend,
                    compiled_backends: compiled_backends.clone(),
                }
            })?;
            backends.push(provider.build_backend());
        }

        Ok(BackendCatalog::new(backends))
    }
}
