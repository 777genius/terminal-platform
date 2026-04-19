use serde::{Deserialize, Serialize};

use terminal_domain::PaneId;

use crate::ProjectionSource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenCursor {
    pub row: u16,
    pub col: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenLine {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSurface {
    pub title: Option<String>,
    pub cursor: Option<ScreenCursor>,
    pub lines: Vec<ScreenLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSnapshot {
    pub pane_id: PaneId,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: ProjectionSource,
    pub surface: ScreenSurface,
}
