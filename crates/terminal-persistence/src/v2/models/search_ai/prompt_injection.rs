use super::super::super::*;
use super::json::parse_optional_json;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptInjectionFindingRecord {
    pub id: String,
    pub package_id: Option<String>,
    pub item_id: Option<String>,
    pub severity: String,
    pub pattern_kind: String,
    pub action_state: String,
    pub detected_at_ms: i64,
    pub evidence_preview: String,
    pub metadata_json: Option<Value>,
}

impl TryFrom<PromptInjectionFindingRow> for PromptInjectionFindingRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: PromptInjectionFindingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            item_id: row.item_id,
            severity: row.severity,
            pattern_kind: row.pattern_kind,
            action_state: row.action_state,
            detected_at_ms: row.detected_at_ms,
            evidence_preview: row.evidence_preview,
            metadata_json: parse_optional_json(row.metadata_json)?,
        })
    }
}
