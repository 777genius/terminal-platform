use std::sync::Arc;

use terminal_backend_api::MuxBackendPort;
use terminal_domain::BackendKind;

pub trait TerminalDaemonBackendProvider: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn build_backend(&self) -> Arc<dyn MuxBackendPort>;
}

#[cfg(feature = "native-backend")]
pub(super) struct NativeBackendProvider;

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
pub(super) struct TmuxBackendProvider;

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
pub(super) struct ZellijBackendProvider;

#[cfg(feature = "zellij-backend")]
impl TerminalDaemonBackendProvider for ZellijBackendProvider {
    fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }

    fn build_backend(&self) -> Arc<dyn MuxBackendPort> {
        Arc::new(terminal_backend_zellij::ZellijBackend) as Arc<dyn MuxBackendPort>
    }
}
