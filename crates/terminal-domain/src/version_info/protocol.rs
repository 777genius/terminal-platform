use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolCompatibilityStatus {
    Compatible,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolCompatibility {
    pub can_connect: bool,
    pub status: ProtocolCompatibilityStatus,
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
