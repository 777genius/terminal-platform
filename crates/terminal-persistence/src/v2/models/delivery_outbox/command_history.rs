use super::super::super::*;

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
