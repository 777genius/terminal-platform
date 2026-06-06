mod backend_catalog;
mod builder;
mod handshake;
mod registry;
mod sessions;
mod terminal_runtime;

pub use backend_catalog::BackendCatalog;
pub use builder::{TerminalRuntimeBuildError, TerminalRuntimeBuilder};
pub use handshake::{RuntimeCapabilities, RuntimeHandshake, RuntimePhase, RuntimeProtocolVersion};
pub use registry::{InMemorySessionRegistry, SessionDescriptor, SessionRegistry};
pub use terminal_runtime::TerminalRuntime;

#[cfg(test)]
mod tests;
