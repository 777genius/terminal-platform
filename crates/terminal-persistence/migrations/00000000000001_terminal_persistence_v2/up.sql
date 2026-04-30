CREATE TABLE IF NOT EXISTS terminal_db_identity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    product TEXT NOT NULL,
    schema_family TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    app_version TEXT,
    diesel_version TEXT,
    sqlite_version TEXT,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS terminal_payload_schemas (
    id TEXT PRIMARY KEY,
    payload_kind TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    schema_json TEXT NOT NULL,
    schema_hash TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS terminal_projection_versions (
    id TEXT PRIMARY KEY,
    projection_kind TEXT NOT NULL,
    version TEXT NOT NULL,
    parser_version TEXT,
    payload_schema_id TEXT REFERENCES terminal_payload_schemas(id) ON DELETE RESTRICT,
    created_at_ms BIGINT NOT NULL,
    UNIQUE(projection_kind, version)
);

CREATE TABLE IF NOT EXISTS terminal_feature_gates (
    id TEXT PRIMARY KEY,
    feature_name TEXT NOT NULL UNIQUE,
    state TEXT NOT NULL CHECK(state IN ('disabled', 'shadow', 'enabled', 'force_disabled')),
    rollout_scope TEXT NOT NULL CHECK(rollout_scope IN ('global', 'session', 'backend', 'developer')),
    reason TEXT,
    enabled_at_ms BIGINT,
    disabled_at_ms BIGINT,
    updated_at_ms BIGINT NOT NULL,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_retention_policies (
    id TEXT PRIMARY KEY,
    policy_kind TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0 CHECK(is_default IN (0, 1)),
    max_bytes BIGINT,
    max_age_ms BIGINT,
    pressure_behavior TEXT NOT NULL CHECK(pressure_behavior IN ('warn_only', 'degrade_with_gap', 'delete_by_request')),
    raw_history_prune_behavior TEXT NOT NULL CHECK(raw_history_prune_behavior IN ('never_silent', 'request_only')),
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);

INSERT OR IGNORE INTO terminal_retention_policies (
    id,
    policy_kind,
    is_default,
    pressure_behavior,
    raw_history_prune_behavior,
    created_at_ms,
    updated_at_ms
) VALUES (
    'default_full_history',
    'full_history',
    1,
    'warn_only',
    'never_silent',
    CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER),
    CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)
);

