diesel::table! {
    terminal_db_identity (id) {
        id -> Integer,
        product -> Text,
        schema_family -> Text,
        created_at_ms -> BigInt,
        updated_at_ms -> BigInt,
        app_version -> Nullable<Text>,
        diesel_version -> Nullable<Text>,
        sqlite_version -> Nullable<Text>,
        notes -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_payload_schemas (id) {
        id -> Text,
        payload_kind -> Text,
        schema_version -> Text,
        schema_json -> Text,
        schema_hash -> Text,
        created_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_projection_versions (id) {
        id -> Text,
        projection_kind -> Text,
        version -> Text,
        parser_version -> Nullable<Text>,
        payload_schema_id -> Nullable<Text>,
        created_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_feature_gates (id) {
        id -> Text,
        feature_name -> Text,
        state -> Text,
        rollout_scope -> Text,
        reason -> Nullable<Text>,
        enabled_at_ms -> Nullable<BigInt>,
        disabled_at_ms -> Nullable<BigInt>,
        updated_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_retention_policies (id) {
        id -> Text,
        policy_kind -> Text,
        is_default -> Integer,
        max_bytes -> Nullable<BigInt>,
        max_age_ms -> Nullable<BigInt>,
        pressure_behavior -> Text,
        raw_history_prune_behavior -> Text,
        created_at_ms -> BigInt,
        updated_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_maintenance_runs (id) {
        id -> Text,
        run_kind -> Text,
        state -> Text,
        selected_policy_id -> Nullable<Text>,
        started_at_ms -> BigInt,
        finished_at_ms -> Nullable<BigInt>,
        summary_json -> Nullable<Text>,
        error -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_integrity_checks (id) {
        id -> Text,
        check_kind -> Text,
        scope_kind -> Text,
        scope_ref -> Nullable<Text>,
        result -> Text,
        checked_at_ms -> BigInt,
        details_json -> Nullable<Text>,
        error -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_data_health_records (id) {
        id -> Text,
        session_id -> Nullable<Text>,
        pane_id -> Nullable<Text>,
        detection_kind -> Text,
        severity -> Text,
        first_bad_event_seq -> Nullable<BigInt>,
        affected_ref -> Nullable<Text>,
        action_state -> Text,
        detected_at_ms -> BigInt,
        resolved_at_ms -> Nullable<BigInt>,
        details_json -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_sessions (id) {
        id -> Text,
        route_json -> Text,
        title -> Nullable<Text>,
        launch_json -> Nullable<Text>,
        source -> Text,
        durability_profile -> Text,
        retention_policy_id -> Text,
        private_mode -> Integer,
        created_at_ms -> BigInt,
        updated_at_ms -> BigInt,
        closed_at_ms -> Nullable<BigInt>,
        state -> Text,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_panes (id) {
        id -> Text,
        session_id -> Text,
        tab_id -> Nullable<Text>,
        stream_id -> Text,
        title -> Nullable<Text>,
        rows -> Integer,
        cols -> Integer,
        last_event_seq -> BigInt,
        created_at_ms -> BigInt,
        closed_at_ms -> Nullable<BigInt>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_backend_capability_reports (id) {
        id -> Text,
        session_id -> Nullable<Text>,
        backend_kind -> Text,
        backend_version -> Nullable<Text>,
        backend_binary_path_hash -> Nullable<Text>,
        route_kind -> Text,
        probe_status -> Text,
        capture_strategy -> Text,
        capture_semantics -> Text,
        can_preserve_process_when_live -> Integer,
        can_capture_scrollback -> Integer,
        command_boundary_confidence -> Text,
        evidence_json -> Nullable<Text>,
        created_at_ms -> BigInt,
        expires_at_ms -> BigInt,
        stale_reason -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_writer_generations (id) {
        id -> Text,
        process_id -> Text,
        lease_token -> Text,
        state -> Text,
        acquired_at_ms -> BigInt,
        heartbeat_at_ms -> BigInt,
        lease_expires_at_ms -> BigInt,
        released_at_ms -> Nullable<BigInt>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_clock_anchors (id) {
        id -> Text,
        writer_generation -> Text,
        wall_time_ms -> BigInt,
        monotonic_ms -> BigInt,
        source -> Text,
        created_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_session_cursors (session_id) {
        session_id -> Text,
        next_commit_seq -> BigInt,
        writer_generation -> Nullable<Text>,
        updated_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_stream_cursors (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Text,
        stream_id -> Text,
        next_event_seq -> BigInt,
        next_byte_seq -> BigInt,
        updated_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_commit_log (id) {
        id -> Text,
        session_id -> Text,
        commit_seq -> BigInt,
        commit_kind -> Text,
        writer_generation -> Text,
        occurred_at_ms -> BigInt,
        created_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_stream_segments (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Text,
        commit_id -> Text,
        stream_id -> Text,
        event_seq_low -> BigInt,
        event_seq_high -> BigInt,
        byte_low -> BigInt,
        byte_high -> BigInt,
        payload -> Binary,
        payload_len -> BigInt,
        stored_byte_len -> BigInt,
        uncompressed_byte_len -> Nullable<BigInt>,
        checksum_algorithm -> Text,
        checksum -> Text,
        compression -> Text,
        capture_semantics -> Text,
        encryption_state -> Text,
        key_ref -> Nullable<Text>,
        created_at_ms -> BigInt,
        writer_generation -> Text,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_journal_events (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Nullable<Text>,
        commit_id -> Text,
        stream_id -> Text,
        event_scope_kind -> Text,
        event_scope_id -> Text,
        event_seq -> BigInt,
        event_type -> Text,
        byte_low -> Nullable<BigInt>,
        byte_high -> Nullable<BigInt>,
        payload_json -> Nullable<Text>,
        payload_schema_id -> Nullable<Text>,
        source_event_id_hash -> Nullable<Text>,
        occurred_at_ms -> BigInt,
        created_at_ms -> BigInt,
        capture_semantics -> Text,
        trust_level -> Text,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_capture_receipts (id) {
        id -> Text,
        session_id -> Text,
        commit_id -> Nullable<Text>,
        source_kind -> Text,
        source_event_id_hash -> Text,
        source_payload_hash -> Text,
        received_at_ms -> BigInt,
        created_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_idempotency_keys (id) {
        id -> Text,
        scope_kind -> Text,
        scope_ref -> Text,
        operation_kind -> Text,
        idempotency_key_hash -> Text,
        request_hash -> Text,
        result_json -> Nullable<Text>,
        state -> Text,
        first_seen_at_ms -> BigInt,
        last_seen_at_ms -> BigInt,
        expires_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_clients (id) {
        id -> Text,
        client_kind -> Text,
        install_ref_hash -> Nullable<Text>,
        browser_profile_ref_hash -> Nullable<Text>,
        user_agent_hash -> Nullable<Text>,
        created_at_ms -> BigInt,
        last_seen_at_ms -> BigInt,
        trust_state -> Text,
    }
}

diesel::table! {
    terminal_delivery_offsets (id) {
        id -> Text,
        client_id -> Text,
        session_id -> Text,
        pane_id -> Nullable<Text>,
        stream_id -> Text,
        last_sent_event_seq -> BigInt,
        last_acked_event_seq -> BigInt,
        last_persisted_event_seq -> BigInt,
        replay_from_event_seq -> Nullable<BigInt>,
        gap_state -> Text,
        updated_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_outbox_messages (id) {
        id -> Text,
        message_kind -> Text,
        dedupe_key -> Nullable<Text>,
        state -> Text,
        payload_json -> Text,
        attempts -> BigInt,
        max_attempts -> BigInt,
        claimed_by -> Nullable<Text>,
        lease_token -> Nullable<Text>,
        claimed_until_ms -> Nullable<BigInt>,
        next_run_at_ms -> BigInt,
        last_error -> Nullable<Text>,
        created_at_ms -> BigInt,
        updated_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_command_blocks (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Text,
        commit_id -> Nullable<Text>,
        command_text -> Nullable<Text>,
        display_text -> Nullable<Text>,
        redacted_text -> Nullable<Text>,
        command_text_source -> Text,
        trust_level -> Text,
        state -> Text,
        cwd -> Nullable<Text>,
        cwd_source -> Nullable<Text>,
        exit_code -> Nullable<Integer>,
        started_event_seq -> Nullable<BigInt>,
        submitted_event_seq -> Nullable<BigInt>,
        finished_event_seq -> Nullable<BigInt>,
        output_event_seq_low -> Nullable<BigInt>,
        output_event_seq_high -> Nullable<BigInt>,
        output_byte_low -> Nullable<BigInt>,
        output_byte_high -> Nullable<BigInt>,
        sensitivity_class -> Text,
        created_at_ms -> BigInt,
        updated_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_command_history_entries (id) {
        id -> Text,
        session_id -> Nullable<Text>,
        pane_id -> Nullable<Text>,
        command_block_id -> Nullable<Text>,
        scope_kind -> Text,
        command_text -> Nullable<Text>,
        display_text -> Text,
        redacted_text -> Nullable<Text>,
        command_hash_algorithm -> Text,
        command_hash_scope -> Text,
        command_hash -> Text,
        cwd -> Nullable<Text>,
        shell_kind -> Nullable<Text>,
        trust_level -> Text,
        source -> Text,
        sensitivity_class -> Text,
        redaction_state -> Text,
        rerun_policy -> Text,
        first_used_at_ms -> BigInt,
        last_used_at_ms -> BigInt,
        use_count -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_topology_snapshots (id) {
        id -> Text,
        session_id -> Text,
        commit_id -> Text,
        high_water_commit_seq -> BigInt,
        pane_high_water_json -> Text,
        topology_json -> Text,
        payload_schema_id -> Nullable<Text>,
        checksum_algorithm -> Text,
        checksum -> Text,
        source -> Text,
        created_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_screen_snapshots (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Text,
        commit_id -> Text,
        projection_source -> Text,
        buffer_kind -> Text,
        rows -> Integer,
        cols -> Integer,
        base_event_seq -> BigInt,
        high_water_event_seq -> BigInt,
        high_water_byte_seq -> Nullable<BigInt>,
        screen_json -> Text,
        parser_version -> Text,
        projection_version -> Text,
        checksum_algorithm -> Text,
        checksum -> Text,
        created_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_history_gaps (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Nullable<Text>,
        stream_id -> Text,
        gap_kind -> Text,
        event_seq_low -> Nullable<BigInt>,
        event_seq_high -> Nullable<BigInt>,
        byte_low -> Nullable<BigInt>,
        byte_high -> Nullable<BigInt>,
        estimated_dropped_bytes -> Nullable<BigInt>,
        estimated_dropped_events -> Nullable<BigInt>,
        reason -> Text,
        writer_generation -> Nullable<Text>,
        opened_at_ms -> BigInt,
        closed_at_ms -> Nullable<BigInt>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_restore_drills (id) {
        id -> Text,
        session_id -> Text,
        drill_kind -> Text,
        result -> Text,
        restore_guarantee_level -> Text,
        checked_at_ms -> BigInt,
        duration_ms -> Nullable<BigInt>,
        source_snapshot_id -> Nullable<Text>,
        evidence_json -> Nullable<Text>,
        error -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_backup_records (id) {
        id -> Text,
        backup_kind -> Text,
        state -> Text,
        target_ref_hash -> Nullable<Text>,
        manifest_json -> Nullable<Text>,
        checksum_algorithm -> Nullable<Text>,
        checksum -> Nullable<Text>,
        source_db_path_hash -> Nullable<Text>,
        started_at_ms -> BigInt,
        finished_at_ms -> Nullable<BigInt>,
        quick_check_result -> Nullable<Text>,
        error -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_storage_pressure_events (id) {
        id -> Text,
        state -> Text,
        db_file_bytes -> Nullable<BigInt>,
        wal_file_bytes -> Nullable<BigInt>,
        disk_free_bytes -> Nullable<BigInt>,
        temp_free_bytes -> Nullable<BigInt>,
        quota_bytes -> Nullable<BigInt>,
        action_taken -> Text,
        reason -> Nullable<Text>,
        created_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_delete_requests (id) {
        id -> Text,
        session_id -> Nullable<Text>,
        request_kind -> Text,
        state -> Text,
        policy_id -> Nullable<Text>,
        requested_at_ms -> BigInt,
        approved_at_ms -> Nullable<BigInt>,
        completed_at_ms -> Nullable<BigInt>,
        requester_ref_hash -> Nullable<Text>,
        reason -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_deletion_tombstones (id) {
        id -> Text,
        delete_request_id -> Nullable<Text>,
        session_id -> Nullable<Text>,
        deleted_scope -> Text,
        policy_id -> Nullable<Text>,
        deleted_at_ms -> BigInt,
        evidence_json -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_export_requests (id) {
        id -> Text,
        session_id -> Nullable<Text>,
        export_kind -> Text,
        state -> Text,
        redaction_profile_id -> Nullable<Text>,
        include_raw -> Integer,
        approved_at_ms -> Nullable<BigInt>,
        requested_at_ms -> BigInt,
        completed_at_ms -> Nullable<BigInt>,
        manifest_json -> Nullable<Text>,
        output_ref_hash -> Nullable<Text>,
        error -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_support_bundles (id) {
        id -> Text,
        scope_json -> Text,
        state -> Text,
        redaction_profile_id -> Nullable<Text>,
        include_raw -> Integer,
        requested_at_ms -> BigInt,
        completed_at_ms -> Nullable<BigInt>,
        manifest_json -> Nullable<Text>,
        output_ref_hash -> Nullable<Text>,
        error -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_crypto_keys (id) {
        id -> Text,
        key_kind -> Text,
        key_ref -> Text,
        protection_kind -> Text,
        state -> Text,
        created_at_ms -> BigInt,
        rotated_at_ms -> Nullable<BigInt>,
        destroyed_at_ms -> Nullable<BigInt>,
        capability_report_json -> Nullable<Text>,
        error_json -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_crypto_key_events (id) {
        id -> Text,
        key_id -> Nullable<Text>,
        event_kind -> Text,
        actor -> Text,
        occurred_at_ms -> BigInt,
        status -> Text,
        error_json -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_external_artifacts (id) {
        id -> Text,
        artifact_kind -> Text,
        artifact_ref_hash -> Text,
        state -> Text,
        encryption_state -> Text,
        key_ref -> Nullable<Text>,
        checksum_algorithm -> Nullable<Text>,
        checksum -> Nullable<Text>,
        size_bytes -> Nullable<BigInt>,
        created_at_ms -> BigInt,
        verified_at_ms -> Nullable<BigInt>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_search_documents (rowid) {
        rowid -> Integer,
        document_id -> Text,
        session_id -> Text,
        pane_id -> Nullable<Text>,
        command_block_id -> Nullable<Text>,
        document_kind -> Text,
        event_seq_low -> Nullable<BigInt>,
        event_seq_high -> Nullable<BigInt>,
        byte_low -> Nullable<BigInt>,
        byte_high -> Nullable<BigInt>,
        redaction_profile_id -> Nullable<Text>,
        redaction_state -> Text,
        source_hash_algorithm -> Text,
        source_hash -> Text,
        text_preview -> Text,
        updated_at_ms -> BigInt,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_ai_context_packages (id) {
        id -> Text,
        session_id -> Nullable<Text>,
        pane_id -> Nullable<Text>,
        state -> Text,
        redaction_profile_id -> Nullable<Text>,
        include_raw -> Integer,
        requested_at_ms -> BigInt,
        built_at_ms -> Nullable<BigInt>,
        item_count -> BigInt,
        manifest_json -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_ai_context_items (id) {
        id -> Text,
        package_id -> Text,
        source_kind -> Text,
        source_ref -> Nullable<Text>,
        session_id -> Nullable<Text>,
        pane_id -> Nullable<Text>,
        command_block_id -> Nullable<Text>,
        event_seq_low -> Nullable<BigInt>,
        event_seq_high -> Nullable<BigInt>,
        byte_low -> Nullable<BigInt>,
        byte_high -> Nullable<BigInt>,
        redaction_state -> Text,
        data_only -> Integer,
        content_preview -> Text,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_prompt_injection_findings (id) {
        id -> Text,
        package_id -> Nullable<Text>,
        item_id -> Nullable<Text>,
        severity -> Text,
        pattern_kind -> Text,
        action_state -> Text,
        detected_at_ms -> BigInt,
        evidence_preview -> Text,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_ai_action_approvals (id) {
        id -> Text,
        package_id -> Nullable<Text>,
        action_kind -> Text,
        state -> Text,
        requester_ref_hash -> Nullable<Text>,
        approver_ref_hash -> Nullable<Text>,
        requested_at_ms -> BigInt,
        decided_at_ms -> Nullable<BigInt>,
        expires_at_ms -> Nullable<BigInt>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    terminal_legacy_migration_records (id) {
        id -> Text,
        legacy_table -> Text,
        legacy_session_id -> Text,
        new_session_id -> Text,
        migrated_at_ms -> BigInt,
        migration_state -> Text,
        notes -> Nullable<Text>,
    }
}

diesel::joinable!(terminal_panes -> terminal_sessions (session_id));
diesel::joinable!(terminal_sessions -> terminal_retention_policies (retention_policy_id));
diesel::joinable!(terminal_data_health_records -> terminal_sessions (session_id));
diesel::joinable!(terminal_projection_versions -> terminal_payload_schemas (payload_schema_id));
diesel::joinable!(terminal_maintenance_runs -> terminal_retention_policies (selected_policy_id));
diesel::joinable!(terminal_crypto_key_events -> terminal_crypto_keys (key_id));
diesel::joinable!(terminal_delivery_offsets -> terminal_clients (client_id));
diesel::joinable!(terminal_delivery_offsets -> terminal_sessions (session_id));
diesel::joinable!(terminal_stream_cursors -> terminal_panes (pane_id));
diesel::joinable!(terminal_stream_cursors -> terminal_sessions (session_id));
diesel::joinable!(terminal_session_cursors -> terminal_sessions (session_id));
diesel::joinable!(terminal_ai_context_packages -> terminal_sessions (session_id));
diesel::joinable!(terminal_ai_context_packages -> terminal_panes (pane_id));
diesel::joinable!(terminal_ai_context_items -> terminal_ai_context_packages (package_id));
diesel::joinable!(terminal_ai_context_items -> terminal_sessions (session_id));
diesel::joinable!(terminal_ai_context_items -> terminal_panes (pane_id));
diesel::joinable!(terminal_ai_context_items -> terminal_command_blocks (command_block_id));
diesel::joinable!(terminal_prompt_injection_findings -> terminal_ai_context_packages (package_id));
diesel::joinable!(terminal_prompt_injection_findings -> terminal_ai_context_items (item_id));
diesel::joinable!(terminal_ai_action_approvals -> terminal_ai_context_packages (package_id));

diesel::allow_tables_to_appear_in_same_query!(
    terminal_db_identity,
    terminal_payload_schemas,
    terminal_projection_versions,
    terminal_feature_gates,
    terminal_retention_policies,
    terminal_maintenance_runs,
    terminal_integrity_checks,
    terminal_data_health_records,
    terminal_sessions,
    terminal_panes,
    terminal_backend_capability_reports,
    terminal_writer_generations,
    terminal_clock_anchors,
    terminal_session_cursors,
    terminal_stream_cursors,
    terminal_commit_log,
    terminal_stream_segments,
    terminal_journal_events,
    terminal_capture_receipts,
    terminal_idempotency_keys,
    terminal_clients,
    terminal_delivery_offsets,
    terminal_outbox_messages,
    terminal_command_blocks,
    terminal_command_history_entries,
    terminal_topology_snapshots,
    terminal_screen_snapshots,
    terminal_history_gaps,
    terminal_restore_drills,
    terminal_backup_records,
    terminal_storage_pressure_events,
    terminal_delete_requests,
    terminal_deletion_tombstones,
    terminal_export_requests,
    terminal_support_bundles,
    terminal_crypto_keys,
    terminal_crypto_key_events,
    terminal_external_artifacts,
    terminal_search_documents,
    terminal_ai_context_packages,
    terminal_ai_context_items,
    terminal_prompt_injection_findings,
    terminal_ai_action_approvals,
    terminal_legacy_migration_records,
);
