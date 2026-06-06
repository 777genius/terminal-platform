use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePressureRecord {
    pub id: String,
    pub state: String,
    pub db_file_bytes: Option<i64>,
    pub wal_file_bytes: Option<i64>,
    pub disk_free_bytes: Option<i64>,
    pub temp_free_bytes: Option<i64>,
    pub quota_bytes: Option<i64>,
    pub action_taken: String,
    pub reason: Option<String>,
    pub created_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<StoragePressureEventRow> for StoragePressureRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: StoragePressureEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl From<NewStoragePressureEventRow> for StoragePressureRecord {
    fn from(row: NewStoragePressureEventRow) -> Self {
        Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row.metadata_json.and_then(|value| serde_json::from_str(&value).ok()),
        }
    }
}
