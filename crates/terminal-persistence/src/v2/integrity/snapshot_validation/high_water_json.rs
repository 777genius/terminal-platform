use super::super::super::*;

pub(in crate::v2) fn validate_topology_pane_high_water_json_payload(
    topology_snapshot_id: &str,
    pane_high_water_json: &str,
    failures: &mut Vec<String>,
) {
    if let Err(error) = parse_pane_high_water_json(pane_high_water_json) {
        failures.push(format!(
            "topology_snapshot:{topology_snapshot_id} invalid pane_high_water_json: {error}"
        ));
    }
}

pub(in crate::v2) fn parse_pane_high_water_json(
    pane_high_water_json: &str,
) -> Result<BTreeMap<String, i64>, TerminalPersistenceV2Error> {
    let value: Value = serde_json::from_str(pane_high_water_json)?;
    let Some(object) = value.as_object() else {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "pane_high_water_json must be a JSON object".to_string(),
        ));
    };
    let mut high_water = BTreeMap::new();
    for (pane_id, raw_value) in object {
        let Some(value) = raw_value.as_i64() else {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "pane_high_water_json value for pane_id={pane_id} must be an integer"
            )));
        };
        if value < 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "pane_high_water_json value for pane_id={pane_id} must be non-negative"
            )));
        }
        high_water.insert(pane_id.clone(), value);
    }
    Ok(high_water)
}
