use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use terminal_domain::PaneId;

use crate::{
    ProjectionSource, ScreenBufferKind, ScreenCursor, ScreenLine, ScreenProgress, ScreenSurface,
    ScreenSurfacePalette,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenLinePatch {
    pub row: u16,
    pub line: ScreenLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenPatch {
    pub title_changed: bool,
    pub title: Option<String>,
    pub working_directory_uri_changed: bool,
    pub working_directory_uri: Option<String>,
    #[serde(default)]
    pub user_variables_changed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_variables: Option<BTreeMap<String, String>>,
    pub cursor_changed: bool,
    pub cursor: Option<ScreenCursor>,
    pub palette_changed: bool,
    pub palette: Option<ScreenSurfacePalette>,
    pub bell_count_changed: bool,
    pub bell_count: Option<u64>,
    pub progress_changed: bool,
    pub progress: Option<ScreenProgress>,
    pub line_updates: Vec<ScreenLinePatch>,
}

impl ScreenPatch {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.title_changed
            && !self.working_directory_uri_changed
            && !self.user_variables_changed
            && !self.cursor_changed
            && !self.palette_changed
            && !self.bell_count_changed
            && !self.progress_changed
            && self.line_updates.is_empty()
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
    #[serde(default, skip_serializing_if = "is_normal_buffer_kind")]
    pub buffer_kind: ScreenBufferKind,
    pub patch: Option<ScreenPatch>,
    pub full_replace: Option<ScreenSurface>,
}

fn is_normal_buffer_kind(value: &ScreenBufferKind) -> bool {
    *value == ScreenBufferKind::Normal
}
