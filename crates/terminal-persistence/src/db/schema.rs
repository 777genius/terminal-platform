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

diesel::joinable!(terminal_panes -> terminal_sessions (session_id));
diesel::joinable!(terminal_sessions -> terminal_retention_policies (retention_policy_id));
diesel::joinable!(terminal_delivery_offsets -> terminal_clients (client_id));
diesel::joinable!(terminal_delivery_offsets -> terminal_sessions (session_id));
diesel::joinable!(terminal_stream_cursors -> terminal_panes (pane_id));
diesel::joinable!(terminal_stream_cursors -> terminal_sessions (session_id));
diesel::joinable!(terminal_session_cursors -> terminal_sessions (session_id));

diesel::allow_tables_to_appear_in_same_query!(
    terminal_db_identity,
    terminal_feature_gates,
    terminal_retention_policies,
    terminal_integrity_checks,
    terminal_sessions,
    terminal_panes,
    terminal_backend_capability_reports,
    terminal_writer_generations,
    terminal_session_cursors,
    terminal_stream_cursors,
    terminal_commit_log,
    terminal_stream_segments,
    terminal_journal_events,
    terminal_capture_receipts,
    terminal_clients,
    terminal_delivery_offsets,
    terminal_command_blocks,
    terminal_command_history_entries,
    terminal_topology_snapshots,
    terminal_screen_snapshots,
    terminal_history_gaps,
    terminal_restore_drills,
    terminal_backup_records,
);
