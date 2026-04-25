use std::{collections::HashMap, fmt, sync::Arc};

use terminal_backend_api::MuxBackendPort;
use terminal_domain::BackendKind;
use terminal_runtime::BackendCatalog;
use thiserror::Error;

pub trait TerminalDaemonBackendProvider: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn build_backend(&self) -> Arc<dyn MuxBackendPort>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalDaemonBackendConfig {
    pub native: bool,
    pub tmux: bool,
    pub zellij: bool,
}

impl TerminalDaemonBackendConfig {
    #[must_use]
    pub fn compiled_defaults() -> Self {
        Self {
            native: cfg!(feature = "native-backend"),
            tmux: cfg!(feature = "tmux-backend"),
            zellij: cfg!(feature = "zellij-backend"),
        }
    }

    #[must_use]
    pub const fn none() -> Self {
        Self { native: false, tmux: false, zellij: false }
    }

    #[must_use]
    pub const fn enable(mut self, backend: BackendKind, enabled: bool) -> Self {
        match backend {
            BackendKind::Native => self.native = enabled,
            BackendKind::Tmux => self.tmux = enabled,
            BackendKind::Zellij => self.zellij = enabled,
        }
        self
    }

    #[must_use]
    pub const fn is_enabled(&self, backend: BackendKind) -> bool {
        match backend {
            BackendKind::Native => self.native,
            BackendKind::Tmux => self.tmux,
            BackendKind::Zellij => self.zellij,
        }
    }

    #[must_use]
    pub fn enabled_backends(&self) -> Vec<BackendKind> {
        sort_backends(
            [BackendKind::Native, BackendKind::Tmux, BackendKind::Zellij]
                .into_iter()
                .filter(|backend| self.is_enabled(*backend))
                .collect(),
        )
    }
}

impl Default for TerminalDaemonBackendConfig {
    fn default() -> Self {
        Self::compiled_defaults()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalDaemonBackendBuildError {
    #[error("terminal-daemon backend config enables no backends")]
    NoBackendsEnabled,
    #[error(
        "terminal-daemon backend {backend:?} was requested but is not compiled in. Compiled backends - {compiled_backends:?}"
    )]
    BackendNotCompiled { backend: BackendKind, compiled_backends: Vec<BackendKind> },
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

#[must_use]
pub fn compiled_backend_kinds() -> Vec<BackendKind> {
    TerminalDaemonBackendRegistry::compiled_default().compiled_backends()
}

fn sort_backends(mut backends: Vec<BackendKind>) -> Vec<BackendKind> {
    backends.sort_by_key(|kind| match kind {
        BackendKind::Native => 0,
        BackendKind::Tmux => 1,
        BackendKind::Zellij => 2,
    });
    backends
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
