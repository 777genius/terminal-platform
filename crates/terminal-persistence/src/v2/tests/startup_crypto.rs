use super::super::*;
use super::support::*;

#[test]
fn detects_alternate_screen_buffer_mode_transitions() {
    let transitions =
        detect_buffer_mode_transitions(b"normal\x1b[?1049halt\x1b[?1049lnormal\x1b[?25h");

    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].action, "enter");
    assert_eq!(transitions[0].target_buffer_kind, "alternate");
    assert_eq!(transitions[0].mode, 1049);
    assert_eq!(transitions[1].action, "leave");
    assert_eq!(transitions[1].target_buffer_kind, "normal");
    assert_eq!(transitions[1].mode, 1049);
}

#[test]
fn opens_db_and_seeds_feature_gates_and_payload_schemas() {
    let store = test_store("seeds");

    assert_eq!(
        store
            .feature_gate_state(FeatureGateName::TerminalPersistenceV2Capture)
            .expect("gate should load"),
        FeatureGateState::Disabled
    );

    let mut connection = store.connection().expect("connection should open");
    let schemas = terminal_payload_schemas::table
        .select((
            terminal_payload_schemas::id,
            terminal_payload_schemas::schema_json,
            terminal_payload_schemas::schema_hash,
        ))
        .load::<(String, String, String)>(&mut connection)
        .expect("payload schemas should load");
    assert_eq!(schemas.len(), 4);
    assert!(schemas.iter().any(|(id, _, _)| id == PAYLOAD_SCHEMA_UI_INPUT_V1));
    assert!(schemas.iter().any(|(id, _, _)| id == PAYLOAD_SCHEMA_HISTORY_GAP_V1));
    assert!(schemas.iter().any(|(id, _, _)| id == PAYLOAD_SCHEMA_JOURNAL_EVENT_V1));
    assert!(schemas.iter().any(|(id, _, _)| id == PAYLOAD_SCHEMA_TOPOLOGY_SNAPSHOT_V1));
    for (_, schema_json, schema_hash) in schemas {
        assert_eq!(schema_hash, blake3_hash_text(&schema_json));
    }

    let identity = terminal_db_identity::table
        .filter(terminal_db_identity::id.eq(1))
        .select(TerminalDbIdentityRow::as_select())
        .first::<TerminalDbIdentityRow>(&mut connection)
        .expect("db identity should load");
    let notes: Value =
        serde_json::from_str(identity.notes.as_deref().expect("identity notes should exist"))
            .expect("identity notes should be json");
    assert_eq!(notes["diagnostic_kind"], "sqlite_startup");
    assert_eq!(notes["journal_mode"], "wal");
    assert_eq!(notes["configured_synchronous"], "NORMAL");
    assert_eq!(notes["foreign_keys"], true);
    assert_eq!(notes["configured_wal_autocheckpoint_pages"], 64);
    assert!(notes["compile_options"].as_array().is_some_and(|values| !values.is_empty()));
}

#[test]
fn authoritative_reads_gate_requires_authoritative_gate() {
    let store = test_store("feature-gate-deps");

    let reads_without_authoritative = store.set_feature_gate_state(
        FeatureGateName::TerminalPersistenceV2AuthoritativeReads,
        FeatureGateState::Enabled,
        Some("test"),
    );
    assert!(matches!(
        reads_without_authoritative,
        Err(TerminalPersistenceV2Error::InvalidData(message))
            if message.contains("requires terminal_persistence_v2_authoritative=enabled")
    ));

    store
        .set_feature_gate_state(
            FeatureGateName::TerminalPersistenceV2Authoritative,
            FeatureGateState::Enabled,
            Some("test"),
        )
        .expect("authoritative gate should enable");
    store
        .set_feature_gate_state(
            FeatureGateName::TerminalPersistenceV2AuthoritativeReads,
            FeatureGateState::Enabled,
            Some("test"),
        )
        .expect("reads gate should enable after authoritative gate");

    let disable_authoritative_first = store.set_feature_gate_state(
        FeatureGateName::TerminalPersistenceV2Authoritative,
        FeatureGateState::Disabled,
        Some("test"),
    );
    assert!(matches!(
        disable_authoritative_first,
        Err(TerminalPersistenceV2Error::InvalidData(message))
            if message.contains("disable terminal_persistence_v2_authoritative_reads first")
    ));

    store
        .set_feature_gate_state(
            FeatureGateName::TerminalPersistenceV2AuthoritativeReads,
            FeatureGateState::Disabled,
            Some("test"),
        )
        .expect("reads gate should disable");
    store
        .set_feature_gate_state(
            FeatureGateName::TerminalPersistenceV2Authoritative,
            FeatureGateState::Disabled,
            Some("test"),
        )
        .expect("authoritative gate should disable after reads gate");
}

