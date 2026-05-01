use super::{prelude::*, *};

impl From<&SplitDirection> for NodeSplitDirection {
    fn from(value: &SplitDirection) -> Self {
        match value {
            SplitDirection::Horizontal => Self::Horizontal,
            SplitDirection::Vertical => Self::Vertical,
        }
    }
}

impl From<&NodeSplitDirection> for SplitDirection {
    fn from(value: &NodeSplitDirection) -> Self {
        match value {
            NodeSplitDirection::Horizontal => Self::Horizontal,
            NodeSplitDirection::Vertical => Self::Vertical,
        }
    }
}

impl From<&PaneSplit> for NodePaneSplit {
    fn from(value: &PaneSplit) -> Self {
        Self {
            direction: (&value.direction).into(),
            first: Box::new((&*value.first).into()),
            second: Box::new((&*value.second).into()),
        }
    }
}

impl From<&PaneTreeNode> for NodePaneTreeNode {
    fn from(value: &PaneTreeNode) -> Self {
        match value {
            PaneTreeNode::Leaf { pane_id } => Self::Leaf { pane_id: pane_id.0.to_string() },
            PaneTreeNode::Split(split) => Self::Split(split.into()),
        }
    }
}

impl From<&TabSnapshot> for NodeTabSnapshot {
    fn from(value: &TabSnapshot) -> Self {
        Self {
            tab_id: value.tab_id.0.to_string(),
            title: value.title.clone(),
            root: (&value.root).into(),
            focused_pane: value.focused_pane.map(|pane_id| pane_id.0.to_string()),
        }
    }
}

impl From<&TopologySnapshot> for NodeTopologySnapshot {
    fn from(value: &TopologySnapshot) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            backend_kind: (&value.backend_kind).into(),
            tabs: value.tabs.iter().map(Into::into).collect(),
            focused_tab: value.focused_tab.map(|tab_id| tab_id.0.to_string()),
        }
    }
}

impl From<&MuxCommandResult> for NodeMuxCommandResult {
    fn from(value: &MuxCommandResult) -> Self {
        Self { changed: value.changed }
    }
}

impl TryFrom<&NodeSessionRoute> for SessionRoute {
    type Error = ProtocolError;

    fn try_from(value: &NodeSessionRoute) -> Result<Self, Self::Error> {
        Ok(Self {
            backend: (&value.backend).into(),
            authority: (&value.authority).into(),
            external: value.external.as_ref().map(|external| terminal_domain::ExternalSessionRef {
                namespace: external.namespace.clone(),
                value: external.value.clone(),
            }),
        })
    }
}

impl From<&NodeBackendKind> for BackendKind {
    fn from(value: &NodeBackendKind) -> Self {
        match value {
            NodeBackendKind::Native => Self::Native,
            NodeBackendKind::Tmux => Self::Tmux,
            NodeBackendKind::Zellij => Self::Zellij,
        }
    }
}

impl From<&NodeRouteAuthority> for RouteAuthority {
    fn from(value: &NodeRouteAuthority) -> Self {
        match value {
            NodeRouteAuthority::LocalDaemon => Self::LocalDaemon,
            NodeRouteAuthority::ImportedForeign => Self::ImportedForeign,
        }
    }
}

impl TryFrom<&NodePaneTreeNode> for PaneTreeNode {
    type Error = ProtocolError;

    fn try_from(value: &NodePaneTreeNode) -> Result<Self, Self::Error> {
        match value {
            NodePaneTreeNode::Leaf { pane_id } => {
                Ok(Self::Leaf { pane_id: parse_pane_id(pane_id)? })
            }
            NodePaneTreeNode::Split(split) => Ok(Self::Split(PaneSplit {
                direction: (&split.direction).into(),
                first: Box::new((&*split.first).try_into()?),
                second: Box::new((&*split.second).try_into()?),
            })),
        }
    }
}

impl TryFrom<&NodeMuxCommand> for MuxCommand {
    type Error = ProtocolError;

    fn try_from(value: &NodeMuxCommand) -> Result<Self, Self::Error> {
        Ok(match value {
            NodeMuxCommand::SplitPane(command) => Self::SplitPane(SplitPaneSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                direction: (&command.direction).into(),
            }),
            NodeMuxCommand::ClosePane { pane_id } => {
                Self::ClosePane { pane_id: parse_pane_id(pane_id)? }
            }
            NodeMuxCommand::FocusPane { pane_id } => {
                Self::FocusPane { pane_id: parse_pane_id(pane_id)? }
            }
            NodeMuxCommand::ResizePane(command) => Self::ResizePane(ResizePaneSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                rows: command.rows,
                cols: command.cols,
            }),
            NodeMuxCommand::NewTab(command) => {
                Self::NewTab(NewTabSpec { title: command.title.clone() })
            }
            NodeMuxCommand::CloseTab { tab_id } => Self::CloseTab { tab_id: parse_tab_id(tab_id)? },
            NodeMuxCommand::FocusTab { tab_id } => Self::FocusTab { tab_id: parse_tab_id(tab_id)? },
            NodeMuxCommand::RenameTab(command) => Self::RenameTab {
                tab_id: parse_tab_id(&command.tab_id)?,
                title: command.title.clone(),
            },
            NodeMuxCommand::SendInput(command) => Self::SendInput(SendInputSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                data: command.data.clone(),
                client_event_id: command.client_event_id.clone(),
            }),
            NodeMuxCommand::SendPaste(command) => Self::SendPaste(SendPasteSpec {
                pane_id: parse_pane_id(&command.pane_id)?,
                data: command.data.clone(),
                client_event_id: command.client_event_id.clone(),
            }),
            NodeMuxCommand::Detach => Self::Detach,
            NodeMuxCommand::SaveSession => Self::SaveSession,
            NodeMuxCommand::OverrideLayout(command) => Self::OverrideLayout(OverrideLayoutSpec {
                tab_id: parse_tab_id(&command.tab_id)?,
                root: (&command.root).try_into()?,
            }),
        })
    }
}

impl TryFrom<&NodeSubscriptionSpec> for SubscriptionSpec {
    type Error = ProtocolError;

    fn try_from(value: &NodeSubscriptionSpec) -> Result<Self, Self::Error> {
        Ok(match value {
            NodeSubscriptionSpec::SessionTopology => Self::SessionTopology,
            NodeSubscriptionSpec::PaneSurface { pane_id } => {
                Self::PaneSurface { pane_id: parse_pane_id(pane_id)? }
            }
        })
    }
}

fn parse_pane_id(value: &str) -> Result<PaneId, ProtocolError> {
    parse_uuid(value, "invalid_pane_id", "pane").map(PaneId::from)
}

fn parse_tab_id(value: &str) -> Result<TabId, ProtocolError> {
    parse_uuid(value, "invalid_tab_id", "tab").map(TabId::from)
}

fn parse_uuid(value: &str, code: &str, label: &str) -> Result<Uuid, ProtocolError> {
    Uuid::parse_str(value).map_err(|error| {
        ProtocolError::new(code, format!("failed to parse {label} id '{value}' - {error}"))
    })
}
