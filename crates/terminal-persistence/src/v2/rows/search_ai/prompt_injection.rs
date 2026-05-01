use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_prompt_injection_findings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct PromptInjectionFindingRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) item_id: Option<String>,
    pub(in crate::v2) severity: String,
    pub(in crate::v2) pattern_kind: String,
    pub(in crate::v2) action_state: String,
    pub(in crate::v2) detected_at_ms: i64,
    pub(in crate::v2) evidence_preview: String,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_prompt_injection_findings)]
pub(in crate::v2) struct NewPromptInjectionFindingRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) item_id: Option<String>,
    pub(in crate::v2) severity: String,
    pub(in crate::v2) pattern_kind: String,
    pub(in crate::v2) action_state: String,
    pub(in crate::v2) detected_at_ms: i64,
    pub(in crate::v2) evidence_preview: String,
    pub(in crate::v2) metadata_json: Option<String>,
}
