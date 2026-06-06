use super::super::super::*;
use super::super::*;

pub(in crate::v2) fn insert_prompt_injection_findings_for_items(
    connection: &mut SqliteConnection,
    package_id: &str,
    items: &[InsertedAiContextItem],
    now: i64,
) -> Result<i64, TerminalPersistenceV2Error> {
    let mut count = 0_i64;
    for item in items {
        if let Some(finding) = prompt_injection_finding(package_id, item, now)? {
            insert_into(terminal_prompt_injection_findings::table)
                .values(&finding)
                .execute(connection)?;
            count += 1;
        }
    }
    Ok(count)
}

fn prompt_injection_finding(
    package_id: &str,
    item: &InsertedAiContextItem,
    now: i64,
) -> Result<Option<NewPromptInjectionFindingRow>, TerminalPersistenceV2Error> {
    let Some(pattern_kind) = detect_prompt_injection_pattern(&item.content_preview) else {
        return Ok(None);
    };

    Ok(Some(NewPromptInjectionFindingRow {
        id: new_id(),
        package_id: Some(package_id.to_string()),
        item_id: Some(item.id.clone()),
        severity: "warning".to_string(),
        pattern_kind: pattern_kind.to_string(),
        action_state: "detected".to_string(),
        detected_at_ms: now,
        evidence_preview: limit_text_preview(&item.content_preview, 160),
        metadata_json: Some(serde_json::to_string(&serde_json::json!({
            "terminal_output_is_data_only": true,
            "auto_action_allowed": false
        }))?),
    }))
}
