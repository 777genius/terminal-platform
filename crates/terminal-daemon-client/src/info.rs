use terminal_domain::{ProtocolCompatibility, ProtocolCompatibilityStatus, protocol_compatibility};
use terminal_protocol::{DaemonPhase, Handshake, ProtocolVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonClientInfo {
    pub expected_protocol: ProtocolVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeAssessmentStatus {
    Ready,
    Starting,
    Degraded,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeAssessment {
    pub can_use: bool,
    pub protocol: ProtocolCompatibility,
    pub status: HandshakeAssessmentStatus,
}

impl Default for DaemonClientInfo {
    fn default() -> Self {
        Self { expected_protocol: ProtocolVersion { major: 0, minor: 2 } }
    }
}

impl DaemonClientInfo {
    #[must_use]
    pub fn assess_handshake(&self, handshake: &Handshake) -> HandshakeAssessment {
        let protocol = protocol_compatibility(
            self.expected_protocol.major,
            self.expected_protocol.minor,
            handshake.protocol_version.major,
            handshake.protocol_version.minor,
        );
        let status = match protocol.status {
            ProtocolCompatibilityStatus::Compatible => match handshake.daemon_phase {
                DaemonPhase::Ready => HandshakeAssessmentStatus::Ready,
                DaemonPhase::Starting => HandshakeAssessmentStatus::Starting,
                DaemonPhase::Degraded => HandshakeAssessmentStatus::Degraded,
            },
            ProtocolCompatibilityStatus::ProtocolMajorUnsupported => {
                HandshakeAssessmentStatus::ProtocolMajorUnsupported
            }
            ProtocolCompatibilityStatus::ProtocolMinorAhead => {
                HandshakeAssessmentStatus::ProtocolMinorAhead
            }
        };

        HandshakeAssessment {
            can_use: protocol.can_connect && matches!(handshake.daemon_phase, DaemonPhase::Ready),
            protocol,
            status,
        }
    }
}
