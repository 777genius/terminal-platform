use serde::{Deserialize, Serialize};

use super::constants::{
    CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR,
    CURRENT_SAVED_SESSION_FORMAT_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedSessionCompatibilityStatus {
    Compatible,
    BinarySkew,
    FormatVersionUnsupported,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionManifest {
    pub format_version: u32,
    pub binary_version: String,
    pub protocol_major: u16,
    pub protocol_minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionCompatibility {
    pub can_restore: bool,
    pub status: SavedSessionCompatibilityStatus,
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

#[must_use]
pub fn saved_session_compatibility(manifest: &SavedSessionManifest) -> SavedSessionCompatibility {
    let status = saved_session_compatibility_status(manifest);
    let can_restore = matches!(
        status,
        SavedSessionCompatibilityStatus::Compatible | SavedSessionCompatibilityStatus::BinarySkew
    );

    SavedSessionCompatibility { can_restore, status }
}

fn saved_session_compatibility_status(
    manifest: &SavedSessionManifest,
) -> SavedSessionCompatibilityStatus {
    if manifest.format_version != CURRENT_SAVED_SESSION_FORMAT_VERSION {
        SavedSessionCompatibilityStatus::FormatVersionUnsupported
    } else if manifest.protocol_major != CURRENT_PROTOCOL_MAJOR {
        SavedSessionCompatibilityStatus::ProtocolMajorUnsupported
    } else if manifest.protocol_minor > CURRENT_PROTOCOL_MINOR {
        SavedSessionCompatibilityStatus::ProtocolMinorAhead
    } else if manifest.binary_version != CURRENT_BINARY_VERSION {
        SavedSessionCompatibilityStatus::BinarySkew
    } else {
        SavedSessionCompatibilityStatus::Compatible
    }
}
