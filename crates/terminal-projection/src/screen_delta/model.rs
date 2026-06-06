use serde::{Deserialize, Serialize};
use terminal_domain::PaneId;

use crate::{ProjectionSource, ScreenCursor, ScreenLine, ScreenSurface};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenLinePatch {
    pub row: u16,
    pub line: ScreenLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenPatch {
    pub title_changed: bool,
    pub title: Option<String>,
    pub cursor_changed: bool,
    pub cursor: Option<ScreenCursor>,
    pub line_updates: Vec<ScreenLinePatch>,
}

impl ScreenPatch {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.title_changed && !self.cursor_changed && self.line_updates.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenDelta {
    pub pane_id: PaneId,
    pub from_sequence: u64,
    pub to_sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: ProjectionSource,
    pub patch: Option<ScreenPatch>,
    pub full_replace: Option<ScreenSurface>,
}
