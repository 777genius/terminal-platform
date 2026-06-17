#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalRectangularArea {
    pub top: u16,
    pub left: u16,
    pub bottom: Option<u16>,
    pub right: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalRectangularFillRequest {
    pub codepoint: u32,
    pub area: TerminalRectangularArea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalRectangularCopyRequest {
    pub source: TerminalRectangularArea,
    pub destination_top: u16,
    pub destination_left: u16,
}

impl TerminalRectangularArea {
    pub fn new(top: u16, left: u16, bottom: Option<u16>, right: Option<u16>) -> Option<Self> {
        let area = Self {
            top: top.max(1),
            left: left.max(1),
            bottom: bottom.map(|value| value.max(1)),
            right: right.map(|value| value.max(1)),
        };
        if area.bottom.is_some_and(|bottom| bottom < area.top)
            || area.right.is_some_and(|right| right < area.left)
        {
            return None;
        }
        Some(area)
    }
}

pub fn parse_terminal_rectangular_fill_request(
    payload: &[u8],
) -> Option<TerminalRectangularFillRequest> {
    let parts = rectangular_payload_parts(payload)?;
    let codepoint = parse_strict_u32_part(parts.first().copied())?;
    let area = TerminalRectangularArea::new(
        parse_strict_u16_part(parts.get(1).copied()).unwrap_or(1),
        parse_strict_u16_part(parts.get(2).copied()).unwrap_or(1),
        parse_strict_u16_part(parts.get(3).copied()),
        parse_strict_u16_part(parts.get(4).copied()),
    )?;
    Some(TerminalRectangularFillRequest { codepoint, area })
}

pub fn parse_terminal_rectangular_area(payload: &[u8]) -> Option<TerminalRectangularArea> {
    let parts = rectangular_payload_parts(payload)?;
    TerminalRectangularArea::new(
        parse_strict_u16_part(parts.first().copied()).unwrap_or(1),
        parse_strict_u16_part(parts.get(1).copied()).unwrap_or(1),
        parse_strict_u16_part(parts.get(2).copied()),
        parse_strict_u16_part(parts.get(3).copied()),
    )
}

pub fn parse_terminal_rectangular_copy_request(
    payload: &[u8],
) -> Option<TerminalRectangularCopyRequest> {
    let parts = rectangular_payload_parts(payload)?;
    let source = TerminalRectangularArea::new(
        parse_strict_u16_part(parts.first().copied()).unwrap_or(1),
        parse_strict_u16_part(parts.get(1).copied()).unwrap_or(1),
        parse_strict_u16_part(parts.get(2).copied()),
        parse_strict_u16_part(parts.get(3).copied()),
    )?;
    Some(TerminalRectangularCopyRequest {
        source,
        destination_top: parse_strict_u16_part(parts.get(5).copied()).unwrap_or(1),
        destination_left: parse_strict_u16_part(parts.get(6).copied()).unwrap_or(1),
    })
}

fn rectangular_payload_parts(payload: &[u8]) -> Option<Vec<&str>> {
    let text = std::str::from_utf8(payload).ok()?;
    let parameters = text.strip_suffix('$')?;
    Some(parameters.split(';').collect())
}

fn parse_strict_u16_part(part: Option<&str>) -> Option<u16> {
    Some(parse_strict_u32_part(part)?.min(i16::MAX as u32) as u16)
}

fn parse_strict_u32_part(part: Option<&str>) -> Option<u32> {
    let value = part?.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    value.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fill_rectangular_area_request() {
        assert_eq!(
            parse_terminal_rectangular_fill_request(b"88;2;3;4;5$"),
            Some(TerminalRectangularFillRequest {
                codepoint: 88,
                area: TerminalRectangularArea { top: 2, left: 3, bottom: Some(4), right: Some(5) },
            })
        );
    }

    #[test]
    fn parses_erase_rectangular_area_bounds() {
        assert_eq!(
            parse_terminal_rectangular_area(b";;;$"),
            Some(TerminalRectangularArea { top: 1, left: 1, bottom: None, right: None })
        );
        assert_eq!(parse_terminal_rectangular_area(b"3;1;2;4$"), None);
        assert_eq!(parse_terminal_rectangular_area(b"1;4;2;3$"), None);
        assert_eq!(parse_terminal_rectangular_area(b"1;2;3;4"), None);
    }

    #[test]
    fn parses_copy_rectangular_area_request() {
        assert_eq!(
            parse_terminal_rectangular_copy_request(b"2;3;4;5;1;6;7;1$"),
            Some(TerminalRectangularCopyRequest {
                source: TerminalRectangularArea {
                    top: 2,
                    left: 3,
                    bottom: Some(4),
                    right: Some(5),
                },
                destination_top: 6,
                destination_left: 7,
            })
        );
        assert_eq!(
            parse_terminal_rectangular_copy_request(b";;;;;;$"),
            Some(TerminalRectangularCopyRequest {
                source: TerminalRectangularArea { top: 1, left: 1, bottom: None, right: None },
                destination_top: 1,
                destination_left: 1,
            })
        );
        assert_eq!(parse_terminal_rectangular_copy_request(b"3;1;2;4;1;1;1;1$"), None);
        assert_eq!(parse_terminal_rectangular_copy_request(b"1;4;2;3;1;1;1;1$"), None);
    }
}
