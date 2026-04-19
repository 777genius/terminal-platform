//! `tmux` foreign adapter implementation will live here.

use terminal_domain::BackendKind;

#[derive(Debug, Default)]
pub struct TmuxBackend;

impl TmuxBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Tmux
    }
}
