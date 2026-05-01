mod attach;
mod capabilities;
mod command_runner;
mod discovery;
mod port;

use crate::prelude::BackendKind;

#[derive(Debug, Clone, Default)]
pub struct TmuxBackend {
    socket_name: Option<String>,
}

impl TmuxBackend {
    #[must_use]
    pub fn with_socket_name(socket_name: impl Into<String>) -> Self {
        Self { socket_name: Some(socket_name.into()) }
    }

    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Tmux
    }
}
