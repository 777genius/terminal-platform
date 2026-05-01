use super::super::super::*;
use super::json::parse_optional_json;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocumentRecord {
    pub document_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub document_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_profile_id: Option<String>,
    pub redaction_state: String,
    pub source_hash_algorithm: String,
    pub source_hash: String,
    pub text_preview: String,
    pub updated_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<SearchDocumentRow> for SearchDocumentRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: SearchDocumentRow) -> Result<Self, Self::Error> {
        Ok(Self {
            document_id: row.document_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            command_block_id: row.command_block_id,
            document_kind: row.document_kind,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            redaction_profile_id: row.redaction_profile_id,
            redaction_state: row.redaction_state,
            source_hash_algorithm: row.source_hash_algorithm,
            source_hash: row.source_hash,
            text_preview: row.text_preview,
            updated_at_ms: row.updated_at_ms,
            metadata_json: parse_optional_json(row.metadata_json)?,
        })
    }
}
