mod transaction;

use super::super::*;

impl TerminalPersistenceV2 {
    pub fn record_ui_input_event(
        &self,
        input: UiInputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.record_ui_input_event_with_connection(&mut connection, input)
    }

    pub(crate) fn record_ui_input_event_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: UiInputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        self.upsert_runtime_session_with_connection(
            connection,
            SessionInput {
                id: Some(input.session_id.clone()),
                route: input.route.clone(),
                title: input.title.clone(),
                launch: input.launch.clone(),
                source: Some("runtime_ui_input".to_string()),
                durability_profile: None,
                retention_policy_id: None,
                private_mode: false,
                metadata: Some(serde_json::json!({
                    "capture_source": "ui_input",
                    "trusted_command_source": true
                })),
            },
        )?;
        self.upsert_runtime_pane_with_connection(
            connection,
            PaneInput {
                id: Some(input.pane_id.clone()),
                session_id: input.session_id.clone(),
                tab_id: None,
                stream_id: None,
                title: None,
                rows: input.rows.unwrap_or(24),
                cols: input.cols.unwrap_or(80),
                metadata: Some(serde_json::json!({
                    "capture_source": "ui_input",
                    "dimensions": if input.rows.is_some() && input.cols.is_some() {
                        "observed"
                    } else {
                        "provisional"
                    }
                })),
            },
        )?;
        if self.is_session_private_with_connection(connection, &input.session_id)? {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "private mode suppresses durable ui input history".to_string(),
            ));
        }

        let lease = self.acquire_writer_generation_with_retry_on_connection(
            connection,
            "runtime-ui-input",
            60_000,
        )?;
        let event_result =
            self.append_ui_input_event_and_command_with_connection(connection, &input, &lease.id);
        let release_result = self.release_writer_generation_with_connection(connection, &lease.id);

        match (event_result, release_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }
}
