use super::super::super::super::*;

pub(super) struct UiInputTransaction<'a> {
    pub(super) input: &'a UiInputEventInput,
    pub(super) now: i64,
    pub(super) stream_id: String,
    pub(super) event_type: &'static str,
    pub(super) payload_json: String,
    pub(super) payload_hash: String,
    pub(super) source_event_id_hash: Option<String>,
    pub(super) capture_source_kind: Option<String>,
    pub(super) command_text: Option<String>,
    pub(super) shell_profile: ShellMetadataProfile,
    pub(super) command_metadata_json: Option<String>,
}

impl<'a> UiInputTransaction<'a> {
    pub(super) fn new(
        input: &'a UiInputEventInput,
        now: i64,
    ) -> Result<Self, TerminalPersistenceV2Error> {
        let payload_json = serde_json::to_string(&serde_json::json!({
            "data": input.data.clone(),
            "is_paste": input.is_paste
        }))?;
        let payload_hash = blake3_hash_text(&payload_json);
        let source_event_id_hash = input.source_event_id.as_ref().map(|source_event_id| {
            blake3_hash_text(&format!("ui-input-client-event:{source_event_id}"))
        });
        let capture_source_kind =
            source_event_id_hash.as_ref().map(|_| ui_input_capture_source_kind(&input.pane_id));
        let shell_profile =
            shell_metadata_profile(input.launch.as_ref(), input.shell_kind.as_deref());
        let command_metadata_json = Some(serde_json::to_string(&serde_json::json!({
            "capture_source": "ui_input",
            "rerun_policy": "confirm",
            "shell_profile": shell_profile
        }))?);

        Ok(Self {
            input,
            now,
            stream_id: DEFAULT_STREAM_ID.to_string(),
            event_type: if input.is_paste { "terminal_paste_input" } else { "terminal_input" },
            payload_json,
            payload_hash,
            source_event_id_hash,
            capture_source_kind,
            command_text: if input.is_paste {
                None
            } else {
                command_text_from_ui_input(&input.data)
            },
            shell_profile,
            command_metadata_json,
        })
    }
}
