use super::super::super::*;

pub(super) fn upsert_terminal_output_target_with_connection(
    store: &TerminalPersistenceV2,
    connection: &mut SqliteConnection,
    input: &TerminalOutputEventInput,
) -> Result<(), TerminalPersistenceV2Error> {
    store.upsert_runtime_session_with_connection(
        connection,
        SessionInput {
            id: Some(input.session_id.clone()),
            route: input.route.clone(),
            title: input.title.clone(),
            launch: input.launch.clone(),
            source: Some("runtime_output_capture".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: Some(serde_json::json!({
                "capture_source": "backend_output",
                "capture_semantics": input.capture_semantics
                    .as_deref()
                    .unwrap_or("raw_vt_stream")
            })),
        },
    )?;
    store.upsert_runtime_pane_with_connection(
        connection,
        PaneInput {
            id: Some(input.pane_id.clone()),
            session_id: input.session_id.clone(),
            tab_id: input.tab_id.clone(),
            stream_id: None,
            title: None,
            rows: input.rows.unwrap_or(24),
            cols: input.cols.unwrap_or(80),
            metadata: Some(serde_json::json!({
                "capture_source": "backend_output",
                "dimensions": if input.rows.is_some() && input.cols.is_some() {
                    "observed"
                } else {
                    "provisional"
                }
            })),
        },
    )?;
    Ok(())
}

pub(super) fn upsert_history_gap_target_with_connection(
    store: &TerminalPersistenceV2,
    connection: &mut SqliteConnection,
    input: &HistoryGapEventInput,
) -> Result<(), TerminalPersistenceV2Error> {
    store.upsert_runtime_session_with_connection(
        connection,
        SessionInput {
            id: Some(input.session_id.clone()),
            route: input.route.clone(),
            title: input.title.clone(),
            launch: input.launch.clone(),
            source: Some("runtime_output_capture".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: Some(serde_json::json!({ "capture_source": "backend_output_gap" })),
        },
    )?;
    store.upsert_runtime_pane_with_connection(
        connection,
        PaneInput {
            id: Some(input.pane_id.clone()),
            session_id: input.session_id.clone(),
            tab_id: input.tab_id.clone(),
            stream_id: None,
            title: None,
            rows: input.rows.unwrap_or(24),
            cols: input.cols.unwrap_or(80),
            metadata: Some(serde_json::json!({ "capture_source": "backend_output_gap" })),
        },
    )?;
    Ok(())
}

pub(super) fn finish_writer_operation<T>(
    operation_result: Result<T, TerminalPersistenceV2Error>,
    release_result: Result<(), TerminalPersistenceV2Error>,
) -> Result<T, TerminalPersistenceV2Error> {
    match (operation_result, release_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) => Err(error),
        (Err(error), _) => Err(error),
    }
}
