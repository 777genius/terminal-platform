//! `Zellij` foreign adapter implementation will live here.

use terminal_domain::BackendKind;

#[derive(Debug, Default)]
pub struct ZellijBackend;

impl ZellijBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }
}
