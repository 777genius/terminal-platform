use serde::{Deserialize, Serialize};

use terminal_domain::PaneId;

use crate::{ProjectionSource, ScreenSurface};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenDelta {
    pub pane_id: PaneId,
    pub from_sequence: u64,
    pub to_sequence: u64,
    pub source: ProjectionSource,
    pub full_replace: Option<ScreenSurface>,
}