#[test]
fn encrypted_history_gate_requires_active_database_key() {
    let store = test_store("encrypted-history-gate");

    let error = store
        .set_feature_gate_state(
            FeatureGateName::EncryptedTerminalHistory,
            FeatureGateState::Enabled,
            Some("test"),
        )
        .expect_err("encryption gate should fail without a database key");
    let capability =
        store.encryption_capability_state().expect("encryption capability should load");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("active non-test database key") || message.contains("active"))
    );
    assert_eq!(capability.feature_gate_state, "disabled");
    assert_eq!(capability.active_database_key_count, 0);
    assert!(!capability.can_enable_encrypted_history);
    assert!(!capability.plaintext_fallback_allowed);
    assert!(!capability.key_material_exported);
    assert_eq!(capability.action_required, "register_active_database_key");
}

#[test]
fn crypto_key_foundation_keeps_refs_opaque_and_allows_test_gate_only_in_test_config() {
    let store = test_store("crypto-key-foundation");
    let key = store
        .register_crypto_key(CryptoKeyInput {
            id: Some("test-db-key".to_string()),
            key_kind: "database_key".to_string(),
            key_ref: "test-provider:terminal-db-key".to_string(),
            protection_kind: "test_plaintext".to_string(),
            state: Some("active".to_string()),
            capability_report: Some(serde_json::json!({
                "provider": "test",
                "stores_key_material_in_db": false
            })),
            error: None,
            metadata: None,
        })
        .expect("test crypto key should register in test config");
    let event = store
        .record_crypto_key_event(CryptoKeyEventInput {
            id: None,
            key_id: Some(key.id.clone()),
            event_kind: "created".to_string(),
            actor: "test".to_string(),
            status: "succeeded".to_string(),
            error: None,
            metadata: Some(serde_json::json!({"audit_only": true})),
            occurred_at_ms: None,
        })
        .expect("crypto key event should persist");
    let capability =
        store.encryption_capability_state().expect("encryption capability should load");

    assert_ne!(key.key_ref_hash, "test-provider:terminal-db-key");
    assert_eq!(key.key_ref_hash, blake3_hash_text("test-provider:terminal-db-key"));
    assert_eq!(event.key_id.as_deref(), Some("test-db-key"));
    assert_eq!(capability.active_database_key_count, 1);
    assert_eq!(capability.test_plaintext_database_key_count, 1);
    assert!(capability.can_enable_encrypted_history);
    assert_eq!(capability.action_required, "none");

    store
        .set_feature_gate_state(
            FeatureGateName::EncryptedTerminalHistory,
            FeatureGateState::Enabled,
            Some("test key in test config"),
        )
        .expect("test config can enable encrypted history with a test key");
    let reopened =
        TerminalPersistenceV2::open_with_config(store.path(), TerminalPersistenceV2Config::test())
            .expect("test config should reopen enabled encrypted-history gate");
    assert_eq!(
        reopened
            .feature_gate_state(FeatureGateName::EncryptedTerminalHistory)
            .expect("gate should load"),
        FeatureGateState::Enabled
    );
}

#[test]
fn production_config_rejects_test_plaintext_crypto_keys_and_key_material_refs() {
    let path = std::env::temp_dir()
        .join(format!("terminal-persistence-v2-crypto-prod-{}.sqlite3", Uuid::new_v4()));
    let store =
        TerminalPersistenceV2::open_with_config(path, TerminalPersistenceV2Config::default())
            .expect("production-like store should open");

    let test_key_error = store
        .register_crypto_key(CryptoKeyInput {
            id: None,
            key_kind: "database_key".to_string(),
            key_ref: "test-provider:terminal-db-key".to_string(),
            protection_kind: "test_plaintext".to_string(),
            state: Some("active".to_string()),
            capability_report: None,
            error: None,
            metadata: None,
        })
        .expect_err("production config should reject test plaintext keys");
    let material_ref_error = store
        .register_crypto_key(CryptoKeyInput {
            id: None,
            key_kind: "database_key".to_string(),
            key_ref: "-----BEGIN PRIVATE KEY-----".to_string(),
            protection_kind: "dpapi_user".to_string(),
            state: Some("active".to_string()),
            capability_report: None,
            error: None,
            metadata: None,
        })
        .expect_err("key refs must not contain key material");

    assert!(
        matches!(test_key_error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("test_plaintext"))
    );
    assert!(
        matches!(material_ref_error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("key material"))
    );
}

#[test]
fn external_artifact_metadata_hashes_refs_and_keeps_future_store_inert() {
    let store = test_store("external-artifact-metadata");
    let artifact_ref = r"C:\Users\User\PROJECT_IT\terminal-platform\backups\history.sqlite3";
    let artifact = store
        .record_external_artifact(ExternalArtifactInput {
            id: Some("artifact-a".to_string()),
            artifact_kind: "backup_file".to_string(),
            artifact_ref: artifact_ref.to_string(),
            state: Some("verified".to_string()),
            encryption_state: Some("encrypted".to_string()),
            key_ref: Some("dpapi:user:terminal-artifact-key".to_string()),
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(blake3_hash_text("backup-bytes")),
            size_bytes: Some(123),
            verified_at_ms: Some(42),
            metadata: Some(serde_json::json!({"path_safety": "hashed_ref_only"})),
        })
        .expect("external artifact metadata should persist");

    assert_eq!(artifact.id, "artifact-a");
    assert_eq!(artifact.artifact_kind, "backup_file");
    assert_eq!(artifact.state, "verified");
    assert_eq!(artifact.encryption_state, "encrypted");
    assert_eq!(artifact.artifact_ref_hash, blake3_hash_text(artifact_ref));
    assert_ne!(artifact.artifact_ref_hash, artifact_ref);
    assert_eq!(
        artifact.metadata_json.as_ref().and_then(|value| value["path_safety"].as_str()),
        Some("hashed_ref_only")
    );
}

