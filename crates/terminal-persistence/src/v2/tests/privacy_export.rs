use super::super::*;
use super::support::*;

#[test]
fn export_and_support_are_redacted_by_default_and_raw_is_gated() {
    let store = test_store("privacy-workflows");
    let (session_id, _pane_id, _writer) = session_and_pane(&store);

    let export = store
        .create_export_request(ExportRequestInput {
            id: None,
            session_id: Some(session_id.clone()),
            export_kind: None,
            redaction_profile_id: None,
            include_raw: false,
            output_ref: Some("C:\\temp\\redacted-export.json".to_string()),
            metadata: None,
        })
        .expect("redacted export request should persist");
    let support = store
        .create_support_bundle(SupportBundleInput {
            id: None,
            scope: serde_json::json!({"session_id": session_id}),
            redaction_profile_id: None,
            include_raw: false,
            output_ref: Some("support-bundle.zip".to_string()),
            metadata: None,
        })
        .expect("redacted support bundle should persist");
    let raw_export = store.create_export_request(ExportRequestInput {
        id: None,
        session_id: None,
        export_kind: Some("raw_transcript".to_string()),
        redaction_profile_id: None,
        include_raw: true,
        output_ref: None,
        metadata: None,
    });

    assert!(!export.include_raw);
    assert_eq!(
        export.manifest_json.as_ref().and_then(|value| value["raw_terminal_output"].as_bool()),
        Some(false)
    );
    assert!(!support.include_raw);
    assert_eq!(
        support
            .manifest_json
            .as_ref()
            .and_then(|value| value["excluded_classes"].as_array())
            .map(Vec::len),
        Some(2)
    );
    assert!(
        matches!(raw_export, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("raw history export is disabled"))
    );

    store
        .set_feature_gate_state(
            FeatureGateName::RawHistoryExport,
            FeatureGateState::Enabled,
            Some("test raw export approval"),
        )
        .expect("raw export gate should enable");
    let approved_raw_export = store
        .create_export_request(ExportRequestInput {
            id: None,
            session_id: None,
            export_kind: Some("raw_transcript".to_string()),
            redaction_profile_id: None,
            include_raw: true,
            output_ref: None,
            metadata: None,
        })
        .expect("raw export should persist when gate is enabled");
    assert!(approved_raw_export.include_raw);
}

