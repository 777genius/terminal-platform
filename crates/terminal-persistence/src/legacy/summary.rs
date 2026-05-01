use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::{SavedSessionManifest, SessionId, SessionRoute};
use terminal_mux_domain::PaneTreeNode;
use terminal_projection::TopologySnapshot;
use uuid::Uuid;

use super::{PersistenceError, SavedSessionSummary};

pub(super) type SavedSessionSummaryRow =
    (String, String, Option<String>, String, String, String, i64);

pub(super) fn decode_saved_session_summary_row(
    (
        session_id,
        route_json,
        title,
        launch_json,
        manifest_json,
        topology_json,
        saved_at_ms,
    ): SavedSessionSummaryRow,
) -> Result<SavedSessionSummary, PersistenceError> {
    let route: SessionRoute = serde_json::from_str(&route_json)?;
    let launch: Option<ShellLaunchSpec> = serde_json::from_str(&launch_json)?;
    let manifest: SavedSessionManifest = serde_json::from_str(&manifest_json)?;
    let topology: TopologySnapshot = serde_json::from_str(&topology_json)?;
    Ok(SavedSessionSummary {
        session_id: SessionId::from(Uuid::parse_str(&session_id).map_err(|error| {
            PersistenceError::InvalidData(format!(
                "invalid saved session id `{session_id}` - {error}"
            ))
        })?),
        route,
        title,
        saved_at_ms,
        manifest,
        has_launch: launch.is_some(),
        tab_count: topology.tabs.len(),
        pane_count: topology.tabs.iter().map(|tab| pane_count(&tab.root)).sum(),
    })
}

fn pane_count(root: &PaneTreeNode) -> usize {
    match root {
        PaneTreeNode::Leaf { .. } => 1,
        PaneTreeNode::Split(split) => pane_count(&split.first) + pane_count(&split.second),
    }
}
