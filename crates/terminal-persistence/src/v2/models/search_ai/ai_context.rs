use super::super::super::*;
use super::json::parse_optional_json;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextPackageRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub requested_at_ms: i64,
    pub built_at_ms: Option<i64>,
    pub item_count: i64,
    pub manifest_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiContextPackageRow> for AiContextPackageRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiContextPackageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            requested_at_ms: row.requested_at_ms,
            built_at_ms: row.built_at_ms,
            item_count: row.item_count,
            manifest_json: parse_optional_json(row.manifest_json)?,
            metadata_json: parse_optional_json(row.metadata_json)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextItemRecord {
    pub id: String,
    pub package_id: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_state: String,
    pub data_only: bool,
    pub content_preview: String,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiContextItemRow> for AiContextItemRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiContextItemRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            source_kind: row.source_kind,
            source_ref: row.source_ref,
            session_id: row.session_id,
            pane_id: row.pane_id,
            command_block_id: row.command_block_id,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            redaction_state: row.redaction_state,
            data_only: row.data_only != 0,
            content_preview: row.content_preview,
            metadata_json: parse_optional_json(row.metadata_json)?,
        })
    }
}