#[test]
fn external_artifact_metadata_rejects_unknown_domains() {
    let store = test_store("external-artifact-domains");

    let error = store
        .record_external_artifact(ExternalArtifactInput {
            id: None,
            artifact_kind: "raw_path".to_string(),
            artifact_ref: "opaque-ref".to_string(),
            state: Some("verified".to_string()),
            encryption_state: Some("encrypted".to_string()),
            key_ref: None,
            checksum_algorithm: None,
            checksum: None,
            size_bytes: None,
            verified_at_ms: None,
            metadata: None,
        })
        .expect_err("unknown artifact kind should fail");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("external artifact kind"))
    );
}

#[test]
fn external_artifact_metadata_rejects_live_database_and_sidecar_refs() {
    let store = test_store("external-artifact-live-db-refs");
    let live_db_ref = store.path().to_string_lossy().to_string();
    let wal_ref = sqlite_sidecar_path(store.path(), "-wal").to_string_lossy().to_string();

    for artifact_ref in [live_db_ref, wal_ref] {
        let error = store
            .record_external_artifact(ExternalArtifactInput {
                id: None,
                artifact_kind: "backup_file".to_string(),
                artifact_ref,
                state: Some("planned".to_string()),
                encryption_state: Some("encrypted".to_string()),
                key_ref: Some("crypto-key:artifact".to_string()),
                checksum_algorithm: Some("blake3".to_string()),
                checksum: Some(blake3_hash_text("artifact-bytes")),
                size_bytes: Some(1),
                verified_at_ms: None,
                metadata: None,
            })
            .expect_err("live db and sidecar refs must be rejected");
        assert!(
            matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("live database or SQLite sidecar"))
        );
    }
}

#[test]
fn crypto_erase_records_destroyed_key_event_and_tombstone_limitations() {
    let store = test_store("crypto-erase");
    let key = store
        .register_crypto_key(CryptoKeyInput {
            id: Some("erase-key".to_string()),
            key_kind: "database_key".to_string(),
            key_ref: "dpapi:user:erase-key".to_string(),
            protection_kind: "test_plaintext".to_string(),
            state: Some("active".to_string()),
            capability_report: None,
            error: None,
            metadata: None,
        })
        .expect("crypto key should register");

    let erased = store
        .complete_crypto_erase(CryptoEraseInput {
            id: Some("erase-request".to_string()),
            key_id: key.id.clone(),
            session_id: None,
            requester_ref: Some("user-a".to_string()),
            reason: Some("test erase".to_string()),
            metadata: Some(serde_json::json!({"requested_by": "test"})),
        })
        .expect("crypto erase should complete");

    let mut connection = store.connection().expect("connection should open");
    let (state, destroyed_at_ms) = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::id.eq(&key.id))
        .select((terminal_crypto_keys::state, terminal_crypto_keys::destroyed_at_ms))
        .first::<(String, Option<i64>)>(&mut connection)
        .expect("destroyed key should load");
    let event_count = terminal_crypto_key_events::table
        .filter(terminal_crypto_key_events::key_id.eq(Some(key.id.clone())))
        .filter(terminal_crypto_key_events::event_kind.eq("destroyed"))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("destroy event should count");
    let evidence_json = terminal_deletion_tombstones::table
        .filter(terminal_deletion_tombstones::id.eq(&erased.tombstone_id))
        .select(terminal_deletion_tombstones::evidence_json)
        .first::<Option<String>>(&mut connection)
        .expect("crypto erase tombstone should load");
    let evidence: Value =
        serde_json::from_str(evidence_json.as_deref().expect("crypto erase evidence should exist"))
            .expect("crypto erase evidence should be json");

    assert_eq!(erased.delete_request_id, "erase-request");
    assert_eq!(erased.key_ref_hash, blake3_hash_text("dpapi:user:erase-key"));
    assert_eq!(state, "destroyed");
    assert!(destroyed_at_ms.is_some());
    assert_eq!(event_count, 1);
    assert_eq!(evidence["canonical_history_deleted"], false);
    assert_eq!(evidence["key_material_exported"], false);
    assert_eq!(evidence["key_ref_hash"], erased.key_ref_hash);
    assert_ne!(evidence["key_ref_hash"], "dpapi:user:erase-key");
    assert!(erased.secure_deletion_limitation.contains("sqlite_pages"));
}
