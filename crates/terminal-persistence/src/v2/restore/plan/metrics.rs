use super::super::super::*;

pub(super) struct RestorePlanInputs {
    pub(super) latest_topology: Option<TopologySnapshotRow>,
    pub(super) latest_screen: Option<ScreenSnapshotRow>,
    pub(super) segment_count: i64,
    pub(super) raw_segment_count: i64,
    pub(super) rendered_segment_count: i64,
    pub(super) stream_event_range: (Option<i64>, Option<i64>),
    pub(super) gap_count: i64,
    pub(super) high_water_commit_seq: i64,
    pub(super) latest_restore_drill: Option<(String, String)>,
    pub(super) latest_restore_drill_status: Option<String>,
    pub(super) authoritative_reads_gate: String,
    pub(super) latest_capability_report: Option<BackendCapabilityReportRow>,
    pub(super) critical_health_record_count: i64,
}

pub(super) fn load_restore_plan_inputs(
    connection: &mut SqliteConnection,
    session_id: &str,
    now: i64,
) -> Result<RestorePlanInputs, TerminalPersistenceV2Error> {
    let latest_topology =
        load_latest_valid_topology_snapshot(connection, session_id, now, "restore_plan")?;
    let topology_pane_high_water = latest_topology
        .as_ref()
        .map(|topology| parse_pane_high_water_json(&topology.pane_high_water_json))
        .transpose()?;
    let latest_screen = load_latest_valid_screen_snapshot(
        connection,
        session_id,
        None,
        topology_pane_high_water.as_ref(),
        now,
        "restore_plan",
    )?;

    let segment_count = count_stream_segments(connection, session_id)?;
    let raw_segment_count = count_raw_stream_segments(connection, session_id)?;
    let rendered_segment_count = segment_count - raw_segment_count;

    let latest_restore_drill = load_latest_restore_drill(connection, session_id)?;
    let latest_restore_drill_status =
        latest_restore_drill.as_ref().map(|(_, result)| result.clone());

    Ok(RestorePlanInputs {
        latest_topology,
        latest_screen,
        segment_count,
        raw_segment_count,
        rendered_segment_count,
        stream_event_range: load_stream_event_range(connection, session_id)?,
        gap_count: load_gap_count(connection, session_id)?,
        high_water_commit_seq: load_high_water_commit_seq(connection, session_id)?,
        latest_restore_drill,
        latest_restore_drill_status,
        authoritative_reads_gate: load_authoritative_reads_gate(connection)?,
        latest_capability_report: latest_backend_capability_report(connection, session_id)?,
        critical_health_record_count: count_critical_health_records(connection, session_id)?,
    })
}

fn count_stream_segments(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .count()
        .get_result(connection)?)
}

fn count_raw_stream_segments(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::capture_semantics.eq("raw_vt_stream"))
        .count()
        .get_result(connection)?)
}

fn load_stream_event_range(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<(Option<i64>, Option<i64>), TerminalPersistenceV2Error> {
    Ok(terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .select((
            diesel::dsl::min(terminal_stream_segments::event_seq_low),
            diesel::dsl::max(terminal_stream_segments::event_seq_high),
        ))
        .first(connection)?)
}

fn load_gap_count(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    let persisted_gap_count: i64 = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .count()
        .get_result(connection)?;
    let journal_gap_count: i64 = terminal_journal_events::table
        .filter(terminal_journal_events::session_id.eq(session_id))
        .filter(terminal_journal_events::event_type.eq("history_gap"))
        .count()
        .get_result(connection)?;
    Ok(persisted_gap_count.max(journal_gap_count))
}

fn load_high_water_commit_seq(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_commit_log::table
        .filter(terminal_commit_log::session_id.eq(session_id))
        .select(diesel::dsl::max(terminal_commit_log::commit_seq))
        .first::<Option<i64>>(connection)?
        .unwrap_or(0))
}

fn load_latest_restore_drill(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<Option<(String, String)>, TerminalPersistenceV2Error> {
    Ok(terminal_restore_drills::table
        .filter(terminal_restore_drills::session_id.eq(session_id))
        .order(terminal_restore_drills::checked_at_ms.desc())
        .select((terminal_restore_drills::id, terminal_restore_drills::result))
        .first(connection)
        .optional()?)
}

fn load_authoritative_reads_gate(
    connection: &mut SqliteConnection,
) -> Result<String, TerminalPersistenceV2Error> {
    Ok(terminal_feature_gates::table
        .filter(
            terminal_feature_gates::feature_name
                .eq(FeatureGateName::TerminalPersistenceV2AuthoritativeReads.as_str()),
        )
        .select(terminal_feature_gates::state)
        .first(connection)
        .optional()?
        .unwrap_or_else(|| FeatureGateState::Disabled.as_str().to_string()))
}

fn count_critical_health_records(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_data_health_records::table
        .filter(
            terminal_data_health_records::session_id
                .eq(Some(session_id.to_string()))
                .or(terminal_data_health_records::session_id.is_null()),
        )
        .filter(terminal_data_health_records::severity.eq("critical"))
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .count()
        .get_result(connection)?)
}
