mod active_sessions;
mod catalog;
mod mappings;
mod saved_sessions;
mod subscriptions;

use terminal_runtime::TerminalRuntime;

#[derive(Clone, Copy)]
pub struct TerminalRuntimeAdapter<'a> {
    pub(super) runtime: &'a TerminalRuntime,
}

impl<'a> TerminalRuntimeAdapter<'a> {
    #[must_use]
    pub fn new(runtime: &'a TerminalRuntime) -> Self {
        Self { runtime }
    }
}
