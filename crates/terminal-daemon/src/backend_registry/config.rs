use terminal_domain::BackendKind;

use super::ordering::sort_backends;

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
