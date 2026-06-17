use crate::ScreenColor;

const MAX_TERMINAL_COLOR_NAME_CHARS: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiSgrColorTarget {
    Foreground,
    Background,
    Underline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalPaletteTarget {
    Ansi(u8),
    Foreground,
    Background,
    Cursor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalKittyColorControlOperation {
    Update(TerminalPaletteTarget, ScreenColor),
    Reset(TerminalPaletteTarget),
    QueryKnown { key: String, target: TerminalPaletteTarget },
    QueryUnknown { key: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKittyColorStackOperation {
    Push,
    Pop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalXtermColorStackOperation {
    Push,
    Pop,
    Store(usize),
    Restore(usize),
    Report,
}

pub fn terminal_default_palette_target_from_osc_code(code: &[u8]) -> Option<TerminalPaletteTarget> {
    match code {
        b"10" => Some(TerminalPaletteTarget::Foreground),
        b"11" => Some(TerminalPaletteTarget::Background),
        b"12" => Some(TerminalPaletteTarget::Cursor),
        _ => None,
    }
}

pub fn next_terminal_default_palette_target(
    target: TerminalPaletteTarget,
) -> Option<TerminalPaletteTarget> {
    match target {
        TerminalPaletteTarget::Foreground => Some(TerminalPaletteTarget::Background),
        TerminalPaletteTarget::Background => Some(TerminalPaletteTarget::Cursor),
        TerminalPaletteTarget::Cursor | TerminalPaletteTarget::Ansi(_) => None,
    }
}

pub fn parse_legacy_linux_console_palette_update(payload: &[u8]) -> Option<(u8, ScreenColor)> {
    match parse_terminal_osc_p_palette_update(payload)? {
        (TerminalPaletteTarget::Ansi(index), color) => Some((index, color)),
        _ => None,
    }
}

pub fn parse_terminal_osc_p_palette_update(
    payload: &[u8],
) -> Option<(TerminalPaletteTarget, ScreenColor)> {
    let rest = payload.strip_prefix(b"P")?;
    if rest.len() != 7 || !rest[1..].iter().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let target = parse_terminal_osc_p_palette_target(*rest.first()?)?;
    Some((
        target,
        ScreenColor::Rgb {
            r: parse_ascii_hex_byte(&rest[1..3])?,
            g: parse_ascii_hex_byte(&rest[3..5])?,
            b: parse_ascii_hex_byte(&rest[5..7])?,
        },
    ))
}

pub fn parse_iterm2_set_colors_update(
    payload: &[u8],
) -> Option<(TerminalPaletteTarget, ScreenColor)> {
    let body = payload.strip_prefix(b"1337;SetColors=")?;
    let separator = body.iter().position(|byte| *byte == b'=')?;
    let (key, value_with_separator) = body.split_at(separator);
    let value = value_with_separator.get(1..)?;
    let target = iterm2_set_colors_palette_target(key)?;
    let color = iterm2_set_colors_color_spec(std::str::from_utf8(value).ok()?.trim())?;
    Some((target, color))
}

pub fn parse_terminal_kitty_color_control(
    payload: &[u8],
) -> Option<Vec<TerminalKittyColorControlOperation>> {
    let body = payload.strip_prefix(b"21;")?;
    let mut operations = Vec::new();
    for field in body.split(|byte| *byte == b';') {
        if field.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = match field.iter().position(|byte| *byte == b'=') {
            Some(separator) => (&field[..separator], Some(&field[separator + 1..])),
            None => (field, None),
        };
        let Some(key) = terminal_kitty_color_control_key(raw_key) else {
            continue;
        };
        let target = terminal_kitty_color_control_target(key.as_bytes());
        match raw_value {
            Some(b"?") => match target {
                Some(target) => {
                    operations.push(TerminalKittyColorControlOperation::QueryKnown { key, target });
                }
                None => operations.push(TerminalKittyColorControlOperation::QueryUnknown { key }),
            },
            Some([]) => {
                if let Some(target) = target {
                    operations.push(TerminalKittyColorControlOperation::Reset(target));
                }
            }
            Some(value) => {
                if let Some(target) = target
                    && let Ok(spec) = std::str::from_utf8(value)
                    && let Some(color) = parse_terminal_color_spec(spec)
                {
                    operations.push(TerminalKittyColorControlOperation::Update(target, color));
                }
            }
            None => {
                if let Some(target) = target {
                    operations.push(TerminalKittyColorControlOperation::Reset(target));
                }
            }
        }
    }

    (!operations.is_empty()).then_some(operations)
}

pub fn parse_terminal_kitty_color_stack(
    payload: &[u8],
) -> Option<TerminalKittyColorStackOperation> {
    match payload {
        b"30001" => Some(TerminalKittyColorStackOperation::Push),
        b"30101" => Some(TerminalKittyColorStackOperation::Pop),
        _ => None,
    }
}

pub fn parse_terminal_xterm_color_stack(
    payload: &[u8],
    final_byte: u8,
) -> Option<Vec<TerminalXtermColorStackOperation>> {
    if final_byte == b'R' {
        return (payload == b"#").then_some(vec![TerminalXtermColorStackOperation::Report]);
    }
    if !matches!(final_byte, b'P' | b'Q') {
        return None;
    }
    let parameters = payload.strip_suffix(b"#")?;
    let slots = parse_xterm_color_stack_slots(parameters)?
        .into_iter()
        .chain(parameters.is_empty().then_some(0));
    let operations = slots
        .map(|slot| match (final_byte, slot) {
            (b'P', 0) => TerminalXtermColorStackOperation::Push,
            (b'Q', 0) => TerminalXtermColorStackOperation::Pop,
            (b'P', slot) => TerminalXtermColorStackOperation::Store(slot),
            (b'Q', slot) => TerminalXtermColorStackOperation::Restore(slot),
            _ => unreachable!("guarded by final byte match"),
        })
        .collect::<Vec<_>>();
    (!operations.is_empty()).then_some(operations)
}

fn parse_xterm_color_stack_slots(parameters: &[u8]) -> Option<Vec<usize>> {
    if parameters.is_empty() {
        return Some(Vec::new());
    }
    let text = std::str::from_utf8(parameters).ok()?;
    let mut slots = Vec::new();
    for value in text.split(';') {
        if value.is_empty() {
            slots.push(0);
            continue;
        }
        let slot = value.parse::<usize>().ok()?;
        if slot > 10 {
            return None;
        }
        slots.push(slot);
    }
    Some(slots)
}

fn iterm2_set_colors_palette_target(key: &[u8]) -> Option<TerminalPaletteTarget> {
    match key {
        b"fg" => Some(TerminalPaletteTarget::Foreground),
        b"bg" => Some(TerminalPaletteTarget::Background),
        b"curbg" => Some(TerminalPaletteTarget::Cursor),
        b"black" => Some(TerminalPaletteTarget::Ansi(0)),
        b"red" => Some(TerminalPaletteTarget::Ansi(1)),
        b"green" => Some(TerminalPaletteTarget::Ansi(2)),
        b"yellow" => Some(TerminalPaletteTarget::Ansi(3)),
        b"blue" => Some(TerminalPaletteTarget::Ansi(4)),
        b"magenta" => Some(TerminalPaletteTarget::Ansi(5)),
        b"cyan" => Some(TerminalPaletteTarget::Ansi(6)),
        b"white" => Some(TerminalPaletteTarget::Ansi(7)),
        b"br_black" => Some(TerminalPaletteTarget::Ansi(8)),
        b"br_red" => Some(TerminalPaletteTarget::Ansi(9)),
        b"br_green" => Some(TerminalPaletteTarget::Ansi(10)),
        b"br_yellow" => Some(TerminalPaletteTarget::Ansi(11)),
        b"br_blue" => Some(TerminalPaletteTarget::Ansi(12)),
        b"br_magenta" => Some(TerminalPaletteTarget::Ansi(13)),
        b"br_cyan" => Some(TerminalPaletteTarget::Ansi(14)),
        b"br_white" => Some(TerminalPaletteTarget::Ansi(15)),
        _ => None,
    }
}

fn terminal_kitty_color_control_target(key: &[u8]) -> Option<TerminalPaletteTarget> {
    match key {
        b"foreground" => Some(TerminalPaletteTarget::Foreground),
        b"background" => Some(TerminalPaletteTarget::Background),
        b"cursor" => Some(TerminalPaletteTarget::Cursor),
        _ => {
            let index = std::str::from_utf8(key).ok()?.parse::<u16>().ok()?;
            (index <= u16::from(u8::MAX)).then_some(TerminalPaletteTarget::Ansi(index as u8))
        }
    }
}

fn terminal_kitty_color_control_key(raw: &[u8]) -> Option<String> {
    if raw.is_empty() || raw.len() > 64 {
        return None;
    }
    raw.iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        .then(|| String::from_utf8(raw.to_vec()).ok())
        .flatten()
}

fn iterm2_set_colors_color_spec(spec: &str) -> Option<ScreenColor> {
    if terminal_compact_hex_color_spec(spec).is_some() {
        return parse_terminal_color_spec(spec);
    }

    let (space, value) = spec.split_once(':')?;
    if !matches!(space, "rgb" | "srgb" | "p3") {
        return None;
    }
    terminal_compact_hex_color_spec(value)?;
    parse_terminal_color_spec(spec)
}

fn parse_terminal_osc_p_palette_target(value: u8) -> Option<TerminalPaletteTarget> {
    parse_ascii_hex_digit(value).map(TerminalPaletteTarget::Ansi).or(match value {
        b'g' | b'G' => Some(TerminalPaletteTarget::Foreground),
        b'h' | b'H' => Some(TerminalPaletteTarget::Background),
        b'l' | b'L' => Some(TerminalPaletteTarget::Cursor),
        _ => None,
    })
}

pub fn is_legacy_linux_console_palette_reset(payload: &[u8]) -> bool {
    payload == b"R"
}

pub fn parse_terminal_color_spec(raw: &str) -> Option<ScreenColor> {
    let text = raw.trim();
    if text.is_empty() || text == "?" {
        return None;
    }
    if let Some(hex) = text.strip_prefix('#') {
        return terminal_hex_color_spec(hex).or_else(|| terminal_css_hex_alpha_color_spec(hex));
    }
    if text.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgb:")) {
        return terminal_rgb_color_spec(&text[4..]);
    }
    if text.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("srgb:")) {
        return terminal_compact_hex_color_spec(&text[5..]);
    }
    if text.get(..3).is_some_and(|prefix| prefix.eq_ignore_ascii_case("p3:")) {
        return terminal_compact_hex_color_spec(&text[3..]);
    }
    if text.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgb("))
        && text.ends_with(')')
    {
        return terminal_css_rgb_color_spec(&text[4..text.len() - 1]);
    }
    if text.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgba:")) {
        return terminal_rgba_color_spec(&text[5..]);
    }
    if text.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgba("))
        && text.ends_with(')')
    {
        return terminal_css_rgba_color_spec(&text[5..text.len() - 1]);
    }
    if text.get(..6).is_some_and(|prefix| prefix.eq_ignore_ascii_case("color("))
        && text.ends_with(')')
    {
        return terminal_css_color_function_spec(&text[6..text.len() - 1]);
    }
    if text.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgbi:")) {
        return terminal_rgbi_color_spec(&text[5..]);
    }
    if let Some(color) = terminal_compact_hex_color_spec(text) {
        return Some(color);
    }
    Some(ScreenColor::Named { name: text.chars().take(MAX_TERMINAL_COLOR_NAME_CHARS).collect() })
}

pub fn parse_colon_sgr_color_part(part: &str) -> Option<(AnsiSgrColorTarget, ScreenColor)> {
    let fields = part.split(':').collect::<Vec<_>>();
    let target = sgr_color_target(fields.first()?.parse::<u16>().ok()?)?;
    let color = parse_colon_sgr_color_fields(&fields[1..])?;
    Some((target, color))
}

pub fn parse_semicolon_sgr_color_fields(parts: &[&str]) -> Option<(ScreenColor, usize)> {
    let mode = parts.first()?.parse::<u16>().ok()?;
    match mode {
        5 => {
            let index_offset =
                if parts.get(1).is_some_and(|field| field.is_empty()) { 2 } else { 1 };
            let index = parse_sgr_u8(parts.get(index_offset)?)?;
            Some((ScreenColor::Indexed { index }, index_offset + 1))
        }
        2 => {
            let (offset, consumed) = semicolon_color_component_offset(parts, 3)?;
            let r = parse_sgr_u8(parts.get(offset)?)?;
            let g = parse_sgr_u8(parts.get(offset + 1)?)?;
            let b = parse_sgr_u8(parts.get(offset + 2)?)?;
            Some((ScreenColor::Rgb { r, g, b }, consumed))
        }
        3 => {
            let (offset, consumed) = semicolon_color_component_offset(parts, 3)?;
            let c = parse_sgr_u8(parts.get(offset)?)?;
            let m = parse_sgr_u8(parts.get(offset + 1)?)?;
            let y = parse_sgr_u8(parts.get(offset + 2)?)?;
            Some((screen_color_from_cmy(c, m, y), consumed))
        }
        4 => {
            let (offset, consumed) = semicolon_color_component_offset(parts, 4)?;
            let c = parse_sgr_u8(parts.get(offset)?)?;
            let m = parse_sgr_u8(parts.get(offset + 1)?)?;
            let y = parse_sgr_u8(parts.get(offset + 2)?)?;
            let k = parse_sgr_u8(parts.get(offset + 3)?)?;
            Some((screen_color_from_cmyk(c, m, y, k), consumed))
        }
        6 => {
            let (offset, consumed) = semicolon_color_component_offset(parts, 4)?;
            let r = parse_sgr_u8(parts.get(offset)?)?;
            let g = parse_sgr_u8(parts.get(offset + 1)?)?;
            let b = parse_sgr_u8(parts.get(offset + 2)?)?;
            let _alpha = parse_sgr_u8(parts.get(offset + 3)?)?;
            Some((ScreenColor::Rgb { r, g, b }, consumed))
        }
        _ => None,
    }
}

pub fn parse_colon_sgr_color_fields(fields: &[&str]) -> Option<ScreenColor> {
    let mode_offset = fields.iter().position(|field| !field.is_empty())?;
    let mode = fields.get(mode_offset)?.parse::<u16>().ok()?;
    let components = fields[mode_offset + 1..]
        .iter()
        .copied()
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    match mode {
        5 => {
            let index = parse_sgr_u8(components.first()?)?;
            Some(ScreenColor::Indexed { index })
        }
        2 => {
            let rgb_offset = if components.len() >= 4 { 1 } else { 0 };
            let r = parse_sgr_u8(components.get(rgb_offset)?)?;
            let g = parse_sgr_u8(components.get(rgb_offset + 1)?)?;
            let b = parse_sgr_u8(components.get(rgb_offset + 2)?)?;
            Some(ScreenColor::Rgb { r, g, b })
        }
        3 => {
            let cmy_offset = if components.len() >= 4 { 1 } else { 0 };
            let c = parse_sgr_u8(components.get(cmy_offset)?)?;
            let m = parse_sgr_u8(components.get(cmy_offset + 1)?)?;
            let y = parse_sgr_u8(components.get(cmy_offset + 2)?)?;
            Some(screen_color_from_cmy(c, m, y))
        }
        4 => {
            let cmyk_offset = if components.len() >= 5 { 1 } else { 0 };
            let c = parse_sgr_u8(components.get(cmyk_offset)?)?;
            let m = parse_sgr_u8(components.get(cmyk_offset + 1)?)?;
            let y = parse_sgr_u8(components.get(cmyk_offset + 2)?)?;
            let k = parse_sgr_u8(components.get(cmyk_offset + 3)?)?;
            Some(screen_color_from_cmyk(c, m, y, k))
        }
        6 => {
            let rgb_offset = if components.len() >= 5 { 1 } else { 0 };
            let r = parse_sgr_u8(components.get(rgb_offset)?)?;
            let g = parse_sgr_u8(components.get(rgb_offset + 1)?)?;
            let b = parse_sgr_u8(components.get(rgb_offset + 2)?)?;
            let _alpha = parse_sgr_u8(components.get(rgb_offset + 3)?)?;
            Some(ScreenColor::Rgb { r, g, b })
        }
        _ => None,
    }
}

fn sgr_color_target(code: u16) -> Option<AnsiSgrColorTarget> {
    match code {
        38 => Some(AnsiSgrColorTarget::Foreground),
        48 => Some(AnsiSgrColorTarget::Background),
        58 => Some(AnsiSgrColorTarget::Underline),
        _ => None,
    }
}

fn parse_sgr_u8(value: &str) -> Option<u8> {
    Some(value.parse::<u16>().ok()?.min(u16::from(u8::MAX)) as u8)
}

fn semicolon_color_component_offset(
    parts: &[&str],
    component_count: usize,
) -> Option<(usize, usize)> {
    let standard_consumed = 1 + component_count;
    if parts.len() < standard_consumed {
        return None;
    }
    if parts.get(1).is_some_and(|field| field.is_empty()) && parts.len() > standard_consumed {
        return Some((2, standard_consumed + 1));
    }
    Some((1, standard_consumed))
}

fn terminal_hex_color_spec(hex: &str) -> Option<ScreenColor> {
    if hex.is_empty() || !hex.len().is_multiple_of(3) {
        return None;
    }
    let component_width = hex.len() / 3;
    if !(1..=4).contains(&component_width) || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(ScreenColor::Rgb {
        r: parse_scaled_hex_color_component(&hex[0..component_width])?,
        g: parse_scaled_hex_color_component(&hex[component_width..component_width * 2])?,
        b: parse_scaled_hex_color_component(&hex[component_width * 2..component_width * 3])?,
    })
}

fn terminal_css_hex_alpha_color_spec(hex: &str) -> Option<ScreenColor> {
    if !matches!(hex.len(), 4 | 8) || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let component_width = hex.len() / 4;
    parse_scaled_hex_color_component(&hex[component_width * 3..component_width * 4])?;
    Some(ScreenColor::Rgb {
        r: parse_scaled_hex_color_component(&hex[0..component_width])?,
        g: parse_scaled_hex_color_component(&hex[component_width..component_width * 2])?,
        b: parse_scaled_hex_color_component(&hex[component_width * 2..component_width * 3])?,
    })
}

fn terminal_rgb_color_spec(spec: &str) -> Option<ScreenColor> {
    if let Some(color) = terminal_compact_hex_color_spec(spec) {
        return Some(color);
    }

    let parts = spec.split('/').collect::<Vec<_>>();
    if parts.len() != 3 {
        return None;
    }
    Some(ScreenColor::Rgb {
        r: parse_scaled_hex_color_component(parts[0])?,
        g: parse_scaled_hex_color_component(parts[1])?,
        b: parse_scaled_hex_color_component(parts[2])?,
    })
}

fn terminal_rgbi_color_spec(spec: &str) -> Option<ScreenColor> {
    let parts = spec.split('/').collect::<Vec<_>>();
    if parts.len() != 3 {
        return None;
    }
    Some(ScreenColor::Rgb {
        r: parse_rgbi_color_component(parts[0])?,
        g: parse_rgbi_color_component(parts[1])?,
        b: parse_rgbi_color_component(parts[2])?,
    })
}

fn terminal_css_rgb_color_spec(spec: &str) -> Option<ScreenColor> {
    let (parts, alpha) = parse_css_color_function_components(spec)?;
    if parts.len() != 3 || alpha.is_some_and(|value| parse_css_alpha_component(value).is_none()) {
        return None;
    }
    Some(ScreenColor::Rgb {
        r: parse_css_rgb_component(parts[0])?,
        g: parse_css_rgb_component(parts[1])?,
        b: parse_css_rgb_component(parts[2])?,
    })
}

fn terminal_rgba_color_spec(spec: &str) -> Option<ScreenColor> {
    let parts = spec.split('/').collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }
    parse_scaled_hex_color_component(parts[3])?;
    Some(ScreenColor::Rgb {
        r: parse_scaled_hex_color_component(parts[0])?,
        g: parse_scaled_hex_color_component(parts[1])?,
        b: parse_scaled_hex_color_component(parts[2])?,
    })
}

fn terminal_css_rgba_color_spec(spec: &str) -> Option<ScreenColor> {
    let (parts, alpha) = parse_css_color_function_components(spec)?;
    let alpha = alpha?;
    if parts.len() != 3 {
        return None;
    }
    parse_css_alpha_component(alpha)?;
    Some(ScreenColor::Rgb {
        r: parse_css_rgb_component(parts[0])?,
        g: parse_css_rgb_component(parts[1])?,
        b: parse_css_rgb_component(parts[2])?,
    })
}

fn terminal_css_color_function_spec(spec: &str) -> Option<ScreenColor> {
    let mut slash_parts = spec.split('/');
    let color_parts = slash_parts.next()?.split_whitespace().collect::<Vec<_>>();
    let alpha = slash_parts.next().map(str::trim);
    if slash_parts.next().is_some() || color_parts.len() != 4 {
        return None;
    }
    if !color_parts[0].eq_ignore_ascii_case("srgb") {
        return None;
    }
    if alpha.is_some_and(|value| parse_css_alpha_component(value).is_none()) {
        return None;
    }
    Some(ScreenColor::Rgb {
        r: parse_css_unit_rgb_component(color_parts[1])?,
        g: parse_css_unit_rgb_component(color_parts[2])?,
        b: parse_css_unit_rgb_component(color_parts[3])?,
    })
}

fn parse_css_color_function_components(spec: &str) -> Option<(Vec<&str>, Option<&str>)> {
    if spec.contains(',') {
        let parts = spec.split(',').map(str::trim).collect::<Vec<_>>();
        return match parts.as_slice() {
            [r, g, b] => Some((vec![*r, *g, *b], None)),
            [r, g, b, a] => Some((vec![*r, *g, *b], Some(*a))),
            _ => None,
        };
    }

    let mut slash_parts = spec.split('/');
    let color_parts = slash_parts.next()?.split_whitespace().collect::<Vec<_>>();
    let alpha = slash_parts.next().map(str::trim);
    if slash_parts.next().is_some() {
        return None;
    }
    Some((color_parts, alpha))
}

fn parse_rgbi_color_component(component: &str) -> Option<u8> {
    let value = component.parse::<f32>().ok()?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return None;
    }
    Some((value.mul_add(255.0, 0.5).floor() as u16).min(u16::from(u8::MAX)) as u8)
}

fn parse_css_rgb_component(component: &str) -> Option<u8> {
    let component = component.trim();
    let value = if let Some(percent) = component.strip_suffix('%') {
        let percent = percent.trim().parse::<f32>().ok()?;
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            return None;
        }
        percent * 255.0 / 100.0
    } else {
        let value = component.parse::<f32>().ok()?;
        if !value.is_finite() || !(0.0..=255.0).contains(&value) {
            return None;
        }
        value
    };
    Some((value + 0.5).floor().min(255.0) as u8)
}