#[test]
fn raw_export_artifact_verifier_requires_approval_and_encrypted_artifact() {
    let store = test_store("raw-export-artifact-verifier");
    let (session_id, _pane_id, _writer) = session_and_pane(&store);
    store
        .set_feature_gate_state(
            FeatureGateName::RawHistoryExport,
            FeatureGateState::Enabled,
            Some("test raw export approval"),
        )
        .expect("raw export gate should enable");

    let plaintext_ref = "C:\\exports\\raw-history-plaintext.cast";
    let plaintext_request = store
        .create_export_request(ExportRequestInput {
            id: None,
            session_id: Some(session_id.clone()),
            export_kind: Some("raw_transcript".to_string()),
            redaction_profile_id: None,
            include_raw: true,
            output_ref: Some(plaintext_ref.to_string()),
            metadata: None,
        })
        .expect("raw export request should persist");
    store
        .record_external_artifact(ExternalArtifactInput {
            id: None,
            artifact_kind: "export_file".to_string(),
            artifact_ref: plaintext_ref.to_string(),
            state: Some("available".to_string()),
            encryption_state: Some("plaintext".to_string()),
            key_ref: None,
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(blake3_hash_text("plaintext-export-bytes")),
            size_bytes: Some(64),
            verified_at_ms: None,
            metadata: None,
        })
        .expect("plaintext export artifact should persist as metadata only");

    let unapproved = store.verify_export_artifact(ExportArtifactVerificationInput {
        export_request_id: plaintext_request.id.clone(),
        artifact_ref: plaintext_ref.to_string(),
        require_encrypted: false,
        metadata: None,
    });
    assert!(
        matches!(unapproved, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("explicitly approved"))
    );

    let approved = store
        .approve_export_request(ExportApprovalInput {
            export_request_id: plaintext_request.id.clone(),
            approver_ref: Some("local-user".to_string()),
            metadata: Some(serde_json::json!({"reason": "test approval"})),
        })
        .expect("raw export request should approve");
    assert_eq!(approved.state, "approved");
    let approval_metadata =
        serde_json::to_string(&approved.metadata_json).expect("approval metadata should serialize");
    assert!(!approval_metadata.contains("local-user"));

    let plaintext_completion = store.verify_export_artifact(ExportArtifactVerificationInput {
        export_request_id: plaintext_request.id,
        artifact_ref: plaintext_ref.to_string(),
        require_encrypted: false,
        metadata: None,
    });
    assert!(
        matches!(plaintext_completion, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("encrypted artifact"))
    );

    let encrypted_ref = "C:\\exports\\raw-history-encrypted.cast";
    let encrypted_request = store
        .create_export_request(ExportRequestInput {
            id: None,
            session_id: Some(session_id),
            export_kind: Some("raw_transcript".to_string()),
            redaction_profile_id: None,
            include_raw: true,
            output_ref: Some(encrypted_ref.to_string()),
            metadata: None,
        })
        .expect("second raw export request should persist");
    store
        .approve_export_request(ExportApprovalInput {
            export_request_id: encrypted_request.id.clone(),
            approver_ref: Some("local-user".to_string()),
            metadata: None,
        })
        .expect("second raw export request should approve");
    store
        .record_external_artifact(ExternalArtifactInput {
            id: None,
            artifact_kind: "export_file".to_string(),
            artifact_ref: encrypted_ref.to_string(),
            state: Some("available".to_string()),
            encryption_state: Some("encrypted".to_string()),
            key_ref: Some("crypto-key:export-v1".to_string()),
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(blake3_hash_text("encrypted-export-bytes")),
            size_bytes: Some(128),
            verified_at_ms: None,
            metadata: None,
        })
        .expect("encrypted export artifact should persist as metadata only");

    let verified = store
        .verify_export_artifact(ExportArtifactVerificationInput {
            export_request_id: encrypted_request.id,
            artifact_ref: encrypted_ref.to_string(),
            require_encrypted: false,
            metadata: Some(serde_json::json!({"worker": "test"})),
        })
        .expect("encrypted raw export should verify");
    assert_eq!(verified.export_state, "succeeded");
    assert_eq!(verified.artifact_state, "verified");
    assert!(verified.raw_export);
    assert!(verified.encrypted_required);
    assert_eq!(verified.encryption_state, "encrypted");
    assert_eq!(verified.artifact_ref_hash, blake3_hash_text(encrypted_ref));
    let manifest = serde_json::to_string(&verified.manifest_json)
        .expect("verification manifest should serialize");
    assert!(manifest.contains("artifact_ref_stored"));
    assert!(!manifest.contains(encrypted_ref));
}

