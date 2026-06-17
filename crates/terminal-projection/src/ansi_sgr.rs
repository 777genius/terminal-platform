use crate::{ScreenTextStyle, ScreenUnderlineStyle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalRectangularAttributeRequest {
    pub top: u16,
    pub left: u16,
    pub bottom: Option<u16>,
    pub right: Option<u16>,
    pub actions: Vec<TerminalRectangularAttributeAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalRectangularAttributeAction {
    ResetCore,
    BoldOn,
    BoldOff,
    UnderlineOn,
    UnderlineOff,
    BlinkOn,
    BlinkOff,
    InverseOn,
    InverseOff,
    HiddenOn,
    HiddenOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalRectangularAttributeMode {
    Change,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnsiSgrStackAttributes {
    pub foreground: bool,
    pub background: bool,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub border: bool,
    pub baseline: bool,
}

impl AnsiSgrStackAttributes {
    pub fn all() -> Self {
        Self {
            foreground: true,
            background: true,
            bold: true,
            dim: true,
            italic: true,
            underline: true,
            blink: true,
            inverse: true,
            hidden: true,
            strikethrough: true,
            overline: true,
            border: true,
            baseline: true,
        }
    }

    fn is_empty(&self) -> bool {
        !self.foreground
            && !self.background
            && !self.bold
            && !self.dim
            && !self.italic
            && !self.underline
            && !self.blink
            && !self.inverse
            && !self.hidden
            && !self.strikethrough
            && !self.overline
            && !self.border
            && !self.baseline
    }
}

pub fn parse_xterm_sgr_stack_attributes(payload: &[u8]) -> Option<AnsiSgrStackAttributes> {
    if !payload.contains(&b'#') {
        return None;
    }

    let payload = std::str::from_utf8(payload).ok()?;
    let parameters = payload.replace('#', "");
    let parameters = parameters.trim();
    if parameters.is_empty() {
        return Some(AnsiSgrStackAttributes::all());
    }

    let mut attributes = AnsiSgrStackAttributes {
        foreground: false,
        background: false,
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        blink: false,
        inverse: false,
        hidden: false,
        strikethrough: false,
        overline: false,
        border: false,
        baseline: false,
    };

    for parameter in parameters.split(';') {
        match parameter.trim().parse::<u16>() {
            Ok(1) => attributes.bold = true,
            Ok(2) => attributes.dim = true,
            Ok(3) => attributes.italic = true,
            Ok(4) | Ok(21) => attributes.underline = true,
            Ok(5) => attributes.blink = true,
            Ok(7) => attributes.inverse = true,
            Ok(8) => attributes.hidden = true,
            Ok(9) => attributes.strikethrough = true,
            Ok(30) => attributes.foreground = true,
            Ok(31) => attributes.background = true,
            _ => {}
        }
    }

    (!attributes.is_empty()).then_some(attributes)
}

pub fn parse_terminal_rectangular_attribute_request(
    payload: &[u8],
) -> Option<TerminalRectangularAttributeRequest> {
    let text = std::str::from_utf8(payload).ok()?;
    let parameters = text.strip_suffix('$')?;
    let parts = parameters.split(';').collect::<Vec<_>>();
    let top = parse_strict_u16_part(parts.first().copied()).unwrap_or(1).max(1);
    let left = parse_strict_u16_part(parts.get(1).copied()).unwrap_or(1).max(1);
    let bottom = parse_strict_u16_part(parts.get(2).copied()).map(|value| value.max(1));
    let right = parse_strict_u16_part(parts.get(3).copied()).map(|value| value.max(1));
    if bottom.is_some_and(|bottom| bottom < top) || right.is_some_and(|right| right < left) {
        return None;
    }

    let actions = parse_terminal_rectangular_attribute_actions(parts.get(4..).unwrap_or_default())?;
    Some(TerminalRectangularAttributeRequest { top, left, bottom, right, actions })
}

pub fn apply_terminal_rectangular_attribute_actions(
    style: &mut ScreenTextStyle,
    actions: &[TerminalRectangularAttributeAction],
    mode: TerminalRectangularAttributeMode,
) {
    for action in actions {
        match (mode, action) {
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::ResetCore,
            ) => {
                style.bold = false;
                style.dim = false;
                style.underline = None;
                style.blink = false;
                style.inverse = false;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::BoldOn,
            ) => {
                style.bold = true;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::BoldOff,
            ) => {
                style.bold = false;
                style.dim = false;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::UnderlineOn,
            ) => {
                style.underline = Some(ScreenUnderlineStyle::Single);
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::UnderlineOff,
            ) => {
                style.underline = None;
                style.underline_color = None;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::BlinkOn,
            ) => {
                style.blink = true;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::BlinkOff,
            ) => {
                style.blink = false;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::InverseOn,
            ) => {
                style.inverse = true;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::InverseOff,
            ) => {
                style.inverse = false;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::HiddenOn,
            ) => {
                style.hidden = true;
            }
            (
                TerminalRectangularAttributeMode::Change,
                TerminalRectangularAttributeAction::HiddenOff,
            ) => {
                style.hidden = false;
            }
            (
                TerminalRectangularAttributeMode::Reverse,
                TerminalRectangularAttributeAction::ResetCore,
            ) => {
                style.bold = !style.bold;
                style.underline = if style.underline.is_some() {
                    None
                } else {
                    Some(ScreenUnderlineStyle::Single)
                };
                style.blink = !style.blink;
                style.inverse = !style.inverse;
            }
            (
                TerminalRectangularAttributeMode::Reverse,
                TerminalRectangularAttributeAction::BoldOn,
            ) => {
                style.bold = !style.bold;
            }
            (
                TerminalRectangularAttributeMode::Reverse,
                TerminalRectangularAttributeAction::UnderlineOn,
            ) => {
                style.underline = if style.underline.is_some() {
                    None
                } else {
                    Some(ScreenUnderlineStyle::Single)
                };
            }
            (
                TerminalRectangularAttributeMode::Reverse,
                TerminalRectangularAttributeAction::BlinkOn,
            ) => {
                style.blink = !style.blink;
            }
            (
                TerminalRectangularAttributeMode::Reverse,
                TerminalRectangularAttributeAction::InverseOn,
            ) => {
                style.inverse = !style.inverse;
            }
            (
                TerminalRectangularAttributeMode::Reverse,
                TerminalRectangularAttributeAction::HiddenOn,
            ) => {
                style.hidden = !style.hidden;
            }
            (TerminalRectangularAttributeMode::Reverse, _) => {}
        }
    }
}

fn parse_terminal_rectangular_attribute_actions(
    parts: &[&str],
) -> Option<Vec<TerminalRectangularAttributeAction>> {
    if parts.is_empty() || parts.iter().all(|part| part.trim().is_empty()) {
        return Some(vec![TerminalRectangularAttributeAction::ResetCore]);
    }

    let mut actions = Vec::new();
    for part in parts {
        match parse_strict_u16_part(Some(part)) {
            Some(0) => actions.push(TerminalRectangularAttributeAction::ResetCore),
            Some(1) => actions.push(TerminalRectangularAttributeAction::BoldOn),
            Some(4) => actions.push(TerminalRectangularAttributeAction::UnderlineOn),
            Some(5) => actions.push(TerminalRectangularAttributeAction::BlinkOn),
            Some(7) => actions.push(TerminalRectangularAttributeAction::InverseOn),
            Some(8) => actions.push(TerminalRectangularAttributeAction::HiddenOn),
            Some(22) => actions.push(TerminalRectangularAttributeAction::BoldOff),
            Some(24) => actions.push(TerminalRectangularAttributeAction::UnderlineOff),
            Some(25) => actions.push(TerminalRectangularAttributeAction::BlinkOff),
            Some(27) => actions.push(TerminalRectangularAttributeAction::InverseOff),
            Some(28) => actions.push(TerminalRectangularAttributeAction::HiddenOff),
            _ => {}
        }
    }

    (!actions.is_empty()).then_some(actions)
}

fn parse_strict_u16_part(part: Option<&str>) -> Option<u16> {
    let value = part?.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(value.parse::<u16>().ok()?.min(i16::MAX as u16))
}

pub fn parse_colon_sgr_underline_style(part: &str) -> Option<Option<ScreenUnderlineStyle>> {
    let fields = part.split(':').collect::<Vec<_>>();
    if fields.first() != Some(&"4") || fields.len() > 3 {
        return None;
    }

    let style = fields.iter().skip(1).find(|field| !field.is_empty())?;
    match style.parse::<u16>().ok()? {
        0 => Some(None),
        1 => Some(Some(ScreenUnderlineStyle::Single)),
        2 => Some(Some(ScreenUnderlineStyle::Double)),
        3 => Some(Some(ScreenUnderlineStyle::Curly)),
        4 => Some(Some(ScreenUnderlineStyle::Dotted)),
        5 => Some(Some(ScreenUnderlineStyle::Dashed)),
        _ => Some(Some(ScreenUnderlineStyle::Single)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colon_underline_style_subparams_with_xtermjs_fallback() {
        assert_eq!(parse_colon_sgr_underline_style("4:0"), Some(None));
        assert_eq!(parse_colon_sgr_underline_style("4::0"), Some(None));
        assert_eq!(
            parse_colon_sgr_underline_style("4:1"),
            Some(Some(ScreenUnderlineStyle::Single))
        );
        assert_eq!(
            parse_colon_sgr_underline_style("4::1"),
            Some(Some(ScreenUnderlineStyle::Single))
        );
        assert_eq!(
            parse_colon_sgr_underline_style("4:2"),
            Some(Some(ScreenUnderlineStyle::Double))
        );
        assert_eq!(parse_colon_sgr_underline_style("4:3"), Some(Some(ScreenUnderlineStyle::Curly)));
        assert_eq!(
            parse_colon_sgr_underline_style("4::3"),
            Some(Some(ScreenUnderlineStyle::Curly))
        );
        assert_eq!(
            parse_colon_sgr_underline_style("4:4"),
            Some(Some(ScreenUnderlineStyle::Dotted))
        );
        assert_eq!(
            parse_colon_sgr_underline_style("4:5"),
            Some(Some(ScreenUnderlineStyle::Dashed))
        );
        assert_eq!(
            parse_colon_sgr_underline_style("4:99"),
            Some(Some(ScreenUnderlineStyle::Single))
        );
        assert_eq!(parse_colon_sgr_underline_style("4:"), None);
        assert_eq!(parse_colon_sgr_underline_style("58:5:1"), None);
    }

    #[test]
    fn parses_xterm_sgr_stack_attribute_selectors() {
        assert_eq!(parse_xterm_sgr_stack_attributes(b"#"), Some(AnsiSgrStackAttributes::all()));

        let attributes = parse_xterm_sgr_stack_attributes(b"1;30#")
            .expect("selected SGR stack attributes should parse");
        assert!(attributes.bold);
        assert!(attributes.foreground);
        assert!(!attributes.background);
        assert!(!attributes.underline);

        let underline = parse_xterm_sgr_stack_attributes(b"21#")
            .expect("double underline selector should parse");
        assert!(underline.underline);

        assert_eq!(parse_xterm_sgr_stack_attributes(b"99#"), None);
        assert_eq!(parse_xterm_sgr_stack_attributes(b"1;30"), None);
    }

    #[test]
    fn parses_terminal_rectangular_attribute_requests() {
        assert_eq!(
            parse_terminal_rectangular_attribute_request(b"1;2;3;4;1;4$"),
            Some(TerminalRectangularAttributeRequest {
                top: 1,
                left: 2,
                bottom: Some(3),
                right: Some(4),
                actions: vec![
                    TerminalRectangularAttributeAction::BoldOn,
                    TerminalRectangularAttributeAction::UnderlineOn
                ],
            })
        );
        assert_eq!(
            parse_terminal_rectangular_attribute_request(b";;;$"),
            Some(TerminalRectangularAttributeRequest {
                top: 1,
                left: 1,
                bottom: None,
                right: None,
                actions: vec![TerminalRectangularAttributeAction::ResetCore],
            })
        );
        assert_eq!(parse_terminal_rectangular_attribute_request(b"3;1;2;4;1$"), None);
        assert_eq!(parse_terminal_rectangular_attribute_request(b"1;4;2;3;1$"), None);
        assert_eq!(parse_terminal_rectangular_attribute_request(b"1;2;3;4;99$"), None);
        assert_eq!(parse_terminal_rectangular_attribute_request(b"1;2;3;4;1"), None);
    }
}
