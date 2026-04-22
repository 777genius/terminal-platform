use proptest::{collection::vec, option::of, prelude::*, test_runner::TestCaseError};
use terminal_domain::{BackendKind, DegradedModeReason, OperationId};
use terminal_protocol::{
    DaemonCapabilities, DaemonPhase, Handshake, ProtocolError, ProtocolVersion, ResponseEnvelope,
    ResponsePayload, TransportResponse, decode_json_frame, encode_json_frame,
};

fn backend_kind_strategy() -> impl Strategy<Value = BackendKind> {
    prop_oneof![Just(BackendKind::Native), Just(BackendKind::Tmux), Just(BackendKind::Zellij),]
}

fn daemon_phase_strategy() -> impl Strategy<Value = DaemonPhase> {
    prop_oneof![Just(DaemonPhase::Starting), Just(DaemonPhase::Ready), Just(DaemonPhase::Degraded),]
}

fn degraded_mode_reason_strategy() -> impl Strategy<Value = DegradedModeReason> {
    prop_oneof![
        Just(DegradedModeReason::UnsupportedByBackend),
        Just(DegradedModeReason::MissingCapability),
        Just(DegradedModeReason::ImportedForeignSession),
        Just(DegradedModeReason::ResizeAuthorityExternal),
        Just(DegradedModeReason::ReadOnlyRoute),
        Just(DegradedModeReason::SavedSessionIncompatible),
        Just(DegradedModeReason::NotYetImplemented),
    ]
}

fn safe_string_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9._:-]{0,48}"
}

fn binary_version_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            proptest::char::range('a', 'z'),
            proptest::char::range('A', 'Z'),
            proptest::char::range('0', '9'),
            Just('.'),
            Just('_'),
            Just('-'),
        ],
        1..33,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn handshake_strategy() -> impl Strategy<Value = Handshake> {
    (
        any::<u16>(),
        any::<u16>(),
        binary_version_strategy(),
        daemon_phase_strategy(),
        any::<[bool; 7]>(),
        vec(backend_kind_strategy(), 0..4),
        safe_string_strategy(),
    )
        .prop_map(
            |(
                major,
                minor,
                binary_version,
                daemon_phase,
                capability_flags,
                available_backends,
                session_scope,
            )| Handshake {
                protocol_version: ProtocolVersion { major, minor },
                binary_version,
                daemon_phase,
                capabilities: DaemonCapabilities {
                    request_reply: capability_flags[0],
                    topology_subscriptions: capability_flags[1],
                    pane_subscriptions: capability_flags[2],
                    backend_discovery: capability_flags[3],
                    backend_capability_queries: capability_flags[4],
                    saved_sessions: capability_flags[5],
                    session_restore: capability_flags[6],
                    degraded_error_reasons: true,
                    session_health: true,
                },
                available_backends,
                session_scope,
            },
        )
}

fn protocol_error_strategy() -> impl Strategy<Value = ProtocolError> {
    ("[a-z0-9_]{1,24}", safe_string_strategy(), of(degraded_mode_reason_strategy())).prop_map(
        |(code, message, degraded_reason)| ProtocolError { code, message, degraded_reason },
    )
}

proptest! {
    #![proptest_config(proptest::test_runner::Config {
        failure_persistence: None,
        .. proptest::test_runner::Config::default()
    })]

    #[test]
    fn handshake_roundtrips_through_transport_frames(handshake in handshake_strategy()) {
        let frame = encode_json_frame(&handshake)
            .map_err(|error| TestCaseError::fail(format!("handshake encode failed: {error}")))?;
        let restored: Handshake = decode_json_frame(&frame)
            .map_err(|error| TestCaseError::fail(format!("handshake decode failed: {error}")))?;

        prop_assert_eq!(restored, handshake);
    }

    #[test]
    fn transport_response_roundtrips_success_envelopes(handshake in handshake_strategy()) {
        let envelope = ResponseEnvelope {
            operation_id: OperationId::new(),
            payload: ResponsePayload::Handshake(handshake),
        };
        let transport = TransportResponse::from_result(Ok(envelope.clone()));
        let frame = encode_json_frame(&transport)
            .map_err(|error| TestCaseError::fail(format!("transport encode failed: {error}")))?;
        let restored: TransportResponse = decode_json_frame(&frame)
            .map_err(|error| TestCaseError::fail(format!("transport decode failed: {error}")))?;

        prop_assert_eq!(restored.clone(), transport);
        match restored.into_result() {
            Ok(result) => prop_assert_eq!(result, envelope),
            Err(error) => {
                return Err(TestCaseError::fail(format!(
                    "success transport restored as error: {error}"
                )));
            }
        }
    }

    #[test]
    fn transport_response_roundtrips_error_envelopes(error in protocol_error_strategy()) {
        let transport = TransportResponse::from_result(Err(error.clone()));
        let frame = encode_json_frame(&transport)
            .map_err(|error| TestCaseError::fail(format!("error transport encode failed: {error}")))?;
        let restored: TransportResponse = decode_json_frame(&frame)
            .map_err(|error| TestCaseError::fail(format!("error transport decode failed: {error}")))?;

        prop_assert_eq!(restored.clone(), transport);
        match restored.into_result() {
            Ok(response) => {
                return Err(TestCaseError::fail(format!(
                    "error transport restored as success: {response:?}"
                )));
            }
            Err(restored_error) => prop_assert_eq!(restored_error, error),
        }
    }

    #[test]
    fn decode_json_frame_rejects_non_json_bytes(frame in vec(any::<u8>(), 0..128)) {
        prop_assume!(serde_json::from_slice::<serde_json::Value>(&frame).is_err());

        match decode_json_frame::<Handshake>(&frame) {
            Ok(handshake) => {
                return Err(TestCaseError::fail(format!(
                    "invalid json unexpectedly decoded as handshake: {handshake:?}"
                )));
            }
            Err(error) => prop_assert_eq!(error.code, "deserialize_failed"),
        }
    }
}
