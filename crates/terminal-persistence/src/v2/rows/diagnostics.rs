use super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_db_identity)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct DbIdentityProbeRow {
    pub(in crate::v2) id: i32,
}

#[derive(Debug, QueryableByName)]
pub(in crate::v2) struct QuickCheckRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub(in crate::v2) quick_check: String,
}

#[derive(Debug, QueryableByName, Serialize)]
pub(in crate::v2) struct WalCheckpointRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(in crate::v2) busy: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(in crate::v2) log: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(in crate::v2) checkpointed: i32,
}

#[derive(Debug, QueryableByName, Serialize)]
pub(in crate::v2) struct ForeignKeyCheckRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub(in crate::v2) table_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::BigInt>)]
    pub(in crate::v2) rowid: Option<i64>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub(in crate::v2) parent: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(in crate::v2) fkid: i32,
}

#[derive(Debug, Clone)]
pub(in crate::v2) struct HistoryValidation {
    pub(in crate::v2) journal_events_checked: usize,
    pub(in crate::v2) stream_segments_checked: usize,
    pub(in crate::v2) screen_snapshots_checked: usize,
    pub(in crate::v2) topology_snapshots_checked: usize,
    pub(in crate::v2) failures: Vec<String>,
}

impl HistoryValidation {
    pub(in crate::v2) fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    pub(in crate::v2) fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub(in crate::v2) fn checksum_failure_count(&self) -> usize {
        self.failures.iter().filter(|failure| failure.contains("checksum mismatch")).count()
    }

    pub(in crate::v2) fn summary(&self) -> String {
        if self.failures.is_empty() {
            "history validation passed".to_string()
        } else {
            self.failures.join("; ")
        }
    }

    pub(in crate::v2) fn to_json(&self) -> Value {
        serde_json::json!({
            "journal_events_checked": self.journal_events_checked,
            "stream_segments_checked": self.stream_segments_checked,
            "screen_snapshots_checked": self.screen_snapshots_checked,
            "topology_snapshots_checked": self.topology_snapshots_checked,
            "failures": self.failures,
        })
    }

    pub(in crate::v2) fn to_restore_evidence(&self) -> Vec<RestoreEvidence> {
        vec![
            RestoreEvidence {
                kind: "journal_events_checked".to_string(),
                value: self.journal_events_checked.to_string(),
            },
            RestoreEvidence {
                kind: "stream_segments_checked".to_string(),
                value: self.stream_segments_checked.to_string(),
            },
            RestoreEvidence {
                kind: "screen_snapshots_checked".to_string(),
                value: self.screen_snapshots_checked.to_string(),
            },
            RestoreEvidence {
                kind: "topology_snapshots_checked".to_string(),
                value: self.topology_snapshots_checked.to_string(),
            },
            RestoreEvidence {
                kind: "history_validation_failures".to_string(),
                value: self.failures.len().to_string(),
            },
        ]
    }
}
