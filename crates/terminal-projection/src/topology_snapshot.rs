use serde::{Deserialize, Serialize};

use terminal_domain::{BackendKind, SessionId, TabId};
use terminal_mux_domain::TabSnapshot;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySnapshot {
    pub session_id: SessionId,
    pub backend_kind: BackendKind,
    pub tabs: Vec<TabSnapshot>,
    pub focused_tab: Option<TabId>,
}
