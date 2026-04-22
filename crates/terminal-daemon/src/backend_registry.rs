use std::{collections::HashMap, fmt, sync::Arc};

use terminal_application::BackendCatalog;
use terminal_backend_api::MuxBackendPort;
use terminal_domain::BackendKind;

use crate::state::{TerminalDaemonBackendConfig, TerminalDaemonStateBuildError};

pub trait TerminalDaemonBackendProvider: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn build_backend(&self) -> Arc<dyn MuxBackendPort>;
}

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
        registry.add_provider(Arc::new(NativeBackendProvider));
        #[cfg(feature = "tmux-backend")]
        registry.add_provider(Arc::new(TmuxBackendProvider));
        #[cfg(feature = "zellij-backend")]
        registry.add_provider(Arc::new(ZellijBackendProvider));

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
        let mut backends = self.providers.keys().copied().collect::<Vec<_>>();
        backends.sort_by_key(|kind| match kind {
            BackendKind::Native => 0,
            BackendKind::Tmux => 1,
            BackendKind::Zellij => 2,
        });
        backends
    }

    pub fn build_catalog(
        &self,
        backend_config: TerminalDaemonBackendConfig,
    ) -> Result<BackendCatalog, TerminalDaemonStateBuildError> {
        let compiled_backends = self.compiled_backends();
        let enabled_backends = backend_config.enabled_backends();

        if enabled_backends.is_empty() {
            return Err(TerminalDaemonStateBuildError::NoBackendsEnabled);
        }

        let mut backends = Vec::with_capacity(enabled_backends.len());
        for backend in enabled_backends {
            let provider = self.providers.get(&backend).ok_or_else(|| {
                TerminalDaemonStateBuildError::BackendNotCompiled {
                    backend,
                    compiled_backends: compiled_backends.clone(),
                }
            })?;
            backends.push(provider.build_backend());
        }

        Ok(BackendCatalog::new(backends))
    }
}

#[cfg(feature = "native-backend")]
struct NativeBackendProvider;

#[cfg(feature = "native-backend")]
impl TerminalDaemonBackendProvider for NativeBackendProvider {
    fn kind(&self) -> BackendKind {
        BackendKind::Native
    }

    fn build_backend(&self) -> Arc<dyn MuxBackendPort> {
        Arc::new(terminal_backend_native::NativeBackend::default()) as Arc<dyn MuxBackendPort>
    }
}

#[cfg(feature = "tmux-backend")]
struct TmuxBackendProvider;

#[cfg(feature = "tmux-backend")]
impl TerminalDaemonBackendProvider for TmuxBackendProvider {
    fn kind(&self) -> BackendKind {
        BackendKind::Tmux
    }

    fn build_backend(&self) -> Arc<dyn MuxBackendPort> {
        Arc::new(terminal_backend_tmux::TmuxBackend::default()) as Arc<dyn MuxBackendPort>
    }
}

#[cfg(feature = "zellij-backend")]
struct ZellijBackendProvider;

#[cfg(feature = "zellij-backend")]
impl TerminalDaemonBackendProvider for ZellijBackendProvider {
    fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }

    fn build_backend(&self) -> Arc<dyn MuxBackendPort> {
        Arc::new(terminal_backend_zellij::ZellijBackend) as Arc<dyn MuxBackendPort>
    }
}
