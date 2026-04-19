use serde::{Deserialize, Serialize};

use terminal_domain::{PaneId, TabId};
use terminal_mux_domain::{PaneTreeNode, SplitDirection};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitPaneSpec {
    pub pane_id: PaneId,
    pub direction: SplitDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResizePaneSpec {
    pub pane_id: PaneId,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NewTabSpec {
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendInputSpec {
    pub pane_id: PaneId,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendPasteSpec {
    pub pane_id: PaneId,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverrideLayoutSpec {
    pub tab_id: TabId,
    pub root: PaneTreeNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MuxCommand {
    SplitPane(SplitPaneSpec),
    ClosePane { pane_id: PaneId },
    FocusPane { pane_id: PaneId },
    ResizePane(ResizePaneSpec),
    NewTab(NewTabSpec),
    CloseTab { tab_id: TabId },
    FocusTab { tab_id: TabId },
    RenameTab { tab_id: TabId, title: String },
    SendInput(SendInputSpec),
    SendPaste(SendPasteSpec),
    Detach,
    SaveSession,
    OverrideLayout(OverrideLayoutSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuxCommandResult {
    pub changed: bool,
}