fn parse_css_unit_rgb_component(component: &str) -> Option<u8> {
    let component = component.trim();
    let value = if let Some(percent) = component.strip_suffix('%') {
        let percent = percent.trim().parse::<f32>().ok()?;
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            return None;
        }
        percent * 255.0 / 100.0
    } else {
        let value = component.parse::<f32>().ok()?;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return None;
        }
        value * 255.0
    };
    Some((value + 0.5).floor().min(255.0) as u8)
}

fn parse_css_alpha_component(component: &str) -> Option<()> {
    let component = component.trim();
    if let Some(percent) = component.strip_suffix('%') {
        let value = percent.trim().parse::<f32>().ok()?;
        return (value.is_finite() && (0.0..=100.0).contains(&value)).then_some(());
    }
    let value = component.parse::<f32>().ok()?;
    (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(())
}

fn parse_scaled_hex_color_component(component: &str) -> Option<u8> {
    if component.is_empty()
        || component.len() > 4
        || !component.chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return None;
    }
    let value = u32::from_str_radix(component, 16).ok()?;
    let max = (1u32 << (component.len() * 4)) - 1;
    Some(((value * 255 + max / 2) / max).min(255) as u8)
}

fn terminal_compact_hex_color_spec(spec: &str) -> Option<ScreenColor> {
    let text = spec.trim();
    if !matches!(text.len(), 3 | 6) || !text.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    terminal_hex_color_spec(text)
}

