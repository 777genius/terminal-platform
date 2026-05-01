use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use terminal_backend_api::BackendError;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct ZellijTabRow {
    pub(crate) tab_id: u32,
    #[serde(default)]
    pub(crate) position: u32,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) active: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct ZellijPaneRow {
    pub(crate) id: u32,
    pub(crate) tab_id: u32,
    #[serde(default)]
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) is_plugin: bool,
    #[serde(default)]
    pub(crate) is_focused: bool,
    #[serde(default)]
    pub(crate) is_floating: bool,
    #[serde(default)]
    pub(crate) pane_x: u16,
    #[serde(default)]
    pub(crate) pane_y: u16,
    pub(crate) pane_rows: u16,
    pub(crate) pane_columns: u16,
}

impl ZellijPaneRow {
    pub(crate) fn backend_ref(&self) -> String {
        if self.is_plugin { format!("plugin_{}", self.id) } else { format!("terminal_{}", self.id) }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(crate) enum ZellijSubscribeEvent {
    PaneUpdate {
        pane_id: String,
        viewport: Vec<String>,
        #[serde(default)]
        _scrollback: Option<Vec<String>>,
        #[serde(default)]
        is_initial: bool,
    },
    PaneClosed {
        pane_id: String,
    },
}

pub(crate) fn parse_tabs_json(output: &str) -> Result<Vec<ZellijTabRow>, BackendError> {
    parse_json_array(output, "list-tabs")
}

pub(crate) fn parse_panes_json(output: &str) -> Result<Vec<ZellijPaneRow>, BackendError> {
    parse_json_array(output, "list-panes")
}

fn parse_json_array<T>(output: &str, command: &str) -> Result<Vec<T>, BackendError>
where
    T: DeserializeOwned,
{
    let payload: Value = serde_json::from_str(output).map_err(|error| {
        BackendError::internal(format!("invalid zellij {command} json: {error}"))
    })?;
    match payload {
        Value::Array(items) => serde_json::from_value(Value::Array(items)).map_err(|error| {
            BackendError::internal(format!("invalid zellij {command} json: {error}"))
        }),
        other => Err(BackendError::internal(format!(
            "unexpected zellij {command} payload while the session was settling: {}",
            summarize_payload(&other)
        ))),
    }
}

fn summarize_payload(payload: &Value) -> String {
    let rendered = payload.to_string();
    if rendered.chars().count() > 160 {
        format!("{}...", rendered.chars().take(160).collect::<String>())
    } else {
        rendered
    }
}
