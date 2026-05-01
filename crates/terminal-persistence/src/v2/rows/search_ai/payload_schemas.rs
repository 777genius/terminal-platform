use super::super::super::*;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_payload_schemas)]
pub(in crate::v2) struct NewPayloadSchemaRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) payload_kind: String,
    pub(in crate::v2) schema_version: String,
    pub(in crate::v2) schema_json: String,
    pub(in crate::v2) schema_hash: String,
    pub(in crate::v2) created_at_ms: i64,
}
