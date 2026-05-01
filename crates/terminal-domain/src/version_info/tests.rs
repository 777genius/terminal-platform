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
