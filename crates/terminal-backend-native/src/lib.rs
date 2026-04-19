//! Native backend implementation will live here.

use terminal_domain::BackendKind;

#[derive(Debug, Default)]
pub struct NativeBackend;

impl NativeBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Native
    }
}
