use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityCheckRecord {
    pub id: String,
    pub check_kind: String,
    pub scope_kind: String,
    pub scope_ref: Option<String>,
    pub result: String,
    pub checked_at_ms: i64,
    pub details_json: Option<Value>,
    pub error: Option<String>,
}
