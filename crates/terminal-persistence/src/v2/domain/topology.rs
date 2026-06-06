use super::super::*;

pub(in crate::v2) fn legacy_pane_high_water(saved: &SavedNativeSession) -> Value {
    let mut map = serde_json::Map::new();
    for screen in &saved.screens {
        map.insert(screen.pane_id.0.to_string(), Value::from(screen.sequence));
    }
    Value::Object(map)
}

pub(in crate::v2) fn topology_pane_high_water_from_store(
    connection: &mut SqliteConnection,
    session_id: &str,
    topology: &TopologySnapshot,
) -> Result<Value, TerminalPersistenceV2Error> {
    let mut map = topology_pane_high_water_map(topology);
    if !map.is_empty() {
        let pane_ids = map.keys().cloned().collect::<Vec<_>>();
        let persisted_high_water = terminal_panes::table
            .filter(terminal_panes::session_id.eq(session_id))
            .filter(terminal_panes::id.eq_any(&pane_ids))
            .select((terminal_panes::id, terminal_panes::last_event_seq))
            .load::<(String, i64)>(connection)?;
        for (pane_id, last_event_seq) in persisted_high_water {
            if let Some(value) = map.get_mut(&pane_id) {
                *value = last_event_seq.max(0);
            }
        }
    }

    let mut output = serde_json::Map::new();
    for (pane_id, high_water_event_seq) in map {
        output.insert(pane_id, Value::from(high_water_event_seq));
    }
    Ok(Value::Object(output))
}

pub(in crate::v2) fn topology_pane_high_water_map(
    topology: &TopologySnapshot,
) -> BTreeMap<String, i64> {
    let mut map = BTreeMap::new();
    for tab in &topology.tabs {
        collect_topology_pane_high_water(&tab.root, &mut map);
    }
    map
}

pub(in crate::v2) fn collect_topology_pane_high_water(
    node: &terminal_mux_domain::PaneTreeNode,
    map: &mut BTreeMap<String, i64>,
) {
    match node {
        terminal_mux_domain::PaneTreeNode::Leaf { pane_id } => {
            map.entry(pane_id.0.to_string()).or_insert(0);
        }
        terminal_mux_domain::PaneTreeNode::Split(split) => {
            collect_topology_pane_high_water(&split.first, map);
            collect_topology_pane_high_water(&split.second, map);
        }
    }
}
