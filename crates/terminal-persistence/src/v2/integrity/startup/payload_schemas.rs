use super::super::super::*;

pub(in crate::v2) fn seed_payload_schemas(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let rows = vec![
        payload_schema_row(
            PAYLOAD_SCHEMA_UI_INPUT_V1,
            "journal_event_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Terminal UI input journal payload",
                "type": "object",
                "required": ["data", "is_paste"],
                "properties": {
                    "data": { "type": "string" },
                    "is_paste": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
            now,
        )?,
        payload_schema_row(
            PAYLOAD_SCHEMA_HISTORY_GAP_V1,
            "journal_event_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Terminal history gap journal payload",
                "type": "object",
                "required": ["reason", "skipped_events", "estimated_dropped_bytes"],
                "properties": {
                    "reason": { "type": "string" },
                    "skipped_events": { "type": "integer", "minimum": 1 },
                    "estimated_dropped_bytes": { "type": ["integer", "null"], "minimum": 0 }
                },
                "additionalProperties": false
            }),
            now,
        )?,
        payload_schema_row(
            PAYLOAD_SCHEMA_JOURNAL_EVENT_V1,
            "journal_event_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Generic terminal journal payload",
                "type": "object",
                "additionalProperties": true
            }),
            now,
        )?,
        payload_schema_row(
            PAYLOAD_SCHEMA_TOPOLOGY_SNAPSHOT_V1,
            "topology_snapshot_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Terminal topology snapshot payload",
                "type": "object",
                "required": ["tabs"],
                "properties": {
                    "tabs": { "type": "array" }
                },
                "additionalProperties": true
            }),
            now,
        )?,
    ];

    for row in rows {
        insert_into(terminal_payload_schemas::table)
            .values(&row)
            .on_conflict(terminal_payload_schemas::id)
            .do_nothing()
            .execute(connection)?;
    }
    Ok(())
}

pub(in crate::v2) fn payload_schema_row(
    id: &str,
    payload_kind: &str,
    schema_version: &str,
    schema: Value,
    created_at_ms: i64,
) -> Result<NewPayloadSchemaRow, TerminalPersistenceV2Error> {
    let schema_json = serde_json::to_string(&schema)?;
    let schema_hash = blake3_hash_text(&schema_json);
    Ok(NewPayloadSchemaRow {
        id: id.to_string(),
        payload_kind: payload_kind.to_string(),
        schema_version: schema_version.to_string(),
        schema_json,
        schema_hash,
        created_at_ms,
    })
}
