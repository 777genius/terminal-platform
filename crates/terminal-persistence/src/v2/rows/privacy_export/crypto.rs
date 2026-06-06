use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_crypto_keys)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct CryptoKeyRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) key_kind: String,
    pub(in crate::v2) key_ref: String,
    pub(in crate::v2) protection_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) rotated_at_ms: Option<i64>,
    pub(in crate::v2) destroyed_at_ms: Option<i64>,
    pub(in crate::v2) capability_report_json: Option<String>,
    pub(in crate::v2) error_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_crypto_keys)]
pub(in crate::v2) struct NewCryptoKeyRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) key_kind: String,
    pub(in crate::v2) key_ref: String,
    pub(in crate::v2) protection_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) rotated_at_ms: Option<i64>,
    pub(in crate::v2) destroyed_at_ms: Option<i64>,
    pub(in crate::v2) capability_report_json: Option<String>,
    pub(in crate::v2) error_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_crypto_key_events)]
pub(in crate::v2) struct NewCryptoKeyEventRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) key_id: Option<String>,
    pub(in crate::v2) event_kind: String,
    pub(in crate::v2) actor: String,
    pub(in crate::v2) occurred_at_ms: i64,
    pub(in crate::v2) status: String,
    pub(in crate::v2) error_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}
