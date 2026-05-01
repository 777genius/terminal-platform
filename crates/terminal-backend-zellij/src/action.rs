use crate::snapshot::ZellijSessionSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ZellijAction {
    NewTab { title: Option<String> },
    FocusTab { backend_tab_id: u32, display_index: u32 },
    CloseTab { backend_tab_id: u32 },
    RenameTab { backend_tab_id: u32, title: String },
    FocusPane { pane_ref: String },
    ClosePane { pane_ref: String },
    WriteChars { pane_ref: String, chars: String },
    Paste { pane_ref: String, text: String },
    SendKeys { pane_ref: String, keys: Vec<String> },
}

impl ZellijAction {
    pub(crate) fn requires_settle(&self) -> bool {
        matches!(
            self,
            Self::NewTab { .. }
                | Self::FocusTab { .. }
                | Self::CloseTab { .. }
                | Self::RenameTab { .. }
                | Self::FocusPane { .. }
                | Self::ClosePane { .. }
        )
    }

    pub(crate) fn settled(
        &self,
        previous: &ZellijSessionSnapshot,
        current: &ZellijSessionSnapshot,
    ) -> bool {
        match self {
            Self::NewTab { title } => {
                current.topology.tabs.len() > previous.topology.tabs.len()
                    && title.as_ref().is_none_or(|title| {
                        current
                            .topology
                            .tabs
                            .iter()
                            .any(|tab| tab.title.as_deref() == Some(title.as_str()))
                    })
            }
            Self::FocusTab { backend_tab_id, .. } => {
                current.focused_backend_tab_id() == Some(*backend_tab_id)
            }
            Self::CloseTab { backend_tab_id } => !current.tab_exists(*backend_tab_id),
            Self::RenameTab { backend_tab_id, title } => {
                current.tab_title(*backend_tab_id) == Some(title.as_str())
            }
            Self::FocusPane { pane_ref } => current
                .topology
                .tabs
                .iter()
                .find(|tab| Some(tab.tab_id) == current.topology.focused_tab)
                .and_then(|tab| tab.focused_pane)
                .and_then(|pane_id| current.pane_targets.get(&pane_id))
                .map(|pane| pane.backend_ref == *pane_ref)
                .unwrap_or(false),
            Self::ClosePane { pane_ref } => {
                !current.pane_targets.values().any(|pane| pane.backend_ref == *pane_ref)
            }
            Self::WriteChars { .. } | Self::Paste { .. } | Self::SendKeys { .. } => true,
        }
    }

    pub(crate) fn args(&self) -> Vec<String> {
        match self {
            Self::NewTab { title } => {
                let mut args = vec!["action".to_string(), "new-tab".to_string()];
                if let Some(title) = title {
                    args.push("--name".to_string());
                    args.push(title.clone());
                }
                args
            }
            Self::FocusTab { backend_tab_id, display_index } => {
                if cfg!(windows) {
                    vec![
                        "action".to_string(),
                        "go-to-tab".to_string(),
                        (display_index + 1).to_string(),
                    ]
                } else {
                    vec![
                        "action".to_string(),
                        "go-to-tab-by-id".to_string(),
                        backend_tab_id.to_string(),
                    ]
                }
            }
            Self::CloseTab { backend_tab_id } => vec![
                "action".to_string(),
                "close-tab".to_string(),
                "--tab-id".to_string(),
                backend_tab_id.to_string(),
            ],
            Self::RenameTab { backend_tab_id, title } => vec![
                "action".to_string(),
                "rename-tab".to_string(),
                "--tab-id".to_string(),
                backend_tab_id.to_string(),
                title.clone(),
            ],
            Self::FocusPane { pane_ref } => {
                vec!["action".to_string(), "focus-pane-id".to_string(), pane_ref.clone()]
            }
            Self::ClosePane { pane_ref } => vec![
                "action".to_string(),
                "close-pane".to_string(),
                "--pane-id".to_string(),
                pane_ref.clone(),
            ],
            Self::WriteChars { pane_ref, chars } => vec![
                "action".to_string(),
                "write-chars".to_string(),
                "--pane-id".to_string(),
                pane_ref.clone(),
                chars.clone(),
            ],
            Self::Paste { pane_ref, text } => vec![
                "action".to_string(),
                "paste".to_string(),
                "--pane-id".to_string(),
                pane_ref.clone(),
                text.clone(),
            ],
            Self::SendKeys { pane_ref, keys } => {
                let mut args = vec![
                    "action".to_string(),
                    "send-keys".to_string(),
                    "--pane-id".to_string(),
                    pane_ref.clone(),
                ];
                args.extend(keys.clone());
                args
            }
        }
    }
}
