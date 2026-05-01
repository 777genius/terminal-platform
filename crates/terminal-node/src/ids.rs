use terminal_domain::{PaneId, SessionId};
use terminal_mux_domain::PaneTreeNode;
use terminal_protocol::ProtocolError;
use uuid::Uuid;

pub(crate) fn parse_session_id(value: &str) -> Result<SessionId, ProtocolError> {
    parse_uuid(value, "invalid_session_id", "session").map(SessionId::from)
}

pub(crate) fn parse_pane_id(value: &str) -> Result<PaneId, ProtocolError> {
    parse_uuid(value, "invalid_pane_id", "pane").map(PaneId::from)
}

fn parse_uuid(value: &str, code: &str, label: &str) -> Result<Uuid, ProtocolError> {
    Uuid::parse_str(value).map_err(|error| {
        ProtocolError::new(code, format!("failed to parse {label} id '{value}' - {error}"))
    })
}

pub(crate) fn focused_pane_id(topology: &terminal_projection::TopologySnapshot) -> Option<PaneId> {
    let tab = topology
        .focused_tab
        .and_then(|focused_tab| topology.tabs.iter().find(|tab| tab.tab_id == focused_tab))
        .or_else(|| topology.tabs.first())?;

    tab.focused_pane.or_else(|| first_pane_id(&tab.root))
}

fn first_pane_id(root: &PaneTreeNode) -> Option<PaneId> {
    match root {
        PaneTreeNode::Leaf { pane_id } => Some(*pane_id),
        PaneTreeNode::Split(split) => {
            first_pane_id(&split.first).or_else(|| first_pane_id(&split.second))
        }
    }
}
