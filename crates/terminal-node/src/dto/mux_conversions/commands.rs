use crate::dto::{prelude::*, *};

use super::ids::{parse_pane_id, parse_tab_id};

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
