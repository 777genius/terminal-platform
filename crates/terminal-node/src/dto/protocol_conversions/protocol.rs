use crate::dto::{prelude::*, *};

impl NodeBindingVersion {
    #[must_use]
    pub fn current(protocol: &ProtocolVersion) -> Self {
        Self { binding_version: env!("CARGO_PKG_VERSION").to_string(), protocol: protocol.into() }
    }
}

impl From<&ProtocolVersion> for NodeProtocolVersion {
    fn from(value: &ProtocolVersion) -> Self {
        Self { major: value.major, minor: value.minor }
    }
}

impl From<&ProtocolCompatibility> for NodeProtocolCompatibility {
    fn from(value: &ProtocolCompatibility) -> Self {
        Self { can_connect: value.can_connect, status: (&value.status).into() }
    }
}

impl From<&ProtocolCompatibilityStatus> for NodeProtocolCompatibilityStatus {
    fn from(value: &ProtocolCompatibilityStatus) -> Self {
        match value {
            ProtocolCompatibilityStatus::Compatible => Self::Compatible,
            ProtocolCompatibilityStatus::ProtocolMajorUnsupported => Self::ProtocolMajorUnsupported,
            ProtocolCompatibilityStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&HandshakeAssessment> for NodeHandshakeAssessment {
    fn from(value: &HandshakeAssessment) -> Self {
        Self {
            can_use: value.can_use,
            protocol: (&value.protocol).into(),
            status: (&value.status).into(),
        }
    }
}

impl From<&HandshakeAssessmentStatus> for NodeHandshakeAssessmentStatus {
    fn from(value: &HandshakeAssessmentStatus) -> Self {
        match value {
            HandshakeAssessmentStatus::Ready => Self::Ready,
            HandshakeAssessmentStatus::Starting => Self::Starting,
            HandshakeAssessmentStatus::Degraded => Self::Degraded,
            HandshakeAssessmentStatus::ProtocolMajorUnsupported => Self::ProtocolMajorUnsupported,
            HandshakeAssessmentStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&Handshake> for NodeHandshake {
    fn from(value: &Handshake) -> Self {
        Self {
            protocol_version: (&value.protocol_version).into(),
            binary_version: value.binary_version.clone(),
            daemon_phase: (&value.daemon_phase).into(),
            capabilities: (&value.capabilities).into(),
            available_backends: value.available_backends.iter().map(Into::into).collect(),
            session_scope: value.session_scope.clone(),
        }
    }
}

impl From<&DaemonPhase> for NodeDaemonPhase {
    fn from(value: &DaemonPhase) -> Self {
        match value {
            DaemonPhase::Starting => Self::Starting,
            DaemonPhase::Ready => Self::Ready,
            DaemonPhase::Degraded => Self::Degraded,
        }
    }
}

impl From<&DaemonCapabilities> for NodeDaemonCapabilities {
    fn from(value: &DaemonCapabilities) -> Self {
        Self {
            request_reply: value.request_reply,
            topology_subscriptions: value.topology_subscriptions,
            pane_subscriptions: value.pane_subscriptions,
            backend_discovery: value.backend_discovery,
            backend_capability_queries: value.backend_capability_queries,
            saved_sessions: value.saved_sessions,
            session_restore: value.session_restore,
            degraded_error_reasons: value.degraded_error_reasons,
            session_health: value.session_health,
        }
    }
}