#[test]
fn support_bundle_completion_writes_redacted_diagnostics_manifest() {
    let store = test_store("support-bundle-diagnostics");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id,
            writer.id,
            b"support diagnostics should not serialize this raw output\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let artifact_ref = "C:\\exports\\support-bundle-redacted.zip";
    let support = store
        .create_support_bundle(SupportBundleInput {
            id: None,
            scope: serde_json::json!({"session_id": session_id, "path": "C:\\secret\\project"}),
            redaction_profile_id: None,
            include_raw: false,
            output_ref: Some(artifact_ref.to_string()),
            metadata: None,
        })
        .expect("support bundle request should persist");
    store
        .record_external_artifact(ExternalArtifactInput {
            id: None,
            artifact_kind: "support_bundle".to_string(),
            artifact_ref: artifact_ref.to_string(),
            state: Some("available".to_string()),
            encryption_state: Some("redacted".to_string()),
            key_ref: None,
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(blake3_hash_text("redacted-support-bytes")),
            size_bytes: Some(256),
            verified_at_ms: None,
            metadata: None,
        })
        .expect("support artifact metadata should persist");

    let diagnostics =
        store.support_bundle_diagnostics(&support.id).expect("support diagnostics should build");
    assert!(!diagnostics.include_raw);
    assert!(!diagnostics.raw_content_included);
    assert_eq!(
        diagnostics.manifest_json["raw_terminal_output_rows_serialized"].as_bool(),
        Some(false)
    );
    let diagnostics_json =
        serde_json::to_string(&diagnostics.manifest_json).expect("diagnostics should serialize");
    assert!(!diagnostics_json.contains("C:\\secret\\project"));
    assert!(!diagnostics_json.contains("support diagnostics should not serialize"));
    assert!(!diagnostics_json.contains(artifact_ref));

    let completed = store
        .complete_support_bundle(SupportBundleCompletionInput {
            support_bundle_id: support.id,
            artifact_ref: Some(artifact_ref.to_string()),
            metadata: Some(serde_json::json!({"worker": "test"})),
        })
        .expect("support bundle should complete");
    assert_eq!(completed.state, "succeeded");
    assert!(completed.completed_at_ms.is_some());
    let manifest = completed.manifest_json.expect("manifest should exist");
    assert_eq!(manifest["diagnostics"]["raw_paths_serialized"].as_bool(), Some(false));
    assert_eq!(manifest["diagnostics"]["crypto_key_refs_serialized"].as_bool(), Some(false));
    assert_eq!(manifest["artifact_verification"]["artifact_ref_stored"].as_bool(), Some(false));
    let manifest_json = serde_json::to_string(&manifest).expect("manifest should serialize");
    assert!(!manifest_json.contains("C:\\secret\\project"));
    assert!(!manifest_json.contains("support diagnostics should not serialize"));
    assert!(!manifest_json.contains(artifact_ref));
}

#[test]
fn raw_support_bundle_completion_requires_encrypted_artifact() {
    let store = test_store("raw-support-bundle-artifact");
    store
        .set_feature_gate_state(
            FeatureGateName::RawHistoryExport,
            FeatureGateState::Enabled,
            Some("test raw support approval"),
        )
        .expect("raw support gate should enable");

    let plaintext_ref = "C:\\exports\\raw-support-plaintext.zip";
    let support = store
        .create_support_bundle(SupportBundleInput {
            id: None,
            scope: serde_json::json!({"all_sessions": true}),
            redaction_profile_id: None,
            include_raw: true,
            output_ref: Some(plaintext_ref.to_string()),
            metadata: None,
        })
        .expect("raw support request should persist");
    store
        .record_external_artifact(ExternalArtifactInput {
            id: None,
            artifact_kind: "support_bundle".to_string(),
            artifact_ref: plaintext_ref.to_string(),
            state: Some("available".to_string()),
            encryption_state: Some("plaintext".to_string()),
            key_ref: None,
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(blake3_hash_text("raw-support-plaintext-bytes")),
            size_bytes: Some(128),
            verified_at_ms: None,
            metadata: None,
        })
        .expect("plaintext raw support artifact metadata should persist");

    let plaintext_completion = store.complete_support_bundle(SupportBundleCompletionInput {
        support_bundle_id: support.id,
        artifact_ref: Some(plaintext_ref.to_string()),
        metadata: None,
    });
    assert!(
        matches!(plaintext_completion, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("encrypted artifact"))
    );

    let encrypted_ref = "C:\\exports\\raw-support-encrypted.zip";
    let encrypted_support = store
        .create_support_bundle(SupportBundleInput {
            id: None,
            scope: serde_json::json!({"all_sessions": true}),
            redaction_profile_id: None,
            include_raw: true,
            output_ref: Some(encrypted_ref.to_string()),
            metadata: None,
        })
        .expect("encrypted raw support request should persist");
    store
        .record_external_artifact(ExternalArtifactInput {
            id: None,
            artifact_kind: "support_bundle".to_string(),
            artifact_ref: encrypted_ref.to_string(),
            state: Some("available".to_string()),
            encryption_state: Some("encrypted".to_string()),
            key_ref: Some("crypto-key:support-v1".to_string()),
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(blake3_hash_text("raw-support-encrypted-bytes")),
            size_bytes: Some(192),
            verified_at_ms: None,
            metadata: None,
        })
        .expect("encrypted raw support artifact metadata should persist");
    let completed = store
        .complete_support_bundle(SupportBundleCompletionInput {
            support_bundle_id: encrypted_support.id,
            artifact_ref: Some(encrypted_ref.to_string()),
            metadata: None,
        })
        .expect("encrypted raw support bundle should complete");
    assert_eq!(completed.state, "succeeded");
    assert!(completed.include_raw);
    let manifest = completed.manifest_json.expect("manifest should exist");
    assert_eq!(manifest["diagnostics"]["include_raw"].as_bool(), Some(true));
    assert_eq!(manifest["artifact_verification"]["encryption_state"].as_str(), Some("encrypted"));
    let manifest_json = serde_json::to_string(&manifest).expect("manifest should serialize");
    assert!(!manifest_json.contains(encrypted_ref));
    assert!(!manifest_json.contains("crypto-key:support-v1"));
}

