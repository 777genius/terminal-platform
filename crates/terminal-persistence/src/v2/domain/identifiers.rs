use super::super::*;
use super::*;

pub(in crate::v2) fn stream_cursor_id(pane_id: &str, stream_id: &str) -> String {
    format!("stream-cursor-{}", blake3_hash_text(&format!("{pane_id}\0{stream_id}")))
}

pub(in crate::v2) fn stream_capture_source_kind(pane_id: &str, stream_id: &str) -> String {
    format!("stream-segment-{}", blake3_hash_text(&format!("{pane_id}\0{stream_id}")))
}

pub(in crate::v2) fn payload_schema_id_for_journal_event(event_type: &str) -> &'static str {
    match event_type {
        "terminal_input" | "terminal_paste_input" => PAYLOAD_SCHEMA_UI_INPUT_V1,
        "history_gap" => PAYLOAD_SCHEMA_HISTORY_GAP_V1,
        _ => PAYLOAD_SCHEMA_JOURNAL_EVENT_V1,
    }
}

pub(in crate::v2) fn delivery_offset_id(
    client_id: &str,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> String {
    format!(
        "delivery-offset-{}",
        blake3_hash_text(&format!("{client_id}\0{session_id}\0{pane_id}\0{stream_id}"))
    )
}

pub(in crate::v2) fn normalize_outbox_dedupe_key(value: &str) -> String {
    format!("blake3:{}", blake3_hash_text(value))
}

pub(in crate::v2) fn ui_input_capture_source_kind(pane_id: &str) -> String {
    format!("ui-input-{}", blake3_hash_text(pane_id))
}

pub(in crate::v2) fn stable_ui_command_block_id(
    session_id: &str,
    pane_id: &str,
    source_event_id_hash: &str,
) -> String {
    format!(
        "command-block-{}",
        blake3_hash_text(&format!("{session_id}\0{pane_id}\0{source_event_id_hash}"))
    )
}

pub(in crate::v2) fn stable_history_id(
    scope_kind: &str,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    command_hash: &str,
) -> String {
    let material = format!(
        "{}\0{}\0{}\0{}",
        scope_kind,
        session_id.unwrap_or_default(),
        pane_id.unwrap_or_default(),
        command_hash
    );
    format!("command-history-{}", blake3_hash_text(&material))
}

pub(in crate::v2) fn stable_search_document_id(
    session_id: &str,
    pane_id: Option<&str>,
    command_block_id: Option<&str>,
    source_hash: &str,
) -> String {
    let material = format!(
        "{}\0{}\0{}\0{}",
        session_id,
        pane_id.unwrap_or_default(),
        command_block_id.unwrap_or_default(),
        source_hash
    );
    format!("search-document-{}", blake3_hash_text(&material))
}

pub(in crate::v2) fn command_text_from_ui_input(data: &str) -> Option<String> {
    let trimmed_end = data.trim_end_matches(['\r', '\n']);
    if trimmed_end.len() == data.len() {
        return None;
    }
    let command = trimmed_end.trim();
    if command.is_empty() { None } else { Some(command.to_string()) }
}
