mod client_info;
mod constructors;
mod ffi_types;
mod handles;
mod input;
mod memory;
mod prelude;
mod saved_sessions;
mod session_lifecycle;
mod subscriptions;
mod surfaces;

pub use client_info::*;
pub use constructors::*;
pub use ffi_types::{
    TerminalCapiClientResult, TerminalCapiStatus, TerminalCapiStringResult,
    TerminalCapiSubscriptionResult,
};
pub use memory::*;
pub use saved_sessions::*;
pub use session_lifecycle::*;
pub use subscriptions::*;
pub use surfaces::*;

#[cfg(test)]
mod tests;
