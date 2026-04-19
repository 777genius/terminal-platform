use serde::{Deserialize, Serialize};

use terminal_domain::{PaneId, TabId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusTarget {
    Pane(PaneId),
    Tab(TabId),
}
