mod active_sessions;
mod catalog;
mod constructors;
mod handshake;
mod saved_sessions;
mod subscriptions;

use crate::sessions::SessionService;

pub struct TerminalRuntime {
    sessions: SessionService,
}
