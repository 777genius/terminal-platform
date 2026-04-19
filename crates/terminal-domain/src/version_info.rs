use serde::{Deserialize, Serialize};

pub const CURRENT_BINARY_VERSION: &str = "0.1.0-dev";
pub const CURRENT_PROTOCOL_MAJOR: u16 = 0;
pub const CURRENT_PROTOCOL_MINOR: u16 = 1;
pub const CURRENT_SAVED_SESSION_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionManifest {
    pub format_version: u32,
    pub binary_version: String,
    pub protocol_major: u16,
    pub protocol_minor: u16,
}

impl SavedSessionManifest {
    #[must_use]
    pub fn current() -> Self {
        Self {
            format_version: CURRENT_SAVED_SESSION_FORMAT_VERSION,
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            protocol_major: CURRENT_PROTOCOL_MAJOR,
            protocol_minor: CURRENT_PROTOCOL_MINOR,
        }
    }
}
