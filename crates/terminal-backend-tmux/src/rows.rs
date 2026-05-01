use crate::prelude::*;

pub(crate) struct TmuxWindowRow {
    pub(crate) window_index: u32,
    pub(crate) window_id: String,
    pub(crate) window_name: String,
    pub(crate) window_active: bool,
    pub(crate) window_layout: String,
}

impl TmuxWindowRow {
    pub(crate) fn parse(line: &str) -> Result<Self, BackendError> {
        let mut fields = line.split('\t');
        let window_index = parse_u32(next_field(&mut fields, "window_index")?, "window_index")?;
        let window_id = next_field(&mut fields, "window_id")?.to_string();
        let window_name = next_field(&mut fields, "window_name")?.to_string();
        let window_active = parse_bool(next_field(&mut fields, "window_active")?);
        let window_layout = next_field(&mut fields, "window_layout")?.to_string();

        Ok(Self { window_index, window_id, window_name, window_active, window_layout })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TmuxPaneRow {
    pub(crate) window_id: String,
    pub(crate) pane_id: String,
    pub(crate) pane_index: u32,
    pub(crate) pane_title: String,
    pub(crate) pane_active: bool,
    pub(crate) pane_width: u16,
    pub(crate) pane_height: u16,
}

impl TmuxPaneRow {
    pub(crate) fn parse(line: &str) -> Result<Self, BackendError> {
        let mut fields = line.split('\t');
        let window_id = next_field(&mut fields, "window_id")?.to_string();
        let pane_id = next_field(&mut fields, "pane_id")?.to_string();
        let pane_index = parse_u32(next_field(&mut fields, "pane_index")?, "pane_index")?;
        let pane_title = next_field(&mut fields, "pane_title")?.to_string();
        let pane_active = parse_bool(next_field(&mut fields, "pane_active")?);
        let pane_width = parse_u16(next_field(&mut fields, "pane_width")?, "pane_width")?;
        let pane_height = parse_u16(next_field(&mut fields, "pane_height")?, "pane_height")?;

        Ok(Self {
            window_id,
            pane_id,
            pane_index,
            pane_title,
            pane_active,
            pane_width,
            pane_height,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TmuxPaneTarget {
    pub(crate) target: String,
    pub(crate) title: Option<String>,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct TmuxTabTarget {
    pub(crate) target: String,
}
fn next_field<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<&'a str, BackendError> {
    fields.next().ok_or_else(|| BackendError::internal(format!("missing tmux field {name}")))
}

fn parse_bool(value: &str) -> bool {
    value == "1"
}

fn parse_u32(value: &str, name: &str) -> Result<u32, BackendError> {
    value.parse().map_err(|error| BackendError::internal(format!("invalid {name}: {error}")))
}

fn parse_u16(value: &str, name: &str) -> Result<u16, BackendError> {
    value.parse().map_err(|error| BackendError::internal(format!("invalid {name}: {error}")))
}

pub(crate) fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
