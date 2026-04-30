use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryClientRecord {
    pub id: String,
    pub client_kind: String,
    pub last_seen_at_ms: i64,
    pub trust_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryOffsetRecord {
    pub id: String,
    pub client_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: String,
    pub last_sent_event_seq: i64,
    pub last_acked_event_seq: i64,
    pub last_persisted_event_seq: i64,
    pub replay_from_event_seq: Option<i64>,
    pub gap_state: String,
    pub updated_at_ms: i64,
}

impl From<DeliveryOffsetRow> for DeliveryOffsetRecord {
    fn from(row: DeliveryOffsetRow) -> Self {
        Self {
            id: row.id,
            client_id: row.client_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            stream_id: row.stream_id,
            last_sent_event_seq: row.last_sent_event_seq,
            last_acked_event_seq: row.last_acked_event_seq,
            last_persisted_event_seq: row.last_persisted_event_seq,
            replay_from_event_seq: row.replay_from_event_seq,
            gap_state: row.gap_state,
            updated_at_ms: row.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReplayWindow {
    pub from_event_seq: Option<i64>,
    pub to_event_seq: i64,
    pub gap_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxMessageRecord {
    pub id: String,
    pub message_kind: String,
    pub dedupe_key: Option<String>,
    pub state: String,
    pub payload_json: Value,
    pub attempts: i64,
    pub max_attempts: i64,
    pub claimed_by: Option<String>,
    pub lease_token: Option<String>,
    pub claimed_until_ms: Option<i64>,
    pub next_run_at_ms: i64,
    pub last_error: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub pending_count: i64,
    pub due_pending_count: i64,
    pub claimed_count: i64,
    pub stale_claim_count: i64,
    pub done_count: i64,
    pub failed_count: i64,
    pub quarantined_count: i64,
    pub oldest_due_pending_age_ms: Option<i64>,
    pub next_pending_due_in_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub feature_gate_state: String,
    pub raw_segment_count: i64,
    pub zstd_segment_count: i64,
    pub unsupported_segment_count: i64,
    pub rewrite_candidate_count: i64,
    pub segments_rewritten: i64,
    pub restore_drill_required: bool,
    pub action_taken: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub policy_id: String,
    pub policy_kind: String,
    pub pressure_behavior: String,
    pub raw_history_prune_behavior: String,
    pub sessions_scanned: i64,
    pub scan_mode: String,
    pub maintenance_deletes_raw_history: bool,
    pub action_taken: String,
}

impl TryFrom<OutboxMessageRow> for OutboxMessageRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: OutboxMessageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            message_kind: row.message_kind,
            dedupe_key: row.dedupe_key,
            state: row.state,
            payload_json: serde_json::from_str(&row.payload_json)?,
            attempts: row.attempts,
            max_attempts: row.max_attempts,
            claimed_by: row.claimed_by,
            lease_token: row.lease_token,
            claimed_until_ms: row.claimed_until_ms,
            next_run_at_ms: row.next_run_at_ms,
            last_error: row.last_error,
            created_at_ms: row.created_at_ms,
            updated_at_ms: row.updated_at_ms,
        })
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandHistoryEntryRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub display_text: String,
    pub last_used_at_ms: i64,
    pub use_count: i64,
}

impl From<CommandHistoryEntryRow> for CommandHistoryEntryRecord {
    fn from(row: CommandHistoryEntryRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            display_text: row.display_text,
            last_used_at_ms: row.last_used_at_ms,
            use_count: row.use_count,
        }
    }
}