fn parse_ascii_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn parse_ascii_hex_byte(value: &[u8]) -> Option<u8> {
    if value.len() != 2 {
        return None;
    }
    Some((parse_ascii_hex_digit(value[0])? << 4) | parse_ascii_hex_digit(value[1])?)
}

fn screen_color_from_cmy(c: u8, m: u8, y: u8) -> ScreenColor {
    ScreenColor::Rgb { r: 255 - c, g: 255 - m, b: 255 - y }
}

fn screen_color_from_cmyk(c: u8, m: u8, y: u8, k: u8) -> ScreenColor {
    ScreenColor::Rgb {
        r: cmyk_component_to_rgb(c, k),
        g: cmyk_component_to_rgb(m, k),
        b: cmyk_component_to_rgb(y, k),
    }
}

fn cmyk_component_to_rgb(component: u8, black: u8) -> u8 {
    let component = u16::from(255 - component);
    let black = u16::from(255 - black);
    ((component * black + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_linux_console_palette_updates() {
        assert_eq!(
            parse_legacy_linux_console_palette_update(b"P1aabbcc"),
            Some((1, ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc }))
        );
        assert_eq!(
            parse_legacy_linux_console_palette_update(b"PFAABBCC"),
            Some((15, ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc }))
        );
        assert!(is_legacy_linux_console_palette_reset(b"R"));
        assert!(!is_legacy_linux_console_palette_reset(b"104"));
        assert_eq!(parse_legacy_linux_console_palette_update(b"P10aabbcc"), None);
        assert_eq!(parse_legacy_linux_console_palette_update(b"P1aabbc"), None);
        assert_eq!(parse_legacy_linux_console_palette_update(b"P1aabbcg"), None);
        assert_eq!(parse_legacy_linux_console_palette_update(b"1aabbcc"), None);
    }

    #[test]
    fn parses_iterm2_osc_p_default_palette_updates() {
        assert_eq!(
            parse_terminal_osc_p_palette_update(b"Pg112233"),
            Some((
                TerminalPaletteTarget::Foreground,
                ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 }
            ))
        );
        assert_eq!(
            parse_terminal_osc_p_palette_update(b"PH445566"),
            Some((
                TerminalPaletteTarget::Background,
                ScreenColor::Rgb { r: 0x44, g: 0x55, b: 0x66 }
            ))
        );
        assert_eq!(
            parse_terminal_osc_p_palette_update(b"Pl778899"),
            Some((TerminalPaletteTarget::Cursor, ScreenColor::Rgb { r: 0x77, g: 0x88, b: 0x99 }))
        );
        assert_eq!(parse_terminal_osc_p_palette_update(b"Pi112233"), None);
        assert_eq!(parse_legacy_linux_console_palette_update(b"Pg112233"), None);
    }

    #[test]
    fn parses_xparse_color_specs_used_by_osc_palette_sequences() {
        assert_eq!(
            parse_terminal_color_spec("#123456"),
            Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 })
        );
        assert_eq!(
            parse_terminal_color_spec("#abc"),
            Some(ScreenColor::Rgb { r: 170, g: 187, b: 204 })
        );
        assert_eq!(
            parse_terminal_color_spec("#abcd"),
            Some(ScreenColor::Rgb { r: 170, g: 187, b: 204 })
        );
        assert_eq!(
            parse_terminal_color_spec("#11223344"),
            Some(ScreenColor::Rgb { r: 17, g: 34, b: 51 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgb:12/34/56"),
            Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgb:1212/3434/5656"),
            Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgb:abc"),
            Some(ScreenColor::Rgb { r: 170, g: 187, b: 204 })
        );
        assert_eq!(
            parse_terminal_color_spec("srgb:0f1020"),
            Some(ScreenColor::Rgb { r: 15, g: 16, b: 32 })
        );
        assert_eq!(
            parse_terminal_color_spec("p3:102030"),
            Some(ScreenColor::Rgb { r: 16, g: 32, b: 48 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgb(12, 34, 56)"),
            Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgb(12 34 56 / 40%)"),
            Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgb(100% 50% 0%)"),
            Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgbi:1.0/0.5/0.0"),
            Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgba:1212/3434/5656/7878"),
            Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgba(12, 34, 56, 0.4)"),
            Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        );
        assert_eq!(
            parse_terminal_color_spec("rgba(12 34 56 / 40%)"),
            Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        );
        assert_eq!(
            parse_terminal_color_spec("color(srgb 1 0.5 0 / 40%)"),
            Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
        );
        assert_eq!(
            parse_terminal_color_spec("color(srgb 100% 50% 0%)"),
            Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
        );
        assert_eq!(
            parse_terminal_color_spec(" Light Blue "),
            Some(ScreenColor::Named { name: "Light Blue".to_string() })
        );
        assert_eq!(parse_terminal_color_spec("?"), None);
        assert_eq!(parse_terminal_color_spec("#12"), None);
        assert_eq!(parse_terminal_color_spec("rgb:12/34"), None);
        assert_eq!(parse_terminal_color_spec("rgb(12, 34)"), None);
        assert_eq!(parse_terminal_color_spec("rgb(12, 34, 999)"), None);
        assert_eq!(parse_terminal_color_spec("rgb(12 34 56 / 140%)"), None);
        assert_eq!(parse_terminal_color_spec("srgb:nothex"), None);
        assert_eq!(parse_terminal_color_spec("p3:abcd"), None);
        assert_eq!(parse_terminal_color_spec("rgba:12/34/56"), None);
        assert_eq!(parse_terminal_color_spec("rgba(12 34 56)"), None);
        assert_eq!(parse_terminal_color_spec("rgba(12, 34, 56, 1.5)"), None);
        assert_eq!(parse_terminal_color_spec("color(display-p3 1 0 0)"), None);
        assert_eq!(parse_terminal_color_spec("color(srgb 1.2 0 0)"), None);
        assert_eq!(parse_terminal_color_spec("rgbi:1.1/0/0"), None);
    }

    #[test]
    fn parses_iterm2_set_colors_palette_updates() {
        assert_eq!(
            parse_iterm2_set_colors_update(b"1337;SetColors=fg=f0f"),
            Some((TerminalPaletteTarget::Foreground, ScreenColor::Rgb { r: 255, g: 0, b: 255 }))
        );
        assert_eq!(
            parse_iterm2_set_colors_update(b"1337;SetColors=bg=112233"),
            Some((TerminalPaletteTarget::Background, ScreenColor::Rgb { r: 17, g: 34, b: 51 }))
        );
        assert_eq!(
            parse_iterm2_set_colors_update(b"1337;SetColors=curbg=srgb:445566"),
            Some((TerminalPaletteTarget::Cursor, ScreenColor::Rgb { r: 68, g: 85, b: 102 }))
        );
        assert_eq!(
            parse_iterm2_set_colors_update(b"1337;SetColors=red=rgb:0f0"),
            Some((TerminalPaletteTarget::Ansi(1), ScreenColor::Rgb { r: 0, g: 255, b: 0 }))
        );
        assert_eq!(
            parse_iterm2_set_colors_update(b"1337;SetColors=br_blue=p3:102030"),
            Some((TerminalPaletteTarget::Ansi(12), ScreenColor::Rgb { r: 16, g: 32, b: 48 }))
        );
        assert_eq!(parse_iterm2_set_colors_update(b"1337;SetColors=link=fff"), None);
        assert_eq!(parse_iterm2_set_colors_update(b"1337;SetColors=curfg=fff"), None);
        assert_eq!(parse_iterm2_set_colors_update(b"1337;SetColors=red=blue"), None);
        assert_eq!(parse_iterm2_set_colors_update(b"1337;SetColors=red=rgb:00/ff/00"), None);
        assert_eq!(parse_iterm2_set_colors_update(b"1337;SetColors=red=display-p3:00ff00"), None);
        assert_eq!(parse_iterm2_set_colors_update(b"1337;SetColors=red"), None);
    }

    #[test]
    fn parses_kitty_color_control_updates_resets_and_queries() {
        assert_eq!(
            parse_terminal_kitty_color_control(
                b"21;foreground=#112233;background=rgb:44/55/66;12=rgbi:1.0/0.5/0.0;cursor;selection_background=?"
            ),
            Some(vec![
                TerminalKittyColorControlOperation::Update(
                    TerminalPaletteTarget::Foreground,
                    ScreenColor::Rgb { r: 17, g: 34, b: 51 },
                ),
                TerminalKittyColorControlOperation::Update(
                    TerminalPaletteTarget::Background,
                    ScreenColor::Rgb { r: 68, g: 85, b: 102 },
                ),
                TerminalKittyColorControlOperation::Update(
                    TerminalPaletteTarget::Ansi(12),
                    ScreenColor::Rgb { r: 255, g: 128, b: 0 },
                ),
                TerminalKittyColorControlOperation::Reset(TerminalPaletteTarget::Cursor),
                TerminalKittyColorControlOperation::QueryUnknown {
                    key: "selection_background".to_string(),
                },
            ])
        );
        assert_eq!(
            parse_terminal_kitty_color_control(b"21;foreground=?;999=#ffffff;bad-key=?"),
            Some(vec![TerminalKittyColorControlOperation::QueryKnown {
                key: "foreground".to_string(),
                target: TerminalPaletteTarget::Foreground,
            }])
        );
        assert_eq!(parse_terminal_kitty_color_control(b"10;foreground=#112233"), None);
    }

    #[test]
    fn parses_kitty_color_stack_push_and_pop() {
        assert_eq!(
            parse_terminal_kitty_color_stack(b"30001"),
            Some(TerminalKittyColorStackOperation::Push)
        );
        assert_eq!(
            parse_terminal_kitty_color_stack(b"30101"),
            Some(TerminalKittyColorStackOperation::Pop)
        );
        assert_eq!(parse_terminal_kitty_color_stack(b"30001;extra"), None);
        assert_eq!(parse_terminal_kitty_color_stack(b"21;foreground=#112233"), None);
    }

    #[test]
    fn parses_xterm_color_stack_push_pop_and_addressed_slots() {
        assert_eq!(
            parse_terminal_xterm_color_stack(b"#", b'P'),
            Some(vec![TerminalXtermColorStackOperation::Push])
        );
        assert_eq!(
            parse_terminal_xterm_color_stack(b"#", b'Q'),
            Some(vec![TerminalXtermColorStackOperation::Pop])
        );
        assert_eq!(
            parse_terminal_xterm_color_stack(b"1#", b'P'),
            Some(vec![TerminalXtermColorStackOperation::Store(1)])
        );
        assert_eq!(
            parse_terminal_xterm_color_stack(b"1;2#", b'Q'),
            Some(vec![
                TerminalXtermColorStackOperation::Restore(1),
                TerminalXtermColorStackOperation::Restore(2)
            ])
        );
        assert_eq!(
            parse_terminal_xterm_color_stack(b"0#", b'P'),
            Some(vec![TerminalXtermColorStackOperation::Push])
        );
        assert_eq!(
            parse_terminal_xterm_color_stack(b"#", b'R'),
            Some(vec![TerminalXtermColorStackOperation::Report])
        );
        assert_eq!(parse_terminal_xterm_color_stack(b"11#", b'P'), None);
        assert_eq!(parse_terminal_xterm_color_stack(b"1#", b'R'), None);
        assert_eq!(parse_terminal_xterm_color_stack(b"1", b'P'), None);
    }

    #[test]
    fn parses_xterm_sgr_color_forms() {
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["5", "196"]),
            Some((ScreenColor::Indexed { index: 196 }, 2))
        );
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["5", "", "196"]),
            Some((ScreenColor::Indexed { index: 196 }, 3))
        );
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["2", "12", "34", "56"]),
            Some((ScreenColor::Rgb { r: 12, g: 34, b: 56 }, 4))
        );
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["2", "", "12", "34", "56"]),
            Some((ScreenColor::Rgb { r: 12, g: 34, b: 56 }, 5))
        );
        assert_eq!(
            parse_colon_sgr_color_part("38:5:196"),
            Some((AnsiSgrColorTarget::Foreground, ScreenColor::Indexed { index: 196 }))
        );
        assert_eq!(
            parse_colon_sgr_color_part("38:5::196"),
            Some((AnsiSgrColorTarget::Foreground, ScreenColor::Indexed { index: 196 }))
        );
        assert_eq!(
            parse_colon_sgr_color_part("38::5::196"),
            Some((AnsiSgrColorTarget::Foreground, ScreenColor::Indexed { index: 196 }))
        );
        assert_eq!(
            parse_colon_sgr_color_part("48:2::12:34:56"),
            Some((AnsiSgrColorTarget::Background, ScreenColor::Rgb { r: 12, g: 34, b: 56 },))
        );
        assert_eq!(
            parse_colon_sgr_color_part("48::2::12::34::56"),
            Some((AnsiSgrColorTarget::Background, ScreenColor::Rgb { r: 12, g: 34, b: 56 },))
        );
        assert_eq!(
            parse_colon_sgr_color_part("58:2:1:9:8:7"),
            Some((AnsiSgrColorTarget::Underline, ScreenColor::Rgb { r: 9, g: 8, b: 7 }))
        );
        assert_eq!(
            parse_colon_sgr_color_part("58::2::9::8::7"),
            Some((AnsiSgrColorTarget::Underline, ScreenColor::Rgb { r: 9, g: 8, b: 7 }))
        );
    }

    #[test]
    fn parses_extended_color_modes_with_explicit_degradation() {
        assert_eq!(
            parse_colon_sgr_color_part("38:6::12:34:56:128"),
            Some((AnsiSgrColorTarget::Foreground, ScreenColor::Rgb { r: 12, g: 34, b: 56 }))
        );
        assert_eq!(
            parse_colon_sgr_color_part("38::6::12::34::56::128"),
            Some((AnsiSgrColorTarget::Foreground, ScreenColor::Rgb { r: 12, g: 34, b: 56 }))
        );
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["6", "9", "8", "7", "128"]),
            Some((ScreenColor::Rgb { r: 9, g: 8, b: 7 }, 5))
        );
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["6", "", "9", "8", "7", "128"]),
            Some((ScreenColor::Rgb { r: 9, g: 8, b: 7 }, 6))
        );
        assert_eq!(
            parse_colon_sgr_color_part("48:3::0:128:255"),
            Some((AnsiSgrColorTarget::Background, ScreenColor::Rgb { r: 255, g: 127, b: 0 }))
        );
        assert_eq!(
            parse_colon_sgr_color_part("48::3::0::128::255"),
            Some((AnsiSgrColorTarget::Background, ScreenColor::Rgb { r: 255, g: 127, b: 0 }))
        );
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["3", "", "0", "128", "255"]),
            Some((ScreenColor::Rgb { r: 255, g: 127, b: 0 }, 5))
        );
        assert_eq!(
            parse_colon_sgr_color_part("58:4::0:128:255:64"),
            Some((AnsiSgrColorTarget::Underline, ScreenColor::Rgb { r: 191, g: 95, b: 0 }))
        );
        assert_eq!(
            parse_colon_sgr_color_part("58::4::0::128::255::64"),
            Some((AnsiSgrColorTarget::Underline, ScreenColor::Rgb { r: 191, g: 95, b: 0 }))
        );
        assert_eq!(
            parse_semicolon_sgr_color_fields(&["4", "", "0", "128", "255", "64"]),
            Some((ScreenColor::Rgb { r: 191, g: 95, b: 0 }, 6))
        );
    }
}
