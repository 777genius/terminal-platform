use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterGenerationLease {
    pub id: String,
    pub process_id: String,
    pub lease_token: String,
    pub lease_expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSegmentReceipt {
    pub commit_id: String,
    pub commit_seq: i64,
    pub segment_id: String,
    pub event_id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEventReceipt {
    pub commit_id: String,
    pub commit_seq: i64,
    pub event_id: String,
    pub event_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamSegmentRecord {
    pub id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub payload: Vec<u8>,
    pub checksum: String,
    pub capture_semantics: String,
    pub created_at_ms: i64,
}

impl From<StreamSegmentRow> for StreamSegmentRecord {
    fn from(row: StreamSegmentRow) -> Self {
        Self {
            id: row.id,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            payload: row.payload,
            checksum: row.checksum,
            capture_semantics: row.capture_semantics,
            created_at_ms: row.created_at_ms,
        }
    }
}
