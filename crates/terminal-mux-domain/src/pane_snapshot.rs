use serde::{Deserialize, Serialize};

use terminal_domain::PaneId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneSnapshot {
    pub pane_id: PaneId,
    pub title: Option<String>,
    pub focused: bool,
}
