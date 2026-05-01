use super::{prelude::*, *};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneSplit {
    pub direction: NodeSplitDirection,
    pub first: Box<NodePaneTreeNode>,
    pub second: Box<NodePaneTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodePaneTreeNode {
    Leaf { pane_id: String },
    Split(NodePaneSplit),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSplitPaneCommand {
    pub pane_id: String,
    pub direction: NodeSplitDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeResizePaneCommand {
    pub pane_id: String,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeNewTabCommand {
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeRenameTabCommand {
    pub tab_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSendInputCommand {
    pub pane_id: String,
    pub data: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub client_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSendPasteCommand {
    pub pane_id: String,
    pub data: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub client_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeOverrideLayoutCommand {
    pub tab_id: String,
    pub root: NodePaneTreeNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodeMuxCommand {
    SplitPane(NodeSplitPaneCommand),
    ClosePane { pane_id: String },
    FocusPane { pane_id: String },
    ResizePane(NodeResizePaneCommand),
    NewTab(NodeNewTabCommand),
    CloseTab { tab_id: String },
    FocusTab { tab_id: String },
    RenameTab(NodeRenameTabCommand),
    SendInput(NodeSendInputCommand),
    SendPaste(NodeSendPasteCommand),
    Detach,
    SaveSession,
    OverrideLayout(NodeOverrideLayoutCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeMuxCommandResult {
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeTabSnapshot {
    pub tab_id: String,
    pub title: Option<String>,
    pub root: NodePaneTreeNode,
    pub focused_pane: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeTopologySnapshot {
    pub session_id: String,
    pub backend_kind: NodeBackendKind,
    pub tabs: Vec<NodeTabSnapshot>,
    pub focused_tab: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeAttachedSession {
    pub session: NodeSessionSummary,
    pub health: NodeSessionHealthSnapshot,
    pub topology: NodeTopologySnapshot,
    pub focused_screen: Option<NodeScreenSnapshot>,
}
