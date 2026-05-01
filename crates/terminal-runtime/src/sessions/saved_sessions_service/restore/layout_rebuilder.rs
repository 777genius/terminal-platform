use std::collections::HashMap;

use terminal_backend_api::{BackendError, MuxCommand, SplitPaneSpec};
use terminal_domain::{PaneId, SessionId, TabId};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};

use crate::sessions::{
    active_session_service::ActiveSessionService,
    runtime::{collect_pane_ids_from_node, tab_snapshot_by_id},
};

use super::topology_diff::new_pane_id_after_split;

pub(super) async fn rebuild_saved_tab_layout(
    active: &ActiveSessionService<'_>,
    restored_session_id: SessionId,
    live_tab_id: TabId,
    saved_tab: &TabSnapshot,
) -> Result<HashMap<PaneId, PaneId>, BackendError> {
    let topology = active.topology_snapshot(restored_session_id).await?;
    let live_tab = tab_snapshot_by_id(&topology, live_tab_id)?;
    let initial_live_pane_id = collect_pane_ids_from_node(&live_tab.root)
        .into_iter()
        .next()
        .ok_or_else(|| BackendError::internal("restored native tab has no initial pane"))?;
    let mut pane_map = HashMap::new();
    let mut pending = vec![(saved_tab.root.clone(), initial_live_pane_id)];

    while let Some((node, live_pane_id)) = pending.pop() {
        match node {
            PaneTreeNode::Leaf { pane_id } => {
                pane_map.insert(pane_id, live_pane_id);
            }
            PaneTreeNode::Split(split) => {
                let new_pane_id = split_live_pane(
                    active,
                    restored_session_id,
                    live_tab_id,
                    live_pane_id,
                    split.direction,
                )
                .await?;

                pending.push((*split.second, new_pane_id));
                pending.push((*split.first, live_pane_id));
            }
        }
    }

    Ok(pane_map)
}

async fn split_live_pane(
    active: &ActiveSessionService<'_>,
    restored_session_id: SessionId,
    live_tab_id: TabId,
    live_pane_id: PaneId,
    direction: terminal_mux_domain::SplitDirection,
) -> Result<PaneId, BackendError> {
    let before = active.topology_snapshot(restored_session_id).await?;
    let before_tab = tab_snapshot_by_id(&before, live_tab_id)?;
    let before_panes = collect_pane_ids_from_node(&before_tab.root);

    active
        .dispatch(
            restored_session_id,
            MuxCommand::SplitPane(SplitPaneSpec { pane_id: live_pane_id, direction }),
        )
        .await?;

    let after = active.topology_snapshot(restored_session_id).await?;
    let after_tab = tab_snapshot_by_id(&after, live_tab_id)?;
    let after_panes = collect_pane_ids_from_node(&after_tab.root);

    new_pane_id_after_split(&before_panes, &after_panes)
}
