use super::super::super::*;

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