INSERT OR IGNORE INTO terminal_feature_gates (
    id,
    feature_name,
    state,
    rollout_scope,
    reason,
    updated_at_ms
) VALUES
    ('terminal_persistence_v2_shadow', 'terminal_persistence_v2_shadow', 'disabled', 'global', 'initial safe default', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    ('terminal_persistence_v2_capture', 'terminal_persistence_v2_capture', 'disabled', 'global', 'requires writer proof', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    ('terminal_persistence_v2_authoritative_reads', 'terminal_persistence_v2_authoritative_reads', 'disabled', 'global', 'requires restore drill proof', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    ('terminal_persistence_v2_authoritative', 'terminal_persistence_v2_authoritative', 'disabled', 'global', 'requires MVP reliability gate', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    ('mux_structured_capture', 'mux_structured_capture', 'disabled', 'backend', 'requires backend capability probes', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    ('segment_compression_zstd', 'segment_compression_zstd', 'disabled', 'global', 'requires compressed restore drills', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    ('raw_history_export', 'raw_history_export', 'disabled', 'global', 'requires export approval flow', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)),
    ('encrypted_terminal_history', 'encrypted_terminal_history', 'disabled', 'global', 'requires key-store proof', CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER));

CREATE TABLE IF NOT EXISTS terminal_maintenance_runs (
    id TEXT PRIMARY KEY,
    run_kind TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending', 'running', 'succeeded', 'failed', 'skipped')),
    selected_policy_id TEXT REFERENCES terminal_retention_policies(id) ON DELETE SET NULL,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT,
    summary_json TEXT,
    error TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_integrity_checks (
    id TEXT PRIMARY KEY,
    check_kind TEXT NOT NULL,
    scope_kind TEXT NOT NULL,
    scope_ref TEXT,
    result TEXT NOT NULL CHECK(result IN ('passed', 'failed', 'degraded', 'skipped')),
    checked_at_ms BIGINT NOT NULL,
    details_json TEXT,
    error TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_data_health_records (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    pane_id TEXT,
    detection_kind TEXT NOT NULL CHECK(detection_kind IN ('checksum_mismatch', 'decode_failed', 'parser_failed', 'projection_drift', 'missing_segment', 'migration_mismatch', 'manual')),
    severity TEXT NOT NULL CHECK(severity IN ('info', 'warning', 'error', 'critical')),
    first_bad_event_seq BIGINT,
    affected_ref TEXT,
    action_state TEXT NOT NULL CHECK(action_state IN ('open', 'quarantined', 'rebuild_pending', 'resolved', 'ignored')),
    detected_at_ms BIGINT NOT NULL,
    resolved_at_ms BIGINT,
    details_json TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_sessions (
    id TEXT PRIMARY KEY,
    route_json TEXT NOT NULL,
    title TEXT,
    launch_json TEXT,
    source TEXT NOT NULL,
    durability_profile TEXT NOT NULL CHECK(durability_profile IN ('reliable_history', 'performance_history', 'test')),
    retention_policy_id TEXT NOT NULL DEFAULT 'default_full_history' REFERENCES terminal_retention_policies(id) ON DELETE RESTRICT,
    private_mode INTEGER NOT NULL DEFAULT 0 CHECK(private_mode IN (0, 1)),
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    closed_at_ms BIGINT,
    state TEXT NOT NULL CHECK(state IN ('active', 'closed', 'deleted', 'legacy_visual_only')),
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_panes (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    tab_id TEXT,
    stream_id TEXT NOT NULL DEFAULT 'primary',
    title TEXT,
    rows INTEGER NOT NULL CHECK(rows > 0),
    cols INTEGER NOT NULL CHECK(cols > 0),
    last_event_seq BIGINT NOT NULL DEFAULT 0 CHECK(last_event_seq >= 0),
    created_at_ms BIGINT NOT NULL,
    closed_at_ms BIGINT,
    metadata_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_terminal_panes_session
ON terminal_panes(session_id);

CREATE TABLE IF NOT EXISTS terminal_backend_capability_reports (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    backend_kind TEXT NOT NULL,
    backend_version TEXT,
    backend_binary_path_hash TEXT,
    route_kind TEXT NOT NULL,
    probe_status TEXT NOT NULL CHECK(probe_status IN ('passed', 'failed', 'partial', 'stale')),
    capture_strategy TEXT NOT NULL CHECK(capture_strategy IN ('raw_stream', 'rendered_stream', 'rendered_snapshot', 'mux_structured', 'imported_snapshot', 'ui_input', 'unknown')),
    capture_semantics TEXT NOT NULL CHECK(capture_semantics IN ('raw_vt_stream', 'rendered_ansi_stream', 'rendered_plaintext_snapshot', 'mux_structured_surface', 'imported_text', 'ui_input')),
    can_preserve_process_when_live INTEGER NOT NULL CHECK(can_preserve_process_when_live IN (0, 1)),
    can_capture_scrollback INTEGER NOT NULL CHECK(can_capture_scrollback IN (0, 1)),
    command_boundary_confidence TEXT NOT NULL CHECK(command_boundary_confidence IN ('verified', 'high', 'medium', 'low', 'none', 'unknown')),
    evidence_json TEXT,
    created_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT NOT NULL,
    stale_reason TEXT
);

CREATE TABLE IF NOT EXISTS terminal_writer_generations (
    id TEXT PRIMARY KEY,
    process_id TEXT NOT NULL,
    lease_token TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('active', 'released', 'stale', 'force_released')),
    acquired_at_ms BIGINT NOT NULL,
    heartbeat_at_ms BIGINT NOT NULL,
    lease_expires_at_ms BIGINT NOT NULL,
    released_at_ms BIGINT,
    metadata_json TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_terminal_writer_generations_active
ON terminal_writer_generations(state)
WHERE state = 'active';

CREATE TABLE IF NOT EXISTS terminal_clock_anchors (
    id TEXT PRIMARY KEY,
    writer_generation TEXT NOT NULL,
    wall_time_ms BIGINT NOT NULL,
    monotonic_ms BIGINT NOT NULL,
    source TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS terminal_session_cursors (
    session_id TEXT PRIMARY KEY REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    next_commit_seq BIGINT NOT NULL DEFAULT 1 CHECK(next_commit_seq >= 1),
    writer_generation TEXT,
    updated_at_ms BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS terminal_commit_log (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    commit_seq BIGINT NOT NULL,
    commit_kind TEXT NOT NULL,
    writer_generation TEXT NOT NULL REFERENCES terminal_writer_generations(id) ON DELETE RESTRICT,
    occurred_at_ms BIGINT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    metadata_json TEXT,
    UNIQUE(session_id, commit_seq)
);

CREATE TABLE IF NOT EXISTS terminal_stream_cursors (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE CASCADE,
    stream_id TEXT NOT NULL,
    next_event_seq BIGINT NOT NULL DEFAULT 1 CHECK(next_event_seq >= 1),
    next_byte_seq BIGINT NOT NULL DEFAULT 0 CHECK(next_byte_seq >= 0),
    updated_at_ms BIGINT NOT NULL,
    UNIQUE(pane_id, stream_id)
);

CREATE INDEX IF NOT EXISTS idx_terminal_stream_cursors_session
ON terminal_stream_cursors(session_id);

CREATE TABLE IF NOT EXISTS terminal_topology_snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    high_water_commit_seq BIGINT NOT NULL,
    pane_high_water_json TEXT NOT NULL,
    topology_json TEXT NOT NULL,
    payload_schema_id TEXT REFERENCES terminal_payload_schemas(id) ON DELETE RESTRICT,
    checksum_algorithm TEXT NOT NULL DEFAULT 'blake3',
    checksum TEXT NOT NULL,
    source TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_stream_segments (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    stream_id TEXT NOT NULL,
    event_seq_low BIGINT NOT NULL CHECK(event_seq_low >= 1),
    event_seq_high BIGINT NOT NULL CHECK(event_seq_high >= event_seq_low),
    byte_low BIGINT NOT NULL CHECK(byte_low >= 0),
    byte_high BIGINT NOT NULL CHECK(byte_high > byte_low),
    payload BLOB NOT NULL,
    payload_len BIGINT NOT NULL CHECK(payload_len >= 0),
    stored_byte_len BIGINT NOT NULL CHECK(stored_byte_len >= 0),
    uncompressed_byte_len BIGINT,
    checksum_algorithm TEXT NOT NULL DEFAULT 'blake3',
    checksum TEXT NOT NULL,
    compression TEXT NOT NULL DEFAULT 'none',
    capture_semantics TEXT NOT NULL DEFAULT 'raw_vt_stream' CHECK(capture_semantics IN ('raw_vt_stream', 'rendered_ansi_stream', 'rendered_plaintext_snapshot', 'mux_structured_surface', 'imported_text', 'ui_input')),
    encryption_state TEXT NOT NULL DEFAULT 'plaintext' CHECK(encryption_state IN ('plaintext', 'encrypted', 'redacted')),
    key_ref TEXT,
    created_at_ms BIGINT NOT NULL,
    writer_generation TEXT NOT NULL,
    metadata_json TEXT,
    UNIQUE(pane_id, stream_id, event_seq_low),
    UNIQUE(pane_id, stream_id, byte_low)
);

CREATE INDEX IF NOT EXISTS idx_terminal_stream_segments_event_range
ON terminal_stream_segments(pane_id, stream_id, event_seq_low, event_seq_high);

CREATE TABLE IF NOT EXISTS terminal_journal_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    stream_id TEXT NOT NULL,
    event_scope_kind TEXT NOT NULL CHECK(event_scope_kind IN ('session', 'pane', 'stream')),
    event_scope_id TEXT NOT NULL,
    event_seq BIGINT NOT NULL CHECK(event_seq >= 1),
    event_type TEXT NOT NULL,
    byte_low BIGINT,
    byte_high BIGINT,
    payload_json TEXT,
    payload_schema_id TEXT REFERENCES terminal_payload_schemas(id) ON DELETE RESTRICT,
    source_event_id_hash TEXT,
    occurred_at_ms BIGINT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    capture_semantics TEXT NOT NULL DEFAULT 'raw_vt_stream' CHECK(capture_semantics IN ('raw_vt_stream', 'rendered_ansi_stream', 'rendered_plaintext_snapshot', 'mux_structured_surface', 'imported_text', 'ui_input')),
    trust_level TEXT NOT NULL DEFAULT 'unknown',
    metadata_json TEXT,
    UNIQUE(event_scope_kind, event_scope_id, stream_id, event_seq),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX IF NOT EXISTS idx_terminal_journal_events_pane_seq
ON terminal_journal_events(pane_id, stream_id, event_seq);

CREATE TABLE IF NOT EXISTS terminal_capture_receipts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    commit_id TEXT REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    source_kind TEXT NOT NULL,
    source_event_id_hash TEXT NOT NULL,
    source_payload_hash TEXT NOT NULL,
    received_at_ms BIGINT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    metadata_json TEXT,
    UNIQUE(session_id, source_kind, source_event_id_hash)
);

CREATE TABLE IF NOT EXISTS terminal_command_blocks (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    command_text TEXT,
    display_text TEXT,
    redacted_text TEXT,
    command_text_source TEXT NOT NULL,
    trust_level TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending_prompt', 'editing', 'submitted', 'running', 'finished', 'abandoned', 'unknown')),
    cwd TEXT,
    cwd_source TEXT,
    exit_code INTEGER,
    started_event_seq BIGINT,
    submitted_event_seq BIGINT,
    finished_event_seq BIGINT,
    output_event_seq_low BIGINT,
    output_event_seq_high BIGINT,
    output_byte_low BIGINT,
    output_byte_high BIGINT,
    sensitivity_class TEXT NOT NULL DEFAULT 'unknown',
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    metadata_json TEXT,
    CHECK((output_event_seq_low IS NULL AND output_event_seq_high IS NULL) OR (output_event_seq_low IS NOT NULL AND output_event_seq_high IS NOT NULL AND output_event_seq_low <= output_event_seq_high)),
    CHECK((output_byte_low IS NULL AND output_byte_high IS NULL) OR (output_byte_low IS NOT NULL AND output_byte_high IS NOT NULL AND output_byte_low < output_byte_high))
);

CREATE INDEX IF NOT EXISTS idx_terminal_command_blocks_session
ON terminal_command_blocks(session_id, created_at_ms);

CREATE TABLE IF NOT EXISTS terminal_command_history_entries (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE SET NULL,
    command_block_id TEXT REFERENCES terminal_command_blocks(id) ON DELETE SET NULL,
    scope_kind TEXT NOT NULL,
    command_text TEXT,
    display_text TEXT NOT NULL,
    redacted_text TEXT,
    command_hash_algorithm TEXT NOT NULL DEFAULT 'blake3',
    command_hash_scope TEXT NOT NULL DEFAULT 'local_keyed',
    command_hash TEXT NOT NULL,
    cwd TEXT,
    shell_kind TEXT,
    trust_level TEXT NOT NULL,
    source TEXT NOT NULL,
    sensitivity_class TEXT NOT NULL DEFAULT 'unknown',
    redaction_state TEXT NOT NULL DEFAULT 'unscanned',
    rerun_policy TEXT NOT NULL DEFAULT 'confirm',
    first_used_at_ms BIGINT NOT NULL,
    last_used_at_ms BIGINT NOT NULL,
    use_count BIGINT NOT NULL DEFAULT 1,
    metadata_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_terminal_command_history_scope
ON terminal_command_history_entries(scope_kind, last_used_at_ms DESC);

CREATE INDEX IF NOT EXISTS idx_terminal_command_history_session
ON terminal_command_history_entries(session_id, last_used_at_ms DESC);

CREATE TABLE IF NOT EXISTS terminal_screen_snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    projection_source TEXT NOT NULL,
    buffer_kind TEXT NOT NULL CHECK(buffer_kind IN ('normal', 'alternate', 'mux_surface', 'unknown')),
    rows INTEGER NOT NULL CHECK(rows > 0),
    cols INTEGER NOT NULL CHECK(cols > 0),
    base_event_seq BIGINT NOT NULL CHECK(base_event_seq >= 0),
    high_water_event_seq BIGINT NOT NULL CHECK(high_water_event_seq >= base_event_seq),
    high_water_byte_seq BIGINT,
    screen_json TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    projection_version TEXT NOT NULL,
    checksum_algorithm TEXT NOT NULL DEFAULT 'blake3',
    checksum TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_outbox_messages (
    id TEXT PRIMARY KEY,
    message_kind TEXT NOT NULL,
    dedupe_key TEXT,
    state TEXT NOT NULL CHECK(state IN ('pending', 'claimed', 'done', 'failed', 'quarantined')),
    payload_json TEXT NOT NULL,
    attempts BIGINT NOT NULL DEFAULT 0,
    max_attempts BIGINT NOT NULL DEFAULT 5,
    claimed_by TEXT,
    lease_token TEXT,
    claimed_until_ms BIGINT,
    next_run_at_ms BIGINT NOT NULL,
    last_error TEXT,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_terminal_outbox_dedupe
ON terminal_outbox_messages(dedupe_key)
WHERE dedupe_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS terminal_idempotency_keys (
    id TEXT PRIMARY KEY,
    scope_kind TEXT NOT NULL,
    scope_ref TEXT NOT NULL,
    operation_kind TEXT NOT NULL,
    idempotency_key_hash TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    result_json TEXT,
    state TEXT NOT NULL,
    first_seen_at_ms BIGINT NOT NULL,
    last_seen_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT NOT NULL,
    UNIQUE(scope_kind, scope_ref, operation_kind, idempotency_key_hash)
);

CREATE TABLE IF NOT EXISTS terminal_clients (
    id TEXT PRIMARY KEY,
    client_kind TEXT NOT NULL,
    install_ref_hash TEXT,
    browser_profile_ref_hash TEXT,
    user_agent_hash TEXT,
    created_at_ms BIGINT NOT NULL,
    last_seen_at_ms BIGINT NOT NULL,
    trust_state TEXT NOT NULL DEFAULT 'local_unverified'
);

CREATE TABLE IF NOT EXISTS terminal_delivery_offsets (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES terminal_clients(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE CASCADE,
    stream_id TEXT NOT NULL,
    last_sent_event_seq BIGINT NOT NULL DEFAULT 0,
    last_acked_event_seq BIGINT NOT NULL DEFAULT 0,
    last_persisted_event_seq BIGINT NOT NULL DEFAULT 0,
    replay_from_event_seq BIGINT,
    gap_state TEXT NOT NULL DEFAULT 'none',
    updated_at_ms BIGINT NOT NULL,
    UNIQUE(client_id, session_id, pane_id, stream_id)
);

CREATE TABLE IF NOT EXISTS terminal_history_gaps (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    stream_id TEXT NOT NULL,
    gap_kind TEXT NOT NULL,
    event_seq_low BIGINT,
    event_seq_high BIGINT,
    byte_low BIGINT,
    byte_high BIGINT,
    estimated_dropped_bytes BIGINT,
    estimated_dropped_events BIGINT,
    reason TEXT NOT NULL,
    writer_generation TEXT,
    opened_at_ms BIGINT NOT NULL,
    closed_at_ms BIGINT,
    metadata_json TEXT,
    CHECK((event_seq_low IS NULL AND event_seq_high IS NULL) OR (event_seq_low IS NOT NULL AND event_seq_high IS NOT NULL AND event_seq_low <= event_seq_high)),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX IF NOT EXISTS idx_terminal_history_gaps_session
ON terminal_history_gaps(session_id, opened_at_ms);

CREATE TABLE IF NOT EXISTS terminal_restore_drills (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    drill_kind TEXT NOT NULL,
    result TEXT NOT NULL CHECK(result IN ('passed', 'failed', 'degraded', 'skipped')),
    restore_guarantee_level TEXT NOT NULL,
    checked_at_ms BIGINT NOT NULL,
    duration_ms BIGINT,
    source_snapshot_id TEXT REFERENCES terminal_screen_snapshots(id) ON DELETE SET NULL,
    evidence_json TEXT,
    error TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_backup_records (
    id TEXT PRIMARY KEY,
    backup_kind TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending', 'running', 'succeeded', 'failed', 'quarantined')),
    target_ref_hash TEXT,
    manifest_json TEXT,
    checksum_algorithm TEXT,
    checksum TEXT,
    source_db_path_hash TEXT,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT,
    quick_check_result TEXT,
    error TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_storage_pressure_events (
    id TEXT PRIMARY KEY,
    state TEXT NOT NULL CHECK(state IN ('ok', 'warning', 'degraded', 'full', 'unknown')),
    db_file_bytes BIGINT,
    wal_file_bytes BIGINT,
    disk_free_bytes BIGINT,
    temp_free_bytes BIGINT,
    quota_bytes BIGINT,
    action_taken TEXT NOT NULL CHECK(action_taken IN ('none', 'warn_only', 'checkpoint_recommended', 'checkpoint_and_warn', 'degrade_with_gap', 'fail_closed')),
    reason TEXT,
    created_at_ms BIGINT NOT NULL,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_delete_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    request_kind TEXT NOT NULL,
    state TEXT NOT NULL,
    policy_id TEXT REFERENCES terminal_retention_policies(id) ON DELETE SET NULL,
    requested_at_ms BIGINT NOT NULL,
    approved_at_ms BIGINT,
    completed_at_ms BIGINT,
    requester_ref_hash TEXT,
    reason TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_deletion_tombstones (
    id TEXT PRIMARY KEY,
    delete_request_id TEXT REFERENCES terminal_delete_requests(id) ON DELETE SET NULL,
    session_id TEXT,
    deleted_scope TEXT NOT NULL,
    policy_id TEXT REFERENCES terminal_retention_policies(id) ON DELETE SET NULL,
    deleted_at_ms BIGINT NOT NULL,
    evidence_json TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_export_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    export_kind TEXT NOT NULL,
    state TEXT NOT NULL,
    redaction_profile_id TEXT,
    include_raw INTEGER NOT NULL DEFAULT 0 CHECK(include_raw IN (0, 1)),
    approved_at_ms BIGINT,
    requested_at_ms BIGINT NOT NULL,
    completed_at_ms BIGINT,
    manifest_json TEXT,
    output_ref_hash TEXT,
    error TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_support_bundles (
    id TEXT PRIMARY KEY,
    scope_json TEXT NOT NULL,
    state TEXT NOT NULL,
    redaction_profile_id TEXT,
    include_raw INTEGER NOT NULL DEFAULT 0 CHECK(include_raw IN (0, 1)),
    requested_at_ms BIGINT NOT NULL,
    completed_at_ms BIGINT,
    manifest_json TEXT,
    output_ref_hash TEXT,
    error TEXT,
    metadata_json TEXT
);

CREATE TABLE IF NOT EXISTS terminal_crypto_keys (
    id TEXT PRIMARY KEY,
    key_kind TEXT NOT NULL CHECK(key_kind IN ('database_key', 'export_key', 'artifact_key')),
    key_ref TEXT NOT NULL,
    protection_kind TEXT NOT NULL CHECK(protection_kind IN ('windows_credential_manager', 'dpapi_user', 'dpapi_machine', 'macos_keychain', 'linux_secret_service', 'test_plaintext')),
    state TEXT NOT NULL CHECK(state IN ('active', 'rotating', 'disabled', 'destroyed', 'unavailable')),
    created_at_ms BIGINT NOT NULL,
    rotated_at_ms BIGINT,
    destroyed_at_ms BIGINT,
    capability_report_json TEXT,
    error_json TEXT,
    metadata_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_terminal_crypto_keys_state
ON terminal_crypto_keys(state, created_at_ms DESC);

CREATE TABLE IF NOT EXISTS terminal_crypto_key_events (
    id TEXT PRIMARY KEY,
    key_id TEXT REFERENCES terminal_crypto_keys(id) ON DELETE SET NULL,
    event_kind TEXT NOT NULL CHECK(event_kind IN ('created', 'unlocked', 'lock_failed', 'rotated', 'destroy_requested', 'destroyed', 'recovery_failed')),
    actor TEXT NOT NULL,
    occurred_at_ms BIGINT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('succeeded', 'failed', 'skipped')),
    error_json TEXT,
    metadata_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_terminal_crypto_key_events_key_time
ON terminal_crypto_key_events(key_id, occurred_at_ms DESC);

CREATE TABLE IF NOT EXISTS terminal_external_artifacts (
    id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL CHECK(artifact_kind IN ('backup_file', 'large_segment', 'export_file', 'support_bundle', 'future_external_store')),
    artifact_ref_hash TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('planned', 'available', 'verified', 'missing', 'deleted', 'quarantined')),
    encryption_state TEXT NOT NULL CHECK(encryption_state IN ('plaintext', 'encrypted', 'redacted', 'crypto_erased')),
    key_ref TEXT,
    checksum_algorithm TEXT,
    checksum TEXT,
    size_bytes BIGINT,
    created_at_ms BIGINT NOT NULL,
    verified_at_ms BIGINT,
    metadata_json TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_terminal_external_artifacts_ref
ON terminal_external_artifacts(artifact_ref_hash);

CREATE TABLE IF NOT EXISTS terminal_search_documents (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE CASCADE,
    command_block_id TEXT REFERENCES terminal_command_blocks(id) ON DELETE SET NULL,
    document_kind TEXT NOT NULL,
    event_seq_low BIGINT,
    event_seq_high BIGINT,
    byte_low BIGINT,
    byte_high BIGINT,
    redaction_profile_id TEXT,
    redaction_state TEXT NOT NULL,
    source_hash_algorithm TEXT NOT NULL DEFAULT 'blake3',
    source_hash TEXT NOT NULL,
    text_preview TEXT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    metadata_json TEXT,
    CHECK((event_seq_low IS NULL AND event_seq_high IS NULL) OR (event_seq_low IS NOT NULL AND event_seq_high IS NOT NULL AND event_seq_low <= event_seq_high)),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX IF NOT EXISTS idx_terminal_search_documents_session
ON terminal_search_documents(session_id, updated_at_ms DESC);

CREATE TABLE IF NOT EXISTS terminal_legacy_migration_records (
    id TEXT PRIMARY KEY,
    legacy_table TEXT NOT NULL,
    legacy_session_id TEXT NOT NULL,
    new_session_id TEXT NOT NULL,
    migrated_at_ms BIGINT NOT NULL,
    migration_state TEXT NOT NULL,
    notes TEXT
);
