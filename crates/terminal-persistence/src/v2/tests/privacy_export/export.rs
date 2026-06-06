use super::super::super::*;
use super::super::support::*;

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
