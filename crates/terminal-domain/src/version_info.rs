use serde::{Deserialize, Serialize};

pub const CURRENT_BINARY_VERSION: &str = "0.1.0-dev";
pub const CURRENT_PROTOCOL_MAJOR: u16 = 0;
pub const CURRENT_PROTOCOL_MINOR: u16 = 2;
pub const CURRENT_SAVED_SESSION_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolCompatibilityStatus {
    Compatible,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolCompatibility {
    pub can_connect: bool,
    pub status: ProtocolCompatibilityStatus,
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
pub fn protocol_compatibility(
    expected_protocol_major: u16,
    expected_protocol_minor: u16,
    actual_protocol_major: u16,
    actual_protocol_minor: u16,
) -> ProtocolCompatibility {
    let status = if actual_protocol_major != expected_protocol_major {
        ProtocolCompatibilityStatus::ProtocolMajorUnsupported
    } else if actual_protocol_minor > expected_protocol_minor {
        ProtocolCompatibilityStatus::ProtocolMinorAhead
    } else {
        ProtocolCompatibilityStatus::Compatible
    };

    ProtocolCompatibility {
        can_connect: matches!(status, ProtocolCompatibilityStatus::Compatible),
        status,
    }
}

#[must_use]
pub fn saved_session_compatibility(manifest: &SavedSessionManifest) -> SavedSessionCompatibility {
    let status = if manifest.format_version != CURRENT_SAVED_SESSION_FORMAT_VERSION {
        SavedSessionCompatibilityStatus::FormatVersionUnsupported
    } else if manifest.protocol_major != CURRENT_PROTOCOL_MAJOR {
        SavedSessionCompatibilityStatus::ProtocolMajorUnsupported
    } else if manifest.protocol_minor > CURRENT_PROTOCOL_MINOR {
        SavedSessionCompatibilityStatus::ProtocolMinorAhead
    } else if manifest.binary_version != CURRENT_BINARY_VERSION {
        SavedSessionCompatibilityStatus::BinarySkew
    } else {
        SavedSessionCompatibilityStatus::Compatible
    };

    let can_restore = matches!(
        status,
        SavedSessionCompatibilityStatus::Compatible | SavedSessionCompatibilityStatus::BinarySkew
    );

    SavedSessionCompatibility { can_restore, status }
}

#[cfg(test)]
mod tests {
    use super::{
        CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR,
        CURRENT_SAVED_SESSION_FORMAT_VERSION, ProtocolCompatibilityStatus,
        SavedSessionCompatibilityStatus, SavedSessionManifest, protocol_compatibility,
        saved_session_compatibility,
    };

    #[test]
    fn marks_current_protocol_as_compatible() {
        let compatibility = protocol_compatibility(
            CURRENT_PROTOCOL_MAJOR,
            CURRENT_PROTOCOL_MINOR,
            CURRENT_PROTOCOL_MAJOR,
            CURRENT_PROTOCOL_MINOR,
        );

        assert!(compatibility.can_connect);
        assert_eq!(compatibility.status, ProtocolCompatibilityStatus::Compatible);
    }

    #[test]
    fn rejects_future_protocol_minor_for_connections() {
        let compatibility = protocol_compatibility(
            CURRENT_PROTOCOL_MAJOR,
            CURRENT_PROTOCOL_MINOR,
            CURRENT_PROTOCOL_MAJOR,
            CURRENT_PROTOCOL_MINOR + 1,
        );

        assert!(!compatibility.can_connect);
        assert_eq!(compatibility.status, ProtocolCompatibilityStatus::ProtocolMinorAhead);
    }

    #[test]
    fn rejects_protocol_major_mismatch_for_connections() {
        let compatibility = protocol_compatibility(
            CURRENT_PROTOCOL_MAJOR,
            CURRENT_PROTOCOL_MINOR,
            CURRENT_PROTOCOL_MAJOR + 1,
            CURRENT_PROTOCOL_MINOR,
        );

        assert!(!compatibility.can_connect);
        assert_eq!(compatibility.status, ProtocolCompatibilityStatus::ProtocolMajorUnsupported);
    }

    #[test]
    fn marks_current_manifest_as_compatible() {
        let compatibility = saved_session_compatibility(&SavedSessionManifest::current());

        assert!(compatibility.can_restore);
        assert_eq!(compatibility.status, SavedSessionCompatibilityStatus::Compatible);
    }

    #[test]
    fn marks_binary_skew_as_restoreable() {
        let compatibility = saved_session_compatibility(&SavedSessionManifest {
            format_version: CURRENT_SAVED_SESSION_FORMAT_VERSION,
            binary_version: "0.2.0-dev".to_string(),
            protocol_major: CURRENT_PROTOCOL_MAJOR,
            protocol_minor: CURRENT_PROTOCOL_MINOR,
        });

        assert!(compatibility.can_restore);
        assert_eq!(compatibility.status, SavedSessionCompatibilityStatus::BinarySkew);
    }

    #[test]
    fn rejects_future_protocol_minor() {
        let compatibility = saved_session_compatibility(&SavedSessionManifest {
            format_version: CURRENT_SAVED_SESSION_FORMAT_VERSION,
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            protocol_major: CURRENT_PROTOCOL_MAJOR,
            protocol_minor: CURRENT_PROTOCOL_MINOR + 1,
        });

        assert!(!compatibility.can_restore);
        assert_eq!(compatibility.status, SavedSessionCompatibilityStatus::ProtocolMinorAhead);
    }
}
