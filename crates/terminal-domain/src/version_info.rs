mod constants;
mod protocol;
mod saved_sessions;

pub use constants::{
    CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR,
    CURRENT_SAVED_SESSION_FORMAT_VERSION,
};
pub use protocol::{ProtocolCompatibility, ProtocolCompatibilityStatus, protocol_compatibility};
pub use saved_sessions::{
    SavedSessionCompatibility, SavedSessionCompatibilityStatus, SavedSessionManifest,
    saved_session_compatibility,
};

#[cfg(test)]
mod tests;