#[test]
fn raw_export_and_support_bundle_are_blocked_by_critical_health_records() {
    let store = test_store("raw-export-health-block");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let output = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id,
            writer.id,
            b"secret bearing output\r\n".to_vec(),
        ))
        .expect("segment should persist");
    store
        .set_feature_gate_state(
            FeatureGateName::RawHistoryExport,
            FeatureGateState::Enabled,
            Some("test raw export approval"),
        )
        .expect("raw export gate should enable");

    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_stream_segments::table.filter(terminal_stream_segments::id.eq(&output.segment_id)),
    )
    .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
    .execute(&mut connection)
    .expect("test should corrupt checksum");
    let integrity = store.run_integrity_check().expect("integrity check should run");
    assert_eq!(integrity.result, "failed");

    let redacted_export = store
        .create_export_request(ExportRequestInput {
            id: None,
            session_id: Some(session_id.clone()),
            export_kind: None,
            redaction_profile_id: None,
            include_raw: false,
            output_ref: None,
            metadata: None,
        })
        .expect("redacted export should still be allowed");
    assert!(!redacted_export.include_raw);

    let raw_export = store.create_export_request(ExportRequestInput {
        id: None,
        session_id: Some(session_id.clone()),
        export_kind: Some("raw_transcript".to_string()),
        redaction_profile_id: None,
        include_raw: true,
        output_ref: None,
        metadata: None,
    });
    assert!(
        matches!(raw_export, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("raw export is blocked by open critical data health record"))
    );

    let raw_support = store.create_support_bundle(SupportBundleInput {
        id: None,
        scope: serde_json::json!({"session_id": session_id}),
        redaction_profile_id: None,
        include_raw: true,
        output_ref: None,
        metadata: None,
    });
    assert!(
        matches!(raw_support, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("raw support bundle is blocked by open critical data health record"))
    );
}

#[test]
fn delete_request_writes_tombstone_without_deleting_canonical_history() {
    let store = test_store("delete-workflow");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"history must not disappear silently\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let request = store
        .create_delete_request(DeleteRequestInput {
            id: None,
            session_id: Some(session_id.clone()),
            request_kind: Some("user_delete".to_string()),
            policy_id: None,
            requester_ref: Some("local-user".to_string()),
            reason: Some("test delete request".to_string()),
            metadata: None,
        })
        .expect("delete request should persist");
    let tombstone = store
        .complete_delete_request_with_tombstone(
            &request.id,
            "session",
            Some(serde_json::json!({"canonical_delete_deferred": true})),
            None,
        )
        .expect("tombstone should persist");
    let segments = store
        .list_stream_segments(&session_id, &pane_id, 1, 10)
        .expect("canonical history should remain readable");

    assert_eq!(request.state, "pending");
    assert_eq!(tombstone.session_id.as_deref(), Some(session_id.as_str()));
    assert_eq!(tombstone.deleted_scope, "session");
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].payload, b"history must not disappear silently\r\n");
}

#[test]
fn canonical_history_prevents_parent_delete() {
    let store = test_store("restrict-delete");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id,
            writer.id,
            b"must stay durable\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let mut connection = store.connection().expect("connection should open");
    let delete_result =
        diesel::delete(terminal_sessions::table.filter(terminal_sessions::id.eq(&session_id)))
            .execute(&mut connection);

    assert!(delete_result.is_err());
}
