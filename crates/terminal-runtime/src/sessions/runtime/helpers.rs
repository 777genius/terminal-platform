use terminal_backend_api::{BackendError, BackendErrorKind, MuxCommand};
use terminal_domain::{PaneId, SessionId, SessionRoute, TabId};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
use terminal_projection::{SessionHealthReason, SessionHealthSnapshot, TopologySnapshot};

pub(in crate::sessions) fn tab_id_for_pane(
    topology: &TopologySnapshot,
    pane_id: PaneId,
) -> Option<TabId> {
    topology
        .tabs
        .iter()
        .find(|tab| collect_pane_ids_from_node(&tab.root).contains(&pane_id))
        .map(|tab| tab.tab_id)
}

pub(in crate::sessions) fn session_health_from_attach_error(
    session_id: SessionId,
    error: &BackendError,
) -> Option<SessionHealthSnapshot> {
    match error.kind {
        BackendErrorKind::Unsupported => error.degraded_reason.as_ref().map(|_| {
            SessionHealthSnapshot::degraded(
                session_id,
                SessionHealthReason::BackendDegraded,
                error.message.clone(),
            )
        }),
        BackendErrorKind::NotFound => Some(SessionHealthSnapshot::terminated(
            session_id,
            SessionHealthReason::SessionNotFound,
            error.message.clone(),
        )),
        BackendErrorKind::Transport => Some(SessionHealthSnapshot::stale(
            session_id,
            SessionHealthReason::BackendTransportLost,
            error.message.clone(),
        )),
        BackendErrorKind::Internal => Some(SessionHealthSnapshot::stale(
            session_id,
            SessionHealthReason::BackendInternalFault,
            error.message.clone(),
        )),
        BackendErrorKind::InvalidInput => None,
    }
}

pub(in crate::sessions) fn session_route_fingerprint(route: &SessionRoute) -> String {
    let external = route
        .external
        .as_ref()
        .map(|external| format!("{}/{}", external.namespace, external.value))
        .unwrap_or_else(|| "-".to_string());

    format!("v1/{:?}/{:?}/{external}", route.backend, route.authority)
}

pub(in crate::sessions) fn collect_pane_ids_from_topology(
    topology: &TopologySnapshot,
) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    for tab in &topology.tabs {
        pane_ids.extend(collect_pane_ids_from_node(&tab.root));
    }
    pane_ids
}

pub(in crate::sessions) fn collect_pane_ids_from_node(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_from_node_inner(root, &mut pane_ids);
    pane_ids
}

pub(in crate::sessions) fn saved_session_title(
    descriptor_title: Option<String>,
    topology: &TopologySnapshot,
) -> Option<String> {
    topology
        .focused_tab
        .and_then(|focused_tab| {
            topology
                .tabs
                .iter()
                .find(|tab| tab.tab_id == focused_tab)
                .and_then(|tab| tab.title.clone())
        })
        .or_else(|| topology.tabs.iter().find_map(|tab| tab.title.clone()))
        .or(descriptor_title)
}

pub(in crate::sessions) fn command_updates_summary_title(command: &MuxCommand) -> bool {
    matches!(
        command,
        MuxCommand::NewTab(_)
            | MuxCommand::CloseTab { .. }
            | MuxCommand::FocusTab { .. }
            | MuxCommand::RenameTab { .. }
    )
}

pub(in crate::sessions) fn tab_snapshot_by_id(
    topology: &TopologySnapshot,
    tab_id: TabId,
) -> Result<TabSnapshot, BackendError> {
    topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id == tab_id)
        .cloned()
        .ok_or_else(|| BackendError::internal(format!("missing restored tab {tab_id:?}")))
}

fn collect_pane_ids_from_node_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_from_node_inner(&split.first, pane_ids);
            collect_pane_ids_from_node_inner(&split.second, pane_ids);
        }
    }
}
