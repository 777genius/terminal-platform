use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ScreenColor, ScreenCursor, ScreenCursorShape, ScreenLine, ScreenLineMedia, ScreenLineMediaKind,
    ScreenLineSemanticMark, ScreenLineSemanticMarkKind, ScreenLineSideEffect,
    ScreenLineSideEffectDisposition, ScreenLineSideEffectKind, ScreenLineSideEffectTarget,
    ScreenLineSpan, ScreenProgress, ScreenProgressState, ScreenSurface, ScreenSurfacePalette,
    ScreenTextBaseline, ScreenTextBorderStyle, ScreenTextStyle, ScreenUnderlineStyle,
    ansi_color::{
        TerminalKittyColorControlOperation, TerminalKittyColorStackOperation,
        TerminalPaletteTarget, TerminalXtermColorStackOperation,
        is_legacy_linux_console_palette_reset, next_terminal_default_palette_target,
        parse_colon_sgr_color_fields, parse_iterm2_set_colors_update,
        parse_legacy_linux_console_palette_update, parse_semicolon_sgr_color_fields,
        parse_terminal_color_spec, parse_terminal_kitty_color_control,
        parse_terminal_kitty_color_stack, parse_terminal_osc_p_palette_update,
        parse_terminal_xterm_color_stack, terminal_default_palette_target_from_osc_code,
    },
    ansi_rect::{
        TerminalRectangularArea, parse_terminal_rectangular_area,
        parse_terminal_rectangular_copy_request, parse_terminal_rectangular_fill_request,
    },
    ansi_sgr::{
        AnsiSgrStackAttributes, TerminalRectangularAttributeAction as RectangularAttributeAction,
        TerminalRectangularAttributeMode as RectangularAttributeMode,
        apply_terminal_rectangular_attribute_actions, parse_colon_sgr_underline_style,
        parse_terminal_rectangular_attribute_request, parse_xterm_sgr_stack_attributes,
    },
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use unicode_width::UnicodeWidthChar;

const MAX_CSI_SEQUENCE_BYTES: usize = 16 * 1024;
const MAX_CONTROL_STRING_BYTES: usize = 512 * 1024;
const MAX_INLINE_IMAGE_BYTES: usize = 384 * 1024;
const MAX_SGR_STACK_DEPTH: usize = 32;
const MAX_COLOR_STACK_DEPTH: usize = 32;
const DEFAULT_TAB_WIDTH: usize = 8;

pub fn screen_lines_from_ansi_output(output: &str) -> Vec<ScreenLine> {
    screen_surface_from_ansi_output(output, None).lines
}

pub fn screen_surface_from_ansi_output(output: &str, title: Option<String>) -> ScreenSurface {
    let mut parser = AnsiScreenLineParser::new();
    parser.push_str(output);
    parser.finish_surface(title)
}

pub fn screen_lines_from_ansi_bytes(output: &[u8]) -> Vec<ScreenLine> {
    screen_surface_from_ansi_bytes(output, None).lines
}

pub fn screen_surface_from_ansi_bytes(output: &[u8], title: Option<String>) -> ScreenSurface {
    let output = normalized_ansi_bytes_lossy(output);
    screen_surface_from_ansi_output(&output, title)
}

fn normalized_ansi_bytes_lossy(output: &[u8]) -> String {
    let mut normalized = Vec::with_capacity(output.len());
    for byte in output {
        if let Some(control) = raw_c1_control(*byte) {
            let mut encoded = [0; 4];
            normalized.extend_from_slice(control.encode_utf8(&mut encoded).as_bytes());
        } else {
            normalized.push(*byte);
        }
    }
    String::from_utf8_lossy(&normalized).into_owned()
}

#[derive(Default)]
struct AnsiScreenLineParser {
    lines: Vec<ScreenLine>,
    line_text: String,
    line_spans: Vec<ScreenLineSpan>,
    line_media: Vec<ScreenLineMedia>,
    line_side_effects: Vec<ScreenLineSideEffect>,
    line_semantic_marks: Vec<ScreenLineSemanticMark>,
    line_cursor_col: usize,
    current_row: usize,
    cursor_touched: bool,
    cursor_hidden: bool,
    cursor_shape: Option<ScreenCursorShape>,
    cursor_blinking: bool,
    current_style: ScreenTextStyle,
    sgr_stack: Vec<SgrStackEntry>,
    title: Option<String>,
    working_directory_uri: Option<String>,
    user_variables: BTreeMap<String, String>,
    palette: ScreenSurfacePalette,
    dynamic_palette: BTreeMap<u8, ScreenColor>,
    color_stack: Vec<ColorPaletteSnapshot>,
    bell_count: u64,
    progress: ScreenProgress,
    scroll_region: Option<ScrollRegion>,
    origin_mode: bool,
    left_right_margin_mode: bool,
    horizontal_margins: Option<HorizontalMargins>,
    protected_mode: bool,
    protected_cells: BTreeSet<(usize, usize)>,
    insert_mode: bool,
    default_tab_stops: bool,
    explicit_tab_stops: BTreeSet<usize>,
    cleared_default_tab_stops: BTreeSet<usize>,
    g0_charset: GraphicCharset,
    g1_charset: GraphicCharset,
    active_charset: ActiveCharset,
    saved_line_state: Option<SavedLineState>,
    normal_screen_state: Option<ScreenBufferState>,
    kitty_chunk: Option<TerminalKittyGraphicsChunk>,
    iterm2_multipart_file: Option<TerminalIterm2MultipartFile>,
    pending_carriage_return: bool,
    pending_backspace_cell: Option<BackspaceCell>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum GraphicCharset {
    #[default]
    Ascii,
    DecSpecialGraphics,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ActiveCharset {
    #[default]
    G0,
    G1,
}

#[derive(Clone)]
struct BackspaceCell {
    text: String,
    style: ScreenTextStyle,
}

#[derive(Clone)]
struct StyledCell {
    text: String,
    style: ScreenTextStyle,
    columns: usize,
    spacer: bool,
}

#[derive(Clone)]
struct SavedLineState {
    row: usize,
    cursor_col: usize,
    cursor_touched: bool,
    cursor_hidden: bool,
    cursor_shape: Option<ScreenCursorShape>,
    cursor_blinking: bool,
    style: ScreenTextStyle,
    protected_mode: bool,
    g0_charset: GraphicCharset,
    g1_charset: GraphicCharset,
    active_charset: ActiveCharset,
}

#[derive(Clone)]
struct SgrStackEntry {
    style: ScreenTextStyle,
    attributes: AnsiSgrStackAttributes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ScrollRegion {
    top: usize,
    bottom: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct HorizontalMargins {
    left: usize,
    right: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct HorizontalRegion {
    left: usize,
    right: usize,
}

#[derive(Clone)]
struct SavedScreenCursorState {
    row: usize,
    cursor_col: usize,
    cursor_touched: bool,
    cursor_hidden: bool,
    cursor_shape: Option<ScreenCursorShape>,
    cursor_blinking: bool,
}

#[derive(Clone, Copy)]
struct ResolvedRectangularArea {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

#[derive(Clone)]
struct RectangularCopyCell {
    cell: StyledCell,
    protected: bool,
}

#[derive(Clone, Copy)]
enum TerminalColumnEditMode {
    Insert,
    Delete,
    ScrollLeft,
    ScrollRight,
}

#[derive(Clone, Default)]
struct ColorPaletteSnapshot {
    palette: ScreenSurfacePalette,
    dynamic_palette: BTreeMap<u8, ScreenColor>,
}

#[derive(Clone)]
struct ScreenBufferState {
    lines: Vec<ScreenLine>,
    line_text: String,
    line_spans: Vec<ScreenLineSpan>,
    line_media: Vec<ScreenLineMedia>,
    line_side_effects: Vec<ScreenLineSideEffect>,
    line_semantic_marks: Vec<ScreenLineSemanticMark>,
    line_cursor_col: usize,
    current_row: usize,
    scroll_region: Option<ScrollRegion>,
    origin_mode: bool,
    left_right_margin_mode: bool,
    horizontal_margins: Option<HorizontalMargins>,
    protected_cells: BTreeSet<(usize, usize)>,
    cursor_touched: bool,
    cursor_hidden: bool,
    cursor_shape: Option<ScreenCursorShape>,
    cursor_blinking: bool,
}

impl AnsiScreenLineParser {
    fn new() -> Self {
        Self { default_tab_stops: true, ..Self::default() }
    }

    fn push_str(&mut self, output: &str) {
        let bytes = output.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if let Some((control, control_len)) = c1_control_at(bytes, index) {
                self.flush_pending_carriage_return_before_control();
                index = match control {
                    C1Control::Index => {
                        self.push_index();
                        index + control_len
                    }
                    C1Control::NextLine => {
                        self.push_line_break();
                        index + control_len
                    }
                    C1Control::ReverseIndex => {
                        self.apply_reverse_index();
                        index + control_len
                    }
                    C1Control::HorizontalTabSet => {
                        self.set_tab_stop_at_cursor();
                        index + control_len
                    }
                    C1Control::SingleShiftTwo | C1Control::SingleShiftThree => index + control_len,
                    C1Control::StartGuardedArea => {
                        self.protected_mode = true;
                        index + control_len
                    }
                    C1Control::EndGuardedArea => {
                        self.protected_mode = false;
                        index + control_len
                    }
                    C1Control::ReturnTerminalId => index + control_len,
                    C1Control::Sos | C1Control::Pm => {
                        skip_control_string(bytes, index + control_len)
                    }
                    C1Control::Csi => self.consume_csi(bytes, index + control_len),
                    C1Control::Osc => self.consume_osc(bytes, index + control_len),
                    C1Control::Dcs => self.consume_dcs(bytes, index + control_len),
                    C1Control::Apc => self.consume_apc(bytes, index + control_len),
                    C1Control::St => index + control_len,
                };
                continue;
            }
            match bytes[index] {
                0x1b => {
                    self.flush_pending_carriage_return_before_control();
                    index = self.consume_escape(bytes, index + 1);
                }
                b'\n' => {
                    self.push_line_break();
                    index += 1;
                }
                0x0b | 0x0c => {
                    self.push_line_break();
                    index += 1;
                }
                b'\r' => {
                    self.pending_backspace_cell = None;
                    self.pending_carriage_return = true;
                    index += 1;
                }
                0x07 => {
                    self.flush_pending_carriage_return_before_control();
                    self.bell_count = self.bell_count.saturating_add(1);
                    index += 1;
                }
                b'\t' => {
                    self.flush_pending_carriage_return_for_rewrite();
                    self.push_tab();
                    index += 1;
                }
                0x0e => {
                    self.flush_pending_carriage_return_before_control();
                    self.active_charset = ActiveCharset::G1;
                    index += 1;
                }
                0x0f => {
                    self.flush_pending_carriage_return_before_control();
                    self.active_charset = ActiveCharset::G0;
                    index += 1;
                }
                0x08 => {
                    self.flush_pending_carriage_return_for_rewrite();
                    self.pending_backspace_cell = self.pop_last_char();
                    index += 1;
                }
                byte if byte < 0x20 || byte == 0x7f => {
                    self.flush_pending_carriage_return_before_control();
                    index += 1;
                }
                _ => {
                    self.flush_pending_carriage_return_for_rewrite();
                    let Some(ch) = output[index..].chars().next() else {
                        break;
                    };
                    let char_len = ch.len_utf8();
                    let display_ch =
                        self.map_active_charset_byte(bytes[index], char_len).unwrap_or(ch);
                    self.push_text(display_ch);
                    index += char_len;
                }
            }
        }
    }

    fn finish_surface(mut self, fallback_title: Option<String>) -> ScreenSurface {
        if self.pending_carriage_return {
            self.pending_carriage_return = false;
        }
        if !self.line_text.is_empty()
            || !self.line_spans.is_empty()
            || !self.line_media.is_empty()
            || !self.line_side_effects.is_empty()
            || !self.line_semantic_marks.is_empty()
            || self.current_row < self.lines.len()
        {
            self.store_current_line();
        }
        let cursor = self.cursor_touched.then(|| ScreenCursor {
            row: self.current_row.min(usize::from(u16::MAX)) as u16,
            col: self.line_cursor_col.min(usize::from(u16::MAX)) as u16,
            shape: if self.cursor_hidden {
                Some(ScreenCursorShape::Hidden)
            } else {
                self.cursor_shape
            },
            blinking: !self.cursor_hidden && self.cursor_blinking,
        });
        ScreenSurface {
            title: self.title.or(fallback_title),
            working_directory_uri: self.working_directory_uri,
            user_variables: self.user_variables,
            cursor,
            palette: self.palette,
            bell_count: self.bell_count,
            progress: self.progress,
            lines: self.lines,
        }
    }

    fn push_line_break(&mut self) {
        self.pending_backspace_cell = None;
        if let Some(region) = self.active_scroll_region()
            && self.current_row == region.bottom
        {
            self.scroll_region_up(region, 1);
            self.load_row(region.bottom);
            self.line_cursor_col = 0;
            return;
        }
        self.store_current_line();
        self.current_row = self.current_row.saturating_add(1);
        self.load_current_line();
        self.line_cursor_col = 0;
    }

    fn push_index(&mut self) {
        self.pending_backspace_cell = None;
        if let Some(region) = self.active_scroll_region()
            && self.current_row == region.bottom
        {
            self.scroll_region_up(region, 1);
            self.load_row(region.bottom);
            return;
        }
        self.store_current_line();
        self.current_row = self.current_row.saturating_add(1);
        self.load_current_line();
    }

    fn store_current_line(&mut self) {
        if !self.current_line_is_visible() && self.current_row >= self.lines.len() {
            return;
        }
        let line = self.current_screen_line();
        if self.current_row > self.lines.len() {
            self.lines.resize_with(self.current_row, || ScreenLine::plain(""));
        }
        if self.current_row == self.lines.len() {
            self.lines.push(line);
        } else {
            self.lines[self.current_row] = line;
        }
    }

    fn load_row(&mut self, row: usize) {
        if row == self.current_row {
            return;
        }
        self.store_current_line();
        self.current_row = row;
        self.load_current_line();
    }

    fn load_current_line(&mut self) {
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        if let Some(line) = self.lines.get(self.current_row).cloned() {
            self.line_text = line.text;
            self.line_spans = line.spans;
            self.line_media = line.media;
            self.line_side_effects = line.side_effects;
            self.line_semantic_marks = line.semantic_marks;
            return;
        }
        self.clear_current_line_contents_preserving_cursor();
    }

    fn active_scroll_region(&self) -> Option<ScrollRegion> {
        let region = self.scroll_region?;
        (region.top < region.bottom).then_some(region)
    }

    fn origin_home_row(&self) -> usize {
        self.origin_mode
            .then(|| self.active_scroll_region().map(|region| region.top))
            .flatten()
            .unwrap_or(0)
    }

    fn absolute_row_for_position(&self, row: usize) -> usize {
        let Some(region) = self.active_scroll_region().filter(|_| self.origin_mode) else {
            return row;
        };
        region.top.saturating_add(row).min(region.bottom)
    }

    fn move_vertical(&mut self, row: usize) {
        self.load_row(self.absolute_row_for_position(row));
    }

    fn move_relative_vertical(&mut self, row: usize) {
        let Some(region) = self.active_scroll_region().filter(|_| self.origin_mode) else {
            self.load_row(row);
            return;
        };
        self.load_row(row.clamp(region.top, region.bottom));
    }

    fn operation_scroll_region(&self) -> Option<ScrollRegion> {
        self.active_scroll_region().or_else(|| {
            let bottom = self.lines.len().max(self.current_row.saturating_add(1)).checked_sub(1)?;
            Some(ScrollRegion { top: 0, bottom })
        })
    }

    fn active_horizontal_region(&self, width: usize) -> Option<HorizontalRegion> {
        if width == 0 {
            return None;
        }
        let full = HorizontalRegion { left: 0, right: width - 1 };
        if !self.left_right_margin_mode {
            return Some(full);
        }
        let Some(margins) = self.horizontal_margins else {
            return Some(full);
        };
        let left = margins.left.min(width - 1);
        let right = margins.right.min(width - 1);
        (left < right).then_some(HorizontalRegion { left, right }).or(Some(full))
    }

    fn ensure_region_rows(&mut self, region: ScrollRegion) {
        if self.lines.len() <= region.bottom {
            self.lines.resize_with(region.bottom + 1, || ScreenLine::plain(""));
        }
    }

    fn scroll_region_up(&mut self, region: ScrollRegion, count: usize) {
        self.store_current_line();
        self.ensure_region_rows(region);
        let height = region.bottom.saturating_sub(region.top).saturating_add(1);
        let count = count.min(height);
        if count == 0 {
            return;
        }
        self.scroll_protection_up(region, count);
        self.lines.drain(region.top..region.top + count);
        let insert_at = region.bottom.saturating_add(1).saturating_sub(count);
        self.lines.splice(
            insert_at..insert_at,
            std::iter::repeat_with(|| ScreenLine::plain("")).take(count),
        );
        self.load_current_line();
    }

    fn scroll_region_down(&mut self, region: ScrollRegion, count: usize) {
        self.store_current_line();
        self.ensure_region_rows(region);
        let height = region.bottom.saturating_sub(region.top).saturating_add(1);
        let count = count.min(height);
        if count == 0 {
            return;
        }
        self.scroll_protection_down(region, count);
        self.lines.splice(
            region.top..region.top,
            std::iter::repeat_with(|| ScreenLine::plain("")).take(count),
        );
        let drain_from = region.bottom.saturating_add(1);
        let drain_to = drain_from.saturating_add(count).min(self.lines.len());
        self.lines.drain(drain_from..drain_to);
        self.load_current_line();
    }

    fn protect_current_row_range(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        for col in start..end {
            self.protected_cells.insert((self.current_row, col));
        }
    }

    fn clear_current_row_protection_range(&mut self, start: usize, end: usize) {
        self.clear_row_protection_range(self.current_row, start, end);
    }

    fn clear_row_protection_range(&mut self, row: usize, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.protected_cells
            .retain(|(protected_row, col)| *protected_row != row || *col < start || *col >= end);
    }

    fn clear_current_row_protection_from(&mut self, start: usize) {
        let row = self.current_row;
        self.protected_cells.retain(|(protected_row, col)| *protected_row != row || *col < start);
    }

    fn clear_row_protection(&mut self, row: usize) {
        self.protected_cells.retain(|(protected_row, _)| *protected_row != row);
    }

    fn clear_rows_protection(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.protected_cells.retain(|(row, _)| *row < start || *row >= end);
    }

    fn clear_current_row_wide_cluster_protection_at(&mut self, cells: &[StyledCell], col: usize) {
        if col >= cells.len() {
            return;
        }
        let start = wide_cluster_start_at(cells, col).unwrap_or(col);
        let end = start.saturating_add(cells[start].columns.max(1)).min(cells.len());
        self.clear_current_row_protection_range(start, end);
    }

    fn write_current_row_protection(&mut self, start: usize, width: usize) {
        let end = start.saturating_add(width);
        if self.protected_mode {
            self.protect_current_row_range(start, end);
        } else {
            self.clear_current_row_protection_range(start, end);
        }
    }

    fn shift_current_row_protection_right(&mut self, start: usize, count: usize) {
        if count == 0 {
            return;
        }
        let row = self.current_row;
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(protected_row, col)| {
            if *protected_row == row && *col >= start {
                shifted.push((row, col.saturating_add(count)));
                false
            } else {
                true
            }
        });
        self.protected_cells.extend(shifted);
    }

    fn delete_current_row_protection_range(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let row = self.current_row;
        let count = end - start;
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(protected_row, col)| {
            if *protected_row != row || *col < start {
                return true;
            }
            if *col < end {
                return false;
            }
            shifted.push((row, col.saturating_sub(count)));
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn shift_row_protection_right_within(
        &mut self,
        row: usize,
        start: usize,
        count: usize,
        width: usize,
    ) {
        if count == 0 || start >= width {
            return;
        }
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(protected_row, col)| {
            if *protected_row != row || *col < start {
                return true;
            }
            let next_col = col.saturating_add(count);
            if next_col < width {
                shifted.push((row, next_col));
            }
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn delete_row_protection_range_within(
        &mut self,
        row: usize,
        start: usize,
        end: usize,
        width: usize,
    ) {
        if start >= end {
            return;
        }
        let count = end - start;
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(protected_row, col)| {
            if *protected_row != row || *col < start {
                return true;
            }
            if *col < end {
                return false;
            }
            let next_col = col.saturating_sub(count);
            if next_col < width {
                shifted.push((row, next_col));
            }
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn row_has_protection_in_range(&self, row: usize, start: usize, end: usize) -> bool {
        start < end && self.protected_cells.range((row, start)..(row, end)).next().is_some()
    }

    fn current_row_has_protection_in_range(&self, start: usize, end: usize) -> bool {
        self.row_has_protection_in_range(self.current_row, start, end)
    }

    fn scroll_protection_up(&mut self, region: ScrollRegion, count: usize) {
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(row, col)| {
            if *row < region.top || *row > region.bottom {
                return true;
            }
            if *row < region.top.saturating_add(count) {
                return false;
            }
            shifted.push((row.saturating_sub(count), *col));
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn scroll_protection_down(&mut self, region: ScrollRegion, count: usize) {
        let cutoff = region.bottom.saturating_add(1).saturating_sub(count);
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(row, col)| {
            if *row < region.top || *row > region.bottom {
                return true;
            }
            if *row >= cutoff {
                return false;
            }
            shifted.push((row.saturating_add(count), *col));
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn insert_protection_rows_within(&mut self, start: usize, bottom: usize, count: usize) {
        if count == 0 || start > bottom {
            return;
        }
        let drop_from = bottom.saturating_add(1).saturating_sub(count);
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(row, col)| {
            if *row < start || *row > bottom {
                return true;
            }
            if *row >= drop_from {
                return false;
            }
            shifted.push((row.saturating_add(count), *col));
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn delete_protection_rows_within(&mut self, start: usize, bottom: usize, count: usize) {
        if count == 0 || start > bottom {
            return;
        }
        let delete_to = start.saturating_add(count).min(bottom.saturating_add(1));
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(row, col)| {
            if *row < start || *row > bottom {
                return true;
            }
            if *row < delete_to {
                return false;
            }
            shifted.push((row.saturating_sub(count), *col));
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn insert_protection_rows_unbounded(&mut self, start: usize, count: usize) {
        if count == 0 {
            return;
        }
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(row, col)| {
            if *row < start {
                return true;
            }
            shifted.push((row.saturating_add(count), *col));
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn delete_protection_rows_unbounded(&mut self, start: usize, count: usize) {
        if count == 0 {
            return;
        }
        let delete_to = start.saturating_add(count);
        let mut shifted = Vec::new();
        self.protected_cells.retain(|(row, col)| {
            if *row < start {
                return true;
            }
            if *row < delete_to {
                return false;
            }
            shifted.push((row.saturating_sub(count), *col));
            false
        });
        self.protected_cells.extend(shifted);
    }

    fn current_screen_line(&self) -> ScreenLine {
        let text = self.line_text.clone();
        let spans = normalized_spans(text.as_str(), self.line_spans.clone());
        ScreenLine {
            text,
            spans,
            media: self.line_media.clone(),
            side_effects: self.line_side_effects.clone(),
            semantic_marks: self.line_semantic_marks.clone(),
            wrapped: false,
        }
    }

    fn current_line_is_visible(&self) -> bool {
        !self.line_text.is_empty()
            || !self.line_spans.is_empty()
            || !self.line_media.is_empty()
            || !self.line_side_effects.is_empty()
            || !self.line_semantic_marks.is_empty()
    }

    fn current_buffer_state(&self) -> ScreenBufferState {
        ScreenBufferState {
            lines: self.lines.clone(),
            line_text: self.line_text.clone(),
            line_spans: self.line_spans.clone(),
            line_media: self.line_media.clone(),
            line_side_effects: self.line_side_effects.clone(),
            line_semantic_marks: self.line_semantic_marks.clone(),
            line_cursor_col: self.line_cursor_col,
            current_row: self.current_row,
            scroll_region: self.scroll_region,
            origin_mode: self.origin_mode,
            left_right_margin_mode: self.left_right_margin_mode,
            horizontal_margins: self.horizontal_margins,
            protected_cells: self.protected_cells.clone(),
            cursor_touched: self.cursor_touched,
            cursor_hidden: self.cursor_hidden,
            cursor_shape: self.cursor_shape,
            cursor_blinking: self.cursor_blinking,
        }
    }

    fn restore_buffer_state(&mut self, state: ScreenBufferState) {
        self.lines = state.lines;
        self.line_text = state.line_text;
        self.line_spans = state.line_spans;
        self.line_media = state.line_media;
        self.line_side_effects = state.line_side_effects;
        self.line_semantic_marks = state.line_semantic_marks;
        self.line_cursor_col = state.line_cursor_col;
        self.current_row = state.current_row;
        self.scroll_region = state.scroll_region;
        self.origin_mode = state.origin_mode;
        self.left_right_margin_mode = state.left_right_margin_mode;
        self.horizontal_margins = state.horizontal_margins;
        self.protected_cells = state.protected_cells;
        self.cursor_touched = state.cursor_touched;
        self.cursor_hidden = state.cursor_hidden;
        self.cursor_shape = state.cursor_shape;
        self.cursor_blinking = state.cursor_blinking;
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
    }

    fn enter_alternate_screen(&mut self) {
        if self.normal_screen_state.is_none() {
            self.store_current_line();
            self.normal_screen_state = Some(self.current_buffer_state());
        }
        self.lines.clear();
        self.current_row = 0;
        self.scroll_region = None;
        self.origin_mode = false;
        self.protected_cells.clear();
        self.clear_current_line();
        self.cursor_touched = true;
    }

    fn leave_alternate_screen(&mut self) {
        let Some(state) = self.normal_screen_state.take() else {
            return;
        };
        self.restore_buffer_state(state);
        self.cursor_touched = true;
    }

    fn push_text(&mut self, ch: char) {
        if let Some((overstrike_ch, overstrike_style)) = self.take_overstrike_cell(ch) {
            self.push_text_with_style(overstrike_ch, overstrike_style);
            return;
        }
        self.push_text_with_style(ch, self.current_style.clone());
    }

    fn erase_blank_cell(&self) -> StyledCell {
        styled_space_cell(ScreenTextStyle {
            background: self.current_style.background.clone(),
            ..ScreenTextStyle::default()
        })
    }

    fn push_text_with_style(&mut self, ch: char, style: ScreenTextStyle) {
        let width = terminal_char_width(ch);
        let mut cells = self.current_line_cells();

        if width == 0 {
            if append_zero_width_to_last_visible_cell(&mut cells, ch) {
                self.rebuild_line_from_cells(cells);
            } else {
                let mut buffer = [0; 4];
                self.append_line_text(ch.encode_utf8(&mut buffer), style);
            }
            return;
        }

        while self.line_cursor_col > cells.len() {
            cells.push(styled_space_cell(ScreenTextStyle::default()));
        }

        let write_start = self.line_cursor_col;
        if self.insert_mode {
            self.clear_current_row_wide_cluster_protection_at(&cells, write_start);
            self.shift_current_row_protection_right(write_start, width);
            clear_wide_cluster_at(&mut cells, write_start);
            cells.splice(
                write_start.min(cells.len())..write_start.min(cells.len()),
                std::iter::repeat_with(|| styled_space_cell(ScreenTextStyle::default()))
                    .take(width),
            );
        }
        for col in write_start..write_start.saturating_add(width) {
            self.clear_current_row_wide_cluster_protection_at(&cells, col);
            clear_wide_cluster_at(&mut cells, col);
        }
        self.write_current_row_protection(write_start, width);

        if write_start < cells.len() {
            cells[write_start] = styled_text_cell(ch, style.clone(), width);
        } else {
            cells.push(styled_text_cell(ch, style.clone(), width));
        }
        for col in write_start + 1..write_start.saturating_add(width) {
            if col < cells.len() {
                cells[col] = styled_spacer_cell(style.clone());
            } else {
                cells.push(styled_spacer_cell(style.clone()));
            }
        }

        self.line_cursor_col = self.line_cursor_col.saturating_add(width);
        self.rebuild_line_from_cells(cells);
    }

    fn append_line_text(&mut self, text: &str, style: ScreenTextStyle) {
        self.line_text.push_str(text);
        if let Some(last) = self.line_spans.last_mut()
            && last.style == style
        {
            last.text.push_str(text);
            return;
        }
        self.line_spans.push(ScreenLineSpan { text: text.to_string(), style });
    }

    fn current_line_cells(&self) -> Vec<StyledCell> {
        if !self.line_spans.is_empty()
            && self.line_spans.iter().map(|span| span.text.as_str()).collect::<String>()
                == self.line_text
        {
            let mut cells = Vec::new();
            for span in &self.line_spans {
                push_text_cells(&mut cells, span.text.as_str(), span.style.clone());
            }
            return cells;
        }

        let mut cells = Vec::new();
        push_text_cells(&mut cells, self.line_text.as_str(), ScreenTextStyle::default());
        cells
    }

    fn projected_row_count(&self) -> usize {
        self.lines.len().max(self.current_row.saturating_add(1)).max(1)
    }

    fn max_projected_line_width(&self, top: usize, bottom: usize) -> Option<usize> {
        let mut width = 0usize;
        for row in top..=bottom {
            let row_width = if row == self.current_row {
                self.current_line_cells().len()
            } else {
                self.lines.get(row).map(screen_line_cell_width).unwrap_or(0)
            };
            width = width.max(row_width);
        }
        (width > 0).then_some(width)
    }

    fn rebuild_line_from_cells(&mut self, cells: Vec<StyledCell>) {
        self.line_text.clear();
        self.line_spans.clear();
        for cell in cells {
            if !cell.spacer {
                self.append_line_text(cell.text.as_str(), cell.style);
            }
        }
    }

    fn push_tab(&mut self) {
        let spaces = next_terminal_tab_stop(
            self.line_cursor_col,
            self.default_tab_stops,
            &self.explicit_tab_stops,
            &self.cleared_default_tab_stops,
        )
        .saturating_sub(self.line_cursor_col);
        for _ in 0..spaces {
            self.push_text_with_style(' ', self.current_style.clone());
        }
    }

    fn pop_last_char(&mut self) -> Option<BackspaceCell> {
        let mut cells = self.current_line_cells();
        if cells.is_empty() || self.line_cursor_col == 0 {
            return None;
        }
        let target_col = self.line_cursor_col.min(cells.len()) - 1;
        let remove_start = wide_cluster_start_at(&cells, target_col).unwrap_or(target_col);
        let previous = cells[remove_start].clone();
        let remove_to = remove_start.saturating_add(previous.columns.max(1)).min(cells.len());
        self.delete_current_row_protection_range(remove_start, remove_to);
        cells.drain(remove_start..remove_to);
        self.line_cursor_col = remove_start;
        self.rebuild_line_from_cells(cells);
        Some(BackspaceCell { text: previous.text, style: previous.style })
    }

    fn flush_pending_carriage_return_for_rewrite(&mut self) {
        if !self.pending_carriage_return {
            return;
        }
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.clear_row_protection(self.current_row);
        self.line_text.clear();
        self.line_spans.clear();
        self.line_cursor_col = 0;
    }

    fn flush_pending_carriage_return_before_control(&mut self) {
        // A control sequence does not move the cursor by itself. Keep a pending
        // carriage return alive through SGR/OSC/DCS/APC so `\r\x1b[31mtext`
        // rewrites the current line just like a terminal would.
    }

    fn consume_escape(&mut self, bytes: &[u8], mut index: usize) -> usize {
        if index >= bytes.len() {
            return index;
        }
        match bytes[index] {
            b'[' => self.consume_csi(bytes, index + 1),
            b']' => self.consume_osc(bytes, index + 1),
            b'P' => self.consume_dcs(bytes, index + 1),
            b'_' => self.consume_apc(bytes, index + 1),
            b'^' | b'X' => skip_control_string(bytes, index + 1),
            b'(' => self.consume_charset_designation(bytes, index + 1, ActiveCharset::G0),
            b')' => self.consume_charset_designation(bytes, index + 1, ActiveCharset::G1),
            b'*' | b'+' | b'-' | b'.' => Self::consume_one_escape_final(bytes, index + 1),
            b'#' => Self::consume_one_escape_final(bytes, index + 1),
            b'%' => Self::consume_utf8_charset_designation(bytes, index + 1),
            b'$' => Self::consume_multibyte_charset_designation(bytes, index + 1),
            b' ' => Self::consume_one_escape_final(bytes, index + 1),
            b'D' => {
                self.push_index();
                index + 1
            }
            b'E' => {
                self.push_line_break();
                index + 1
            }
            b'H' => {
                self.set_tab_stop_at_cursor();
                index + 1
            }
            b'6' => {
                self.apply_back_index();
                index + 1
            }
            b'7' => {
                self.save_line_state();
                index + 1
            }
            b'8' => {
                self.restore_line_state();
                index + 1
            }
            b'9' => {
                self.apply_forward_index();
                index + 1
            }
            b'M' => {
                self.apply_reverse_index();
                index + 1
            }
            b'N' | b'O' => index + 1,
            b'V' => {
                self.protected_mode = true;
                index + 1
            }
            b'W' => {
                self.protected_mode = false;
                index + 1
            }
            b'Z' => index + 1,
            b'c' => {
                self.reset_terminal_state();
                index + 1
            }
            0x1b => index,
            _ => {
                index += 1;
                index
            }
        }
    }

    fn consume_charset_designation(
        &mut self,
        bytes: &[u8],
        index: usize,
        target: ActiveCharset,
    ) -> usize {
        let Some(byte) = bytes.get(index).copied() else {
            return index;
        };
        let charset = match byte {
            b'0' => Some(GraphicCharset::DecSpecialGraphics),
            b'@' | b'B' => Some(GraphicCharset::Ascii),
            _ => None,
        };
        if let Some(charset) = charset {
            match target {
                ActiveCharset::G0 => {
                    self.g0_charset = charset;
                    self.active_charset = ActiveCharset::G0;
                }
                ActiveCharset::G1 => {
                    self.g1_charset = charset;
                }
            }
        }
        index + 1
    }

    fn consume_one_escape_final(bytes: &[u8], index: usize) -> usize {
        if bytes.get(index).is_some() { index + 1 } else { index }
    }

    fn consume_utf8_charset_designation(bytes: &[u8], index: usize) -> usize {
        match bytes.get(index).copied() {
            Some(b'@' | b'G') => index + 1,
            Some(b'/') => Self::consume_one_escape_final(bytes, index + 1),
            Some(_) => index + 1,
            None => index,
        }
    }

    fn consume_multibyte_charset_designation(bytes: &[u8], index: usize) -> usize {
        match bytes.get(index).copied() {
            Some(b'(' | b')' | b'*' | b'+' | b'-' | b'.' | b'/') => {
                Self::consume_one_escape_final(bytes, index + 1)
            }
            Some(_) => index + 1,
            None => index,
        }
    }

    fn consume_csi(&mut self, bytes: &[u8], index: usize) -> usize {
        let Some((payload, final_byte, next_index)) = read_csi(bytes, index) else {
            return bytes.len();
        };
        match final_byte {
            b'm' => apply_sgr_payload(&mut self.current_style, payload, &self.dynamic_palette),
            b'@' if csi_has_space_intermediate(payload) => self.apply_scroll_left_columns(payload),
            b'@' => self.apply_insert_characters(payload),
            b'K' => self.apply_erase_in_line(payload),
            b'J' => self.apply_erase_in_display(payload),
            b'I' => self.apply_cursor_forward_tabulation(payload),
            b'G' | b'`' => self.apply_horizontal_absolute(payload),
            b'H' | b'f' => self.apply_cursor_position(payload),
            b'g' => self.apply_tab_clear(payload),
            b'C' => self.apply_cursor_forward(payload),
            b'D' => self.apply_cursor_backward(payload),
            b'A' if csi_has_space_intermediate(payload) => self.apply_scroll_right_columns(payload),
            b'A' => self.apply_cursor_up(payload),
            b'B' | b'e' => self.apply_cursor_down(payload),
            b'E' => self.apply_cursor_next_line(payload),
            b'F' => self.apply_cursor_previous_line(payload),
            b'L' => self.apply_insert_lines(payload),
            b'M' => self.apply_delete_lines(payload),
            b'S' => self.apply_scroll_up(payload),
            b'T' => self.apply_scroll_down(payload),
            b'^' => self.apply_scroll_down(payload),
            b'd' => self.apply_vertical_absolute(payload),
            b'a' => self.apply_cursor_forward(payload),
            b'b' => self.apply_repeat_preceding_character(payload),
            b'r' if csi_has_dollar_intermediate(payload) => {
                self.apply_rectangular_attributes(payload, RectangularAttributeMode::Change)
            }
            b't' if csi_has_dollar_intermediate(payload) => {
                self.apply_rectangular_attributes(payload, RectangularAttributeMode::Reverse)
            }
            b'x' if csi_has_dollar_intermediate(payload) => {
                self.apply_fill_rectangular_area(payload)
            }
            b'z' if csi_has_dollar_intermediate(payload) => {
                self.apply_erase_rectangular_area(payload, false)
            }
            b'{' if csi_has_dollar_intermediate(payload) => {
                self.apply_erase_rectangular_area(payload, true)
            }
            b'v' if csi_has_dollar_intermediate(payload) => {
                self.apply_copy_rectangular_area(payload)
            }
            b'}' if csi_has_apostrophe_intermediate(payload) => self.apply_insert_columns(payload),
            b'~' if csi_has_apostrophe_intermediate(payload) => self.apply_delete_columns(payload),
            b'r' => self.apply_scroll_region(payload),
            b'h' => self.apply_mode_set(payload),
            b'l' => self.apply_mode_reset(payload),
            b'{' if parse_xterm_sgr_stack_attributes(payload).is_some() => {
                let Some(attributes) = parse_xterm_sgr_stack_attributes(payload) else {
                    unreachable!("guarded by parse_xterm_sgr_stack_attributes");
                };
                self.push_sgr_state(attributes);
            }
            b'P' if parse_terminal_xterm_color_stack(payload, final_byte).is_some() => {
                let Some(operations) = parse_terminal_xterm_color_stack(payload, final_byte) else {
                    unreachable!("guarded by parse_terminal_xterm_color_stack");
                };
                self.apply_color_stack_operations(operations);
            }
            b'Q' if parse_terminal_xterm_color_stack(payload, final_byte).is_some() => {
                let Some(operations) = parse_terminal_xterm_color_stack(payload, final_byte) else {
                    unreachable!("guarded by parse_terminal_xterm_color_stack");
                };
                self.apply_color_stack_operations(operations);
            }
            b'p' if parse_xterm_sgr_stack_attributes(payload).is_some() => {
                let Some(attributes) = parse_xterm_sgr_stack_attributes(payload) else {
                    unreachable!("guarded by parse_xterm_sgr_stack_attributes");
                };
                self.push_sgr_state(attributes);
            }
            b'}' if payload == b"#" => self.pop_sgr_state(),
            b'q' if payload == b"#" => self.pop_sgr_state(),
            b'q' if csi_has_quote_intermediate(payload) => self.apply_character_protection(payload),
            b'q' if csi_has_space_intermediate(payload) => self.apply_cursor_shape(payload),
            b'p' if csi_has_bang_intermediate(payload) => self.apply_soft_terminal_reset(),
            b'P' => self.apply_delete_characters(payload),
            b'X' => self.apply_erase_characters(payload),
            b'Z' => self.apply_cursor_backward_tabulation(payload),
            b's' if self.left_right_margin_mode => self.apply_horizontal_margins(payload),
            b's' => self.save_line_state(),
            b'u' => self.restore_line_state(),
            _ => {}
        }
        next_index
    }

    fn save_line_state(&mut self) {
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.saved_line_state = Some(SavedLineState {
            row: self.current_row,
            cursor_col: self.line_cursor_col,
            cursor_touched: self.cursor_touched,
            cursor_hidden: self.cursor_hidden,
            cursor_shape: self.cursor_shape,
            cursor_blinking: self.cursor_blinking,
            style: self.current_style.clone(),
            protected_mode: self.protected_mode,
            g0_charset: self.g0_charset,
            g1_charset: self.g1_charset,
            active_charset: self.active_charset,
        });
    }

    fn restore_line_state(&mut self) {
        let Some(saved) = self.saved_line_state.clone() else {
            return;
        };
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.load_row(saved.row);
        self.line_cursor_col = saved.cursor_col;
        self.cursor_touched = saved.cursor_touched;
        self.cursor_hidden = saved.cursor_hidden;
        self.cursor_shape = saved.cursor_shape;
        self.cursor_blinking = saved.cursor_blinking;
        self.current_style = saved.style;
        self.protected_mode = saved.protected_mode;
        self.g0_charset = saved.g0_charset;
        self.g1_charset = saved.g1_charset;
        self.active_charset = saved.active_charset;
    }

    fn push_sgr_state(&mut self, attributes: AnsiSgrStackAttributes) {
        if self.sgr_stack.len() >= MAX_SGR_STACK_DEPTH {
            self.sgr_stack.remove(0);
        }

        let mut sgr_style = self.current_style.clone();
        sgr_style.hyperlink = None;
        self.sgr_stack.push(SgrStackEntry { style: sgr_style, attributes });
    }

    fn pop_sgr_state(&mut self) {
        let Some(entry) = self.sgr_stack.pop() else {
            return;
        };

        let hyperlink = self.current_style.hyperlink.clone();
        restore_sgr_stack_attributes(&mut self.current_style, &entry.style, entry.attributes);
        self.current_style.hyperlink = hyperlink;
    }

    fn apply_erase_in_line(&mut self, payload: &[u8]) {
        let (selective, payload) = selective_csi_payload(payload);
        let mode = first_csi_numeric_parameter(payload).unwrap_or(0);
        match mode {
            0 if self.pending_carriage_return && !selective => self.clear_current_line(),
            0 => {
                if self.pending_carriage_return {
                    self.pending_carriage_return = false;
                    self.pending_backspace_cell = None;
                    self.line_cursor_col = 0;
                }
                let mut cells = self.current_line_cells();
                let truncate_at = self.line_cursor_col.min(cells.len());
                let cell_len = cells.len();
                let blank_cell = self.erase_blank_cell();
                if selective && self.current_row_has_protection_in_range(truncate_at, cell_len) {
                    erase_unprotected_cell_range(
                        &mut cells,
                        truncate_at,
                        cell_len,
                        self.current_row,
                        &self.protected_cells,
                        &blank_cell,
                    );
                    self.rebuild_line_from_cells(cells);
                } else {
                    clear_wide_cluster_at(&mut cells, truncate_at);
                    self.clear_current_row_protection_from(truncate_at);
                    cells.truncate(truncate_at);
                    self.rebuild_line_from_cells(cells);
                }
            }
            1 => {
                let mut cells = self.current_line_cells();
                let end = self.line_cursor_col.min(cells.len().saturating_sub(1));
                let blank_cell = self.erase_blank_cell();
                if selective {
                    erase_unprotected_cell_range(
                        &mut cells,
                        0,
                        end + 1,
                        self.current_row,
                        &self.protected_cells,
                        &blank_cell,
                    );
                } else {
                    erase_cell_range(&mut cells, 0, end + 1, &blank_cell);
                    self.clear_current_row_protection_range(0, end + 1);
                }
                self.rebuild_line_from_cells(cells);
            }
            2 if selective => {
                let mut cells = self.current_line_cells();
                let cell_len = cells.len();
                let blank_cell = self.erase_blank_cell();
                if self.current_row_has_protection_in_range(0, cell_len) {
                    erase_unprotected_cell_range(
                        &mut cells,
                        0,
                        cell_len,
                        self.current_row,
                        &self.protected_cells,
                        &blank_cell,
                    );
                    self.rebuild_line_from_cells(cells);
                } else {
                    self.clear_current_line_contents_preserving_cursor();
                }
            }
            2 => self.clear_current_line(),
            _ => {}
        }
    }

    fn apply_erase_in_display(&mut self, payload: &[u8]) {
        let (selective, payload) = selective_csi_payload(payload);
        let mode = first_csi_numeric_parameter(payload).unwrap_or(0);
        if selective {
            self.apply_selective_erase_in_display(mode);
            return;
        }
        if mode == 2 || mode == 3 {
            self.lines.clear();
            self.current_row = 0;
            self.protected_cells.clear();
            self.clear_current_line();
        } else if mode == 0 && self.pending_carriage_return {
            self.clear_current_line();
        } else if mode == 0 {
            self.apply_erase_in_line(payload);
            self.clear_rows_protection(self.current_row + 1, usize::MAX);
            self.lines.truncate(self.current_row);
        } else if mode == 1 {
            let rows_to_clear = self.current_row.min(self.lines.len());
            self.clear_rows_protection(0, rows_to_clear);
            for row in &mut self.lines[..rows_to_clear] {
                *row = ScreenLine::plain("");
            }
            self.apply_erase_in_line(payload);
        }
    }

    fn apply_selective_erase_in_display(&mut self, mode: u16) {
        let original_row = self.current_row;
        let original_cursor_col = self.line_cursor_col;
        let original_cursor_touched = self.cursor_touched;
        let original_cursor_hidden = self.cursor_hidden;
        let original_cursor_shape = self.cursor_shape;
        let original_cursor_blinking = self.cursor_blinking;
        self.store_current_line();

        let last_row = self.lines.len().max(original_row.saturating_add(1)).saturating_sub(1);
        match mode {
            0 => {
                self.load_row(original_row);
                self.apply_selective_erase_current_line_range(original_cursor_col, usize::MAX);
                for row in original_row.saturating_add(1)..=last_row {
                    self.load_row(row);
                    self.apply_selective_erase_current_line_range(0, usize::MAX);
                }
            }
            1 => {
                for row in 0..original_row {
                    self.load_row(row);
                    self.apply_selective_erase_current_line_range(0, usize::MAX);
                }
                self.load_row(original_row);
                self.apply_selective_erase_current_line_range(
                    0,
                    original_cursor_col.saturating_add(1),
                );
            }
            2 | 3 => {
                for row in 0..=last_row {
                    self.load_row(row);
                    self.apply_selective_erase_current_line_range(0, usize::MAX);
                }
            }
            _ => {}
        }

        self.load_row(original_row);
        self.line_cursor_col = original_cursor_col;
        self.cursor_touched = original_cursor_touched;
        self.cursor_hidden = original_cursor_hidden;
        self.cursor_shape = original_cursor_shape;
        self.cursor_blinking = original_cursor_blinking;
    }

    fn apply_selective_erase_current_line_range(&mut self, start: usize, end: usize) {
        let mut cells = self.current_line_cells();
        let start = start.min(cells.len());
        let end = end.min(cells.len());
        if !self.current_row_has_protection_in_range(start, end) {
            if start == 0 && end == cells.len() {
                self.clear_current_line_contents_preserving_cursor();
            } else if start < end {
                let blank_cell = self.erase_blank_cell();
                erase_cell_range(&mut cells, start, end, &blank_cell);
                self.rebuild_line_from_cells(cells);
            }
            return;
        }

        let blank_cell = self.erase_blank_cell();
        erase_unprotected_cell_range(
            &mut cells,
            start,
            end,
            self.current_row,
            &self.protected_cells,
            &blank_cell,
        );
        self.rebuild_line_from_cells(cells);
    }

    fn apply_horizontal_absolute(&mut self, payload: &[u8]) {
        let col = first_csi_numeric_parameter(payload).unwrap_or(1);
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.line_cursor_col = usize::from(col.saturating_sub(1));
        self.cursor_touched = true;
    }

    fn apply_cursor_forward(&mut self, payload: &[u8]) {
        let count = first_csi_numeric_parameter(payload).unwrap_or(1).max(1);
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.line_cursor_col = self.line_cursor_col.saturating_add(usize::from(count));
        self.cursor_touched = true;
    }

    fn apply_cursor_backward(&mut self, payload: &[u8]) {
        let count = first_csi_numeric_parameter(payload).unwrap_or(1).max(1) as usize;
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.line_cursor_col = self.line_cursor_col.saturating_sub(count);
        self.cursor_touched = true;
    }

    fn apply_cursor_forward_tabulation(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        for _ in 0..count {
            self.line_cursor_col = next_terminal_tab_stop(
                self.line_cursor_col,
                self.default_tab_stops,
                &self.explicit_tab_stops,
                &self.cleared_default_tab_stops,
            );
        }
        self.cursor_touched = true;
    }

    fn apply_cursor_backward_tabulation(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        for _ in 0..count {
            self.line_cursor_col = previous_terminal_tab_stop(
                self.line_cursor_col,
                self.default_tab_stops,
                &self.explicit_tab_stops,
                &self.cleared_default_tab_stops,
            );
        }
        self.cursor_touched = true;
    }

    fn set_tab_stop_at_cursor(&mut self) {
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.explicit_tab_stops.insert(self.line_cursor_col);
    }

    fn apply_tab_clear(&mut self, payload: &[u8]) {
        match first_csi_numeric_parameter(payload).unwrap_or(0) {
            0 => {
                self.explicit_tab_stops.remove(&self.line_cursor_col);
                if is_default_terminal_tab_stop(self.line_cursor_col) {
                    self.cleared_default_tab_stops.insert(self.line_cursor_col);
                }
            }
            3 => {
                self.default_tab_stops = false;
                self.explicit_tab_stops.clear();
                self.cleared_default_tab_stops.clear();
            }
            _ => {}
        }
    }

    fn apply_cursor_up(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.move_relative_vertical(self.current_row.saturating_sub(count));
        self.cursor_touched = true;
    }

    fn apply_cursor_down(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.move_relative_vertical(self.current_row.saturating_add(count));
        self.cursor_touched = true;
    }

    fn apply_cursor_next_line(&mut self, payload: &[u8]) {
        self.apply_cursor_down(payload);
        self.line_cursor_col = 0;
    }

    fn apply_cursor_previous_line(&mut self, payload: &[u8]) {
        self.apply_cursor_up(payload);
        self.line_cursor_col = 0;
    }

    fn apply_cursor_position(&mut self, payload: &[u8]) {
        let (row, col) = first_two_csi_numeric_parameters(payload);
        self.move_vertical(usize::from(row.unwrap_or(1).max(1) - 1));
        self.line_cursor_col = usize::from(col.unwrap_or(1).max(1) - 1);
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.cursor_touched = true;
    }

    fn apply_vertical_absolute(&mut self, payload: &[u8]) {
        let row = first_csi_numeric_parameter(payload).unwrap_or(1).max(1);
        self.move_vertical(usize::from(row - 1));
        self.cursor_touched = true;
    }

    fn apply_reverse_index(&mut self) {
        if let Some(region) = self.active_scroll_region()
            && self.current_row == region.top
        {
            self.scroll_region_down(region, 1);
            self.load_row(region.top);
            self.cursor_touched = true;
            return;
        }
        if self.current_row == 0 {
            return;
        }
        self.load_row(self.current_row - 1);
        self.cursor_touched = true;
    }

    fn apply_mode_set(&mut self, payload: &[u8]) {
        let modes = csi_modes(payload);
        if modes.contains(&4) {
            self.insert_mode = true;
        }

        let modes = csi_private_modes(payload);
        if modes.contains(&12) {
            self.cursor_blinking = true;
            self.cursor_touched = true;
        }
        if modes.contains(&25) {
            self.cursor_hidden = false;
            self.cursor_touched = true;
        }
        if modes.contains(&6) {
            self.origin_mode = true;
            self.load_row(self.origin_home_row());
            self.line_cursor_col = 0;
            self.cursor_touched = true;
        }
        if modes.contains(&69) {
            self.left_right_margin_mode = true;
        }
        if modes.contains(&1048) {
            self.save_line_state();
        }
        if modes.iter().any(|mode| matches!(mode, 47 | 1047 | 1049)) {
            self.enter_alternate_screen();
        }
    }

    fn apply_mode_reset(&mut self, payload: &[u8]) {
        let modes = csi_modes(payload);
        if modes.contains(&4) {
            self.insert_mode = false;
        }

        let modes = csi_private_modes(payload);
        if modes.contains(&12) {
            self.cursor_blinking = false;
            self.cursor_touched = true;
        }
        if modes.contains(&25) {
            self.cursor_hidden = true;
            self.cursor_touched = true;
        }
        if modes.contains(&6) {
            self.origin_mode = false;
            self.load_row(0);
            self.line_cursor_col = 0;
            self.cursor_touched = true;
        }
        if modes.contains(&69) {
            self.left_right_margin_mode = false;
            self.horizontal_margins = None;
        }
        if modes.contains(&1048) {
            self.restore_line_state();
        }
        if modes.iter().any(|mode| matches!(mode, 47 | 1047 | 1049)) {
            self.leave_alternate_screen();
        }
    }

    fn apply_soft_terminal_reset(&mut self) {
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.insert_mode = false;
        self.scroll_region = None;
        self.origin_mode = false;
        self.left_right_margin_mode = false;
        self.horizontal_margins = None;
        self.protected_mode = false;
        self.current_style = ScreenTextStyle::default();
        self.sgr_stack.clear();
        self.cursor_hidden = false;
        self.cursor_shape = None;
        self.cursor_blinking = false;
        self.g0_charset = GraphicCharset::Ascii;
        self.g1_charset = GraphicCharset::Ascii;
        self.active_charset = ActiveCharset::G0;
        self.saved_line_state = None;
    }

    fn apply_cursor_shape(&mut self, payload: &[u8]) {
        let Some(code) = first_csi_numeric_parameter(payload).or(Some(0)) else {
            return;
        };
        let Some((shape, blinking)) = cursor_shape_from_decscusr(code) else {
            return;
        };
        self.cursor_shape = Some(shape);
        self.cursor_blinking = blinking;
        self.cursor_touched = true;
    }

    fn apply_character_protection(&mut self, payload: &[u8]) {
        self.protected_mode = matches!(first_csi_numeric_parameter(payload).unwrap_or(0), 1);
    }

    fn apply_horizontal_margins(&mut self, payload: &[u8]) {
        if !self.left_right_margin_mode {
            return;
        }
        let Some(region) = self.operation_scroll_region() else {
            return;
        };
        let projected_width = self
            .max_projected_line_width(region.top, region.bottom)
            .unwrap_or_else(|| self.line_cursor_col.saturating_add(1))
            .max(self.line_cursor_col.saturating_add(1))
            .max(1);
        let (left, right) = first_two_csi_numeric_parameters(payload);
        let left = usize::from(left.unwrap_or(1));
        let right = usize::from(
            right.unwrap_or_else(|| u16::try_from(projected_width).unwrap_or(u16::MAX)),
        );
        let left = if left == 0 { 0 } else { left - 1 }.min(projected_width - 1);
        let right = if right == 0 {
            projected_width - 1
        } else {
            right.saturating_sub(1).min(projected_width - 1)
        };
        if left >= right {
            return;
        }

        self.horizontal_margins = Some(HorizontalMargins { left, right });
        self.load_row(self.origin_home_row());
        self.line_cursor_col = left;
        self.cursor_touched = true;
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
    }

    fn apply_scroll_region(&mut self, payload: &[u8]) {
        let (top, bottom) = first_two_csi_numeric_parameters(payload);
        if top.is_none() && bottom.is_none() {
            self.scroll_region = None;
            self.load_row(self.origin_home_row());
            self.line_cursor_col = 0;
            self.cursor_touched = true;
            return;
        }

        let top = usize::from(top.unwrap_or(1).max(1).saturating_sub(1));
        let default_bottom = self.projected_row_count().min(usize::from(u16::MAX)) as u16;
        let bottom = bottom.unwrap_or(default_bottom.max(1));
        let bottom = usize::from(bottom.max(1).saturating_sub(1));
        if top < bottom {
            self.scroll_region = Some(ScrollRegion { top, bottom });
            self.ensure_region_rows(ScrollRegion { top, bottom });
        } else {
            self.scroll_region = None;
        }
        self.load_row(self.origin_home_row());
        self.line_cursor_col = 0;
        self.cursor_touched = true;
    }

    fn apply_insert_lines(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        if let Some(region) = self.active_scroll_region() {
            if self.current_row < region.top || self.current_row > region.bottom {
                return;
            }
            self.store_current_line();
            self.ensure_region_rows(region);
            let count = count.min(region.bottom.saturating_sub(self.current_row).saturating_add(1));
            self.insert_protection_rows_within(self.current_row, region.bottom, count);
            self.lines.splice(
                self.current_row..self.current_row,
                std::iter::repeat_with(|| ScreenLine::plain("")).take(count),
            );
            let drain_from = region.bottom.saturating_add(1);
            let drain_to = drain_from.saturating_add(count).min(self.lines.len());
            self.lines.drain(drain_from..drain_to);
            self.load_current_line();
            return;
        }
        self.store_current_line();
        if self.current_row > self.lines.len() {
            self.lines.resize_with(self.current_row, || ScreenLine::plain(""));
        }
        self.insert_protection_rows_unbounded(self.current_row, count);
        self.lines.splice(
            self.current_row..self.current_row,
            std::iter::repeat_with(|| ScreenLine::plain("")).take(count),
        );
        self.load_current_line();
    }

    fn apply_delete_lines(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        if let Some(region) = self.active_scroll_region() {
            if self.current_row < region.top || self.current_row > region.bottom {
                return;
            }
            self.store_current_line();
            self.ensure_region_rows(region);
            let count = count.min(region.bottom.saturating_sub(self.current_row).saturating_add(1));
            self.delete_protection_rows_within(self.current_row, region.bottom, count);
            self.lines.drain(self.current_row..self.current_row + count);
            let insert_at = region.bottom.saturating_add(1).saturating_sub(count);
            self.lines.splice(
                insert_at..insert_at,
                std::iter::repeat_with(|| ScreenLine::plain("")).take(count),
            );
            self.load_current_line();
            return;
        }
        self.store_current_line();
        if self.current_row >= self.lines.len() {
            self.load_current_line();
            return;
        }
        let original_len = self.lines.len();
        let delete_to = self.current_row.saturating_add(count).min(self.lines.len());
        self.delete_protection_rows_unbounded(
            self.current_row,
            delete_to.saturating_sub(self.current_row),
        );
        self.lines.drain(self.current_row..delete_to);
        while self.lines.len() < original_len {
            self.lines.push(ScreenLine::plain(""));
        }
        self.load_current_line();
    }

    fn apply_scroll_up(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        let Some(region) = self.operation_scroll_region() else {
            return;
        };
        self.scroll_region_up(region, count);
    }

    fn apply_scroll_down(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        let Some(region) = self.operation_scroll_region() else {
            return;
        };
        self.scroll_region_down(region, count);
    }

    fn apply_insert_characters(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        let mut cells = self.current_line_cells();
        let insert_at = self.line_cursor_col.min(cells.len());
        self.clear_current_row_wide_cluster_protection_at(&cells, insert_at);
        self.shift_current_row_protection_right(insert_at, count);
        clear_wide_cluster_at(&mut cells, insert_at);
        let blank_cell = self.erase_blank_cell();
        cells.splice(
            insert_at..insert_at,
            std::iter::repeat_with(|| blank_cell.clone()).take(count),
        );
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.rebuild_line_from_cells(cells);
    }

    fn apply_delete_characters(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        let mut cells = self.current_line_cells();
        let delete_at = self.line_cursor_col.min(cells.len());
        let delete_to = delete_at.saturating_add(count).min(cells.len());
        self.clear_current_row_wide_cluster_protection_at(&cells, delete_at);
        self.clear_current_row_wide_cluster_protection_at(&cells, delete_to.saturating_sub(1));
        self.delete_current_row_protection_range(delete_at, delete_to);
        clear_wide_cluster_at(&mut cells, delete_at);
        clear_wide_cluster_at(&mut cells, delete_to.saturating_sub(1));
        cells.drain(delete_at..delete_to);
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.rebuild_line_from_cells(cells);
    }

    fn apply_insert_columns(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.apply_terminal_column_edit(count, TerminalColumnEditMode::Insert);
    }

    fn apply_delete_columns(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.apply_terminal_column_edit(count, TerminalColumnEditMode::Delete);
    }

    fn apply_scroll_left_columns(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.apply_terminal_column_edit(count, TerminalColumnEditMode::ScrollLeft);
    }

    fn apply_scroll_right_columns(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        self.apply_terminal_column_edit(count, TerminalColumnEditMode::ScrollRight);
    }

    fn apply_back_index(&mut self) {
        let left_margin = self
            .operation_scroll_region()
            .and_then(|region| {
                let width = self
                    .max_projected_line_width(region.top, region.bottom)
                    .unwrap_or_else(|| self.line_cursor_col.saturating_add(1))
                    .max(self.line_cursor_col.saturating_add(1));
                self.active_horizontal_region(width)
            })
            .map(|region| region.left)
            .unwrap_or(0);
        if self.line_cursor_col > left_margin {
            self.pending_carriage_return = false;
            self.pending_backspace_cell = None;
            self.line_cursor_col -= 1;
            self.cursor_touched = true;
            return;
        }
        if self.line_cursor_col < left_margin && self.line_cursor_col > 0 {
            self.pending_carriage_return = false;
            self.pending_backspace_cell = None;
            self.line_cursor_col -= 1;
            self.cursor_touched = true;
            return;
        }

        self.apply_terminal_column_edit(1, TerminalColumnEditMode::ScrollRight);
    }

    fn apply_forward_index(&mut self) {
        let Some(region) = self.operation_scroll_region() else {
            self.pending_carriage_return = false;
            self.pending_backspace_cell = None;
            self.line_cursor_col = self.line_cursor_col.saturating_add(1);
            self.cursor_touched = true;
            return;
        };
        let terminal_width = self
            .max_projected_line_width(region.top, region.bottom)
            .unwrap_or(self.line_cursor_col.saturating_add(1))
            .max(self.line_cursor_col.saturating_add(1));
        let right_margin = self
            .active_horizontal_region(terminal_width)
            .map(|region| region.right)
            .unwrap_or(terminal_width.saturating_sub(1));
        if self.line_cursor_col < right_margin {
            self.pending_carriage_return = false;
            self.pending_backspace_cell = None;
            self.line_cursor_col += 1;
            self.cursor_touched = true;
            return;
        }
        if self.line_cursor_col > right_margin {
            self.pending_carriage_return = false;
            self.pending_backspace_cell = None;
            self.line_cursor_col = self.line_cursor_col.saturating_add(1);
            self.cursor_touched = true;
            return;
        }

        self.apply_terminal_column_edit(1, TerminalColumnEditMode::ScrollLeft);
    }

    fn apply_terminal_column_edit(&mut self, count: usize, mode: TerminalColumnEditMode) {
        let Some(region) = self.operation_scroll_region() else {
            return;
        };
        let cursor_scoped =
            matches!(mode, TerminalColumnEditMode::Insert | TerminalColumnEditMode::Delete);
        if cursor_scoped && (self.current_row < region.top || self.current_row > region.bottom) {
            return;
        }

        self.store_current_line();
        self.ensure_region_rows(region);

        let original_row = self.current_row;
        let original_col = self.line_cursor_col;
        let start_col = match mode {
            TerminalColumnEditMode::Insert | TerminalColumnEditMode::Delete => original_col,
            TerminalColumnEditMode::ScrollLeft | TerminalColumnEditMode::ScrollRight => 0,
        };
        let minimum_width = match mode {
            TerminalColumnEditMode::Insert | TerminalColumnEditMode::Delete => {
                start_col.saturating_add(count)
            }
            TerminalColumnEditMode::ScrollLeft | TerminalColumnEditMode::ScrollRight => count,
        };
        let terminal_width = self
            .max_projected_line_width(region.top, region.bottom)
            .unwrap_or(minimum_width)
            .max(minimum_width);
        let Some(horizontal_region) = self.active_horizontal_region(terminal_width) else {
            self.load_row(original_row);
            self.line_cursor_col = original_col;
            return;
        };
        let (start_col, end_col) = match mode {
            TerminalColumnEditMode::Insert | TerminalColumnEditMode::Delete => {
                if original_col < horizontal_region.left || original_col > horizontal_region.right {
                    self.load_row(original_row);
                    self.line_cursor_col = original_col;
                    return;
                }
                (original_col, horizontal_region.right.saturating_add(1))
            }
            TerminalColumnEditMode::ScrollLeft | TerminalColumnEditMode::ScrollRight => {
                (horizontal_region.left, horizontal_region.right.saturating_add(1))
            }
        };
        if start_col >= end_col || end_col > terminal_width {
            self.load_row(original_row);
            self.line_cursor_col = original_col;
            return;
        }

        let region_width = end_col - start_col;
        let count = match mode {
            TerminalColumnEditMode::Insert | TerminalColumnEditMode::Delete => {
                count.min(region_width)
            }
            TerminalColumnEditMode::ScrollLeft | TerminalColumnEditMode::ScrollRight => {
                count.min(region_width)
            }
        };
        if count == 0 {
            self.load_row(original_row);
            self.line_cursor_col = original_col;
            return;
        }

        let blank_cell = self.erase_blank_cell();
        for row in region.top..=region.bottom {
            self.load_row(row);
            let mut cells = self.current_line_cells();
            extend_cells_to_width_with_cell(&mut cells, terminal_width, &blank_cell);
            clear_wide_cluster_at_with_cell(&mut cells, start_col, &blank_cell);
            match mode {
                TerminalColumnEditMode::Insert => {
                    clear_wide_cluster_at_with_cell(
                        &mut cells,
                        end_col.saturating_sub(count).saturating_sub(1),
                        &blank_cell,
                    );
                    let mut next_cells = cells.iter().take(start_col).cloned().collect::<Vec<_>>();
                    next_cells.extend(std::iter::repeat_with(|| blank_cell.clone()).take(count));
                    next_cells.extend(
                        cells
                            .iter()
                            .skip(start_col)
                            .take(end_col.saturating_sub(start_col + count))
                            .cloned(),
                    );
                    next_cells.extend(cells.iter().skip(end_col).cloned());
                    self.shift_row_protection_right_within(row, start_col, count, end_col);
                    self.clear_row_protection_range(row, start_col, start_col + count);
                    self.rebuild_line_from_cells(next_cells);
                }
                TerminalColumnEditMode::Delete => {
                    clear_wide_cluster_at_with_cell(&mut cells, start_col + count, &blank_cell);
                    cells.drain(start_col..start_col + count);
                    cells.splice(
                        end_col - count..end_col - count,
                        std::iter::repeat_with(|| blank_cell.clone()).take(count),
                    );
                    self.delete_row_protection_range_within(
                        row,
                        start_col,
                        start_col + count,
                        end_col,
                    );
                    self.rebuild_line_from_cells(cells);
                }
                TerminalColumnEditMode::ScrollLeft => {
                    clear_wide_cluster_at_with_cell(&mut cells, start_col + count, &blank_cell);
                    cells.drain(start_col..start_col + count);
                    cells.splice(
                        end_col - count..end_col - count,
                        std::iter::repeat_with(|| blank_cell.clone()).take(count),
                    );
                    self.delete_row_protection_range_within(
                        row,
                        start_col,
                        start_col + count,
                        end_col,
                    );
                    self.rebuild_line_from_cells(cells);
                }
                TerminalColumnEditMode::ScrollRight => {
                    clear_wide_cluster_at_with_cell(
                        &mut cells,
                        end_col.saturating_sub(count).saturating_sub(1),
                        &blank_cell,
                    );
                    let mut next_cells = cells.iter().take(start_col).cloned().collect::<Vec<_>>();
                    next_cells.extend(std::iter::repeat_with(|| blank_cell.clone()).take(count));
                    next_cells.extend(
                        cells
                            .iter()
                            .skip(start_col)
                            .take(end_col.saturating_sub(start_col + count))
                            .cloned(),
                    );
                    next_cells.extend(cells.iter().skip(end_col).cloned());
                    self.shift_row_protection_right_within(row, start_col, count, end_col);
                    self.clear_row_protection_range(row, start_col, start_col + count);
                    self.rebuild_line_from_cells(next_cells);
                }
            }
        }

        self.load_row(original_row);
        self.line_cursor_col = original_col;
        self.cursor_touched = true;
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
    }

    fn apply_erase_characters(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        let mut cells = self.current_line_cells();
        let erase_at = self.line_cursor_col.min(cells.len());
        let erase_to = erase_at.saturating_add(count).min(cells.len());
        let blank_cell = self.erase_blank_cell();
        erase_cell_range(&mut cells, erase_at, erase_to, &blank_cell);
        self.clear_current_row_protection_range(erase_at, erase_to);
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.rebuild_line_from_cells(cells);
    }

    fn apply_repeat_preceding_character(&mut self, payload: &[u8]) {
        let count = usize::from(first_csi_numeric_parameter(payload).unwrap_or(1).max(1));
        let mut cells = self.current_line_cells();
        let Some(repeated) = repeatable_cell_before_cursor(&cells, self.line_cursor_col) else {
            return;
        };

        while self.line_cursor_col > cells.len() {
            cells.push(styled_space_cell(ScreenTextStyle::default()));
        }

        for _ in 0..count {
            write_styled_cell_at(&mut cells, self.line_cursor_col, &repeated);
            self.write_current_row_protection(self.line_cursor_col, repeated.columns.max(1));
            self.line_cursor_col = self.line_cursor_col.saturating_add(repeated.columns.max(1));
        }

        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.rebuild_line_from_cells(cells);
    }

    fn apply_rectangular_attributes(&mut self, payload: &[u8], mode: RectangularAttributeMode) {
        let row_count = self.projected_row_count();
        let Some(request) = parse_terminal_rectangular_attribute_request(payload) else {
            return;
        };

        let top = usize::from(request.top.saturating_sub(1));
        let bottom = usize::from(request.bottom.unwrap_or(row_count as u16).saturating_sub(1));
        let left = usize::from(request.left.saturating_sub(1));
        let right = request
            .right
            .map(usize::from)
            .or_else(|| self.max_projected_line_width(top, bottom))
            .unwrap_or(usize::from(request.left))
            .max(usize::from(request.left))
            .saturating_sub(1);

        let saved_cursor = self.saved_screen_cursor_state();
        self.store_current_line();

        for row in top..=bottom {
            self.load_row(row);
            let mut cells = self.current_line_cells();
            if request.right.is_some() {
                while cells.len() <= right {
                    cells.push(styled_space_cell(ScreenTextStyle::default()));
                }
            }
            let end = right.saturating_add(1).min(cells.len());
            if left < end {
                for col in left..end {
                    apply_rectangular_attribute_to_cell(&mut cells, col, &request.actions, mode);
                }
                self.rebuild_line_from_cells(cells);
            }
        }

        self.restore_screen_cursor_state(saved_cursor);
    }

    fn apply_fill_rectangular_area(&mut self, payload: &[u8]) {
        let Some(request) = parse_terminal_rectangular_fill_request(payload) else {
            return;
        };
        let Some(ch) = char::from_u32(request.codepoint) else {
            return;
        };
        if ch.is_control() || terminal_char_width(ch) != 1 {
            return;
        }
        let Some(area) = self.resolve_rectangular_area(request.area) else {
            return;
        };

        let fill_cell = styled_text_cell(ch, self.current_style.clone(), 1);
        let saved_cursor = self.saved_screen_cursor_state();
        self.store_current_line();

        for row in area.top..=area.bottom {
            self.load_row(row);
            let mut cells = self.current_line_cells();
            while cells.len() <= area.right {
                cells.push(styled_space_cell(ScreenTextStyle::default()));
            }
            for col in area.left..=area.right {
                write_styled_cell_at(&mut cells, col, &fill_cell);
                self.write_current_row_protection(col, 1);
            }
            self.rebuild_line_from_cells(cells);
        }

        self.restore_screen_cursor_state(saved_cursor);
    }

    fn apply_erase_rectangular_area(&mut self, payload: &[u8], selective: bool) {
        let Some(request) = parse_terminal_rectangular_area(payload) else {
            return;
        };
        let Some(area) = self.resolve_rectangular_area(request) else {
            return;
        };

        let saved_cursor = self.saved_screen_cursor_state();
        self.store_current_line();

        for row in area.top..=area.bottom {
            self.load_row(row);
            let mut cells = self.current_line_cells();
            let blank_cell = self.erase_blank_cell();
            if !selective {
                while cells.len() <= area.right {
                    cells.push(blank_cell.clone());
                }
            }
            let end = area.right.saturating_add(1).min(cells.len());
            if area.left >= end {
                continue;
            }
            if selective {
                erase_unprotected_cell_range(
                    &mut cells,
                    area.left,
                    end,
                    row,
                    &self.protected_cells,
                    &blank_cell,
                );
            } else {
                erase_cell_range(&mut cells, area.left, end, &blank_cell);
                self.clear_current_row_protection_range(area.left, end);
            }
            self.rebuild_line_from_cells(cells);
        }

        self.restore_screen_cursor_state(saved_cursor);
    }

    fn apply_copy_rectangular_area(&mut self, payload: &[u8]) {
        let Some(request) = parse_terminal_rectangular_copy_request(payload) else {
            return;
        };
        let Some(source) = self.resolve_rectangular_area(request.source) else {
            return;
        };

        let destination_top = usize::from(request.destination_top.saturating_sub(1));
        let destination_left = usize::from(request.destination_left.saturating_sub(1));
        let row_count = self.projected_row_count();
        if destination_top >= row_count {
            return;
        }

        let saved_cursor = self.saved_screen_cursor_state();
        self.store_current_line();
        let snapshot = self.copy_rectangular_area_snapshot(source);

        for (row_offset, copied_row) in snapshot.iter().enumerate() {
            let destination_row = destination_top.saturating_add(row_offset);
            if destination_row >= row_count {
                break;
            }

            self.load_row(destination_row);
            let mut cells = self.current_line_cells();
            let copy_width = copied_row.len();
            for (col_offset, copied) in copied_row.iter().enumerate() {
                let destination_col = destination_left.saturating_add(col_offset);
                while cells.len() <= destination_col {
                    cells.push(styled_space_cell(ScreenTextStyle::default()));
                }

                let Some(cell) = rectangular_copy_cell_for_write(copied_row, col_offset) else {
                    continue;
                };
                write_styled_cell_at(&mut cells, destination_col, &cell);
                let protected_width = cell.columns.max(1).min(copy_width - col_offset);
                self.write_copied_cell_protection(
                    destination_col,
                    protected_width,
                    copied.protected,
                );
            }
            self.rebuild_line_from_cells(cells);
        }

        self.restore_screen_cursor_state(saved_cursor);
    }

    fn copy_rectangular_area_snapshot(
        &mut self,
        source: ResolvedRectangularArea,
    ) -> Vec<Vec<RectangularCopyCell>> {
        let width = source.right.saturating_sub(source.left).saturating_add(1);
        let mut snapshot = Vec::new();
        for row in source.top..=source.bottom {
            self.load_row(row);
            let cells = self.current_line_cells();
            let copied_row = (0..width)
                .map(|offset| {
                    let col = source.left.saturating_add(offset);
                    let cell = cells
                        .get(col)
                        .cloned()
                        .unwrap_or_else(|| styled_space_cell(ScreenTextStyle::default()));
                    RectangularCopyCell {
                        cell,
                        protected: self.protected_cells.contains(&(row, col)),
                    }
                })
                .collect();
            snapshot.push(copied_row);
        }
        snapshot
    }

    fn write_copied_cell_protection(&mut self, start: usize, width: usize, protected: bool) {
        let end = start.saturating_add(width);
        if protected {
            self.protect_current_row_range(start, end);
        } else {
            self.clear_current_row_protection_range(start, end);
        }
    }

    fn resolve_rectangular_area(
        &self,
        request: TerminalRectangularArea,
    ) -> Option<ResolvedRectangularArea> {
        let row_count = self.projected_row_count();
        let top = usize::from(request.top.saturating_sub(1));
        let bottom = usize::from(request.bottom.unwrap_or(row_count as u16).saturating_sub(1));
        let left = usize::from(request.left.saturating_sub(1));
        let right = request
            .right
            .map(usize::from)
            .or_else(|| self.max_projected_line_width(top, bottom))
            .unwrap_or(usize::from(request.left))
            .max(usize::from(request.left))
            .saturating_sub(1);
        (top <= bottom && left <= right).then_some(ResolvedRectangularArea {
            top,
            bottom,
            left,
            right,
        })
    }

    fn saved_screen_cursor_state(&self) -> SavedScreenCursorState {
        SavedScreenCursorState {
            row: self.current_row,
            cursor_col: self.line_cursor_col,
            cursor_touched: self.cursor_touched,
            cursor_hidden: self.cursor_hidden,
            cursor_shape: self.cursor_shape,
            cursor_blinking: self.cursor_blinking,
        }
    }

    fn restore_screen_cursor_state(&mut self, state: SavedScreenCursorState) {
        self.load_row(state.row);
        self.line_cursor_col = state.cursor_col;
        self.cursor_touched = state.cursor_touched;
        self.cursor_hidden = state.cursor_hidden;
        self.cursor_shape = state.cursor_shape;
        self.cursor_blinking = state.cursor_blinking;
    }

    fn clear_current_line(&mut self) {
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.clear_row_protection(self.current_row);
        self.line_text.clear();
        self.line_spans.clear();
        self.line_media.clear();
        self.line_side_effects.clear();
        self.line_semantic_marks.clear();
        self.line_cursor_col = 0;
    }

    fn clear_current_line_contents_preserving_cursor(&mut self) {
        self.pending_carriage_return = false;
        self.pending_backspace_cell = None;
        self.clear_row_protection(self.current_row);
        self.line_text.clear();
        self.line_spans.clear();
        self.line_media.clear();
        self.line_side_effects.clear();
        self.line_semantic_marks.clear();
    }

    fn reset_terminal_state(&mut self) {
        self.lines.clear();
        self.current_row = 0;
        self.cursor_touched = false;
        self.cursor_hidden = false;
        self.cursor_shape = None;
        self.cursor_blinking = false;
        self.clear_current_line();
        self.current_style = ScreenTextStyle::default();
        self.sgr_stack.clear();
        self.title = None;
        self.working_directory_uri = None;
        self.user_variables.clear();
        self.palette = ScreenSurfacePalette::default();
        self.dynamic_palette.clear();
        self.color_stack.clear();
        self.bell_count = 0;
        self.progress = ScreenProgress::default();
        self.scroll_region = None;
        self.origin_mode = false;
        self.left_right_margin_mode = false;
        self.horizontal_margins = None;
        self.protected_mode = false;
        self.protected_cells.clear();
        self.insert_mode = false;
        self.default_tab_stops = true;
        self.explicit_tab_stops.clear();
        self.cleared_default_tab_stops.clear();
        self.g0_charset = GraphicCharset::Ascii;
        self.g1_charset = GraphicCharset::Ascii;
        self.active_charset = ActiveCharset::G0;
        self.saved_line_state = None;
        self.normal_screen_state = None;
        self.kitty_chunk = None;
        self.iterm2_multipart_file = None;
    }

    fn apply_color_stack_osc(&mut self, payload: &[u8]) {
        match parse_terminal_kitty_color_stack(payload) {
            Some(TerminalKittyColorStackOperation::Push) => self.push_color_stack_snapshot(),
            Some(TerminalKittyColorStackOperation::Pop) => self.pop_color_stack_snapshot(),
            None => {}
        }
    }

    fn apply_color_stack_operations(&mut self, operations: Vec<TerminalXtermColorStackOperation>) {
        for operation in operations {
            match operation {
                TerminalXtermColorStackOperation::Push => self.push_color_stack_snapshot(),
                TerminalXtermColorStackOperation::Pop => self.pop_color_stack_snapshot(),
                TerminalXtermColorStackOperation::Store(slot) => {
                    self.store_color_stack_snapshot(slot)
                }
                TerminalXtermColorStackOperation::Restore(slot) => {
                    self.restore_color_stack_snapshot(slot);
                }
                TerminalXtermColorStackOperation::Report => {}
            }
        }
    }

    fn push_color_stack_snapshot(&mut self) {
        if self.color_stack.len() >= MAX_COLOR_STACK_DEPTH {
            self.color_stack.remove(0);
        }
        self.color_stack.push(self.current_color_palette_snapshot());
    }

    fn pop_color_stack_snapshot(&mut self) {
        if let Some(snapshot) = self.color_stack.pop() {
            self.restore_color_palette_snapshot(snapshot);
        }
    }

    fn store_color_stack_snapshot(&mut self, slot: usize) {
        if slot == 0 || slot > MAX_COLOR_STACK_DEPTH {
            return;
        }
        let index = slot - 1;
        if self.color_stack.len() <= index {
            self.color_stack.resize_with(index + 1, ColorPaletteSnapshot::default);
        }
        self.color_stack[index] = self.current_color_palette_snapshot();
    }

    fn restore_color_stack_snapshot(&mut self, slot: usize) {
        if slot == 0 {
            self.pop_color_stack_snapshot();
            return;
        }
        if let Some(snapshot) = self.color_stack.get(slot - 1).cloned() {
            self.restore_color_palette_snapshot(snapshot);
        }
    }

    fn current_color_palette_snapshot(&self) -> ColorPaletteSnapshot {
        ColorPaletteSnapshot {
            palette: self.palette.clone(),
            dynamic_palette: self.dynamic_palette.clone(),
        }
    }

    fn restore_color_palette_snapshot(&mut self, snapshot: ColorPaletteSnapshot) {
        self.palette = snapshot.palette;
        self.dynamic_palette = snapshot.dynamic_palette;
    }

    fn consume_osc(&mut self, bytes: &[u8], index: usize) -> usize {
        let Some(control_string) = read_osc_control_string(bytes, index) else {
            return bytes.len();
        };
        let payload = control_string.payload;
        if let Some(hyperlink) = parse_osc8_hyperlink(payload) {
            self.current_style.hyperlink = hyperlink;
        }
        if let Some(media) = terminal_iterm2_multipart_file_media(
            &mut self.iterm2_multipart_file,
            payload,
            control_string.truncated,
        ) {
            self.push_media(media);
        }
        if let Some(media) = terminal_media_from_osc_payload(payload, control_string.truncated) {
            self.push_media(media);
        }
        if let Some(side_effect) = terminal_side_effect_from_osc_payload(payload) {
            self.push_side_effect(side_effect);
        }
        if let Some(title) = terminal_title_from_osc_payload(payload) {
            self.title = Some(title);
        }
        if let Some(working_directory_uri) =
            terminal_working_directory_uri_from_osc_payload(payload)
        {
            self.working_directory_uri = working_directory_uri;
        }
        if let Some((key, value)) = terminal_user_variable_from_osc(payload) {
            self.user_variables.insert(key, value);
        }
        if let Some(progress) = terminal_progress_from_osc(payload) {
            self.progress = progress;
        }
        if let Some(shape) = terminal_cursor_shape_from_osc_payload(payload) {
            self.cursor_shape = Some(shape);
            self.cursor_touched = true;
        }
        self.apply_color_stack_osc(payload);
        apply_surface_palette_osc(&mut self.palette, payload);
        apply_dynamic_palette_osc(&mut self.dynamic_palette, payload);
        if let Some(mut mark) = terminal_shell_integration_mark(payload) {
            mark.col = self.current_col();
            self.push_semantic_mark(mark);
        }
        control_string.next_index
    }

    fn consume_dcs(&mut self, bytes: &[u8], index: usize) -> usize {
        let Some(control_string) = read_st_control_string(bytes, index) else {
            return bytes.len();
        };
        if let Some(passthrough) =
            terminal_tmux_passthrough_payload(control_string.payload, control_string.truncated)
        {
            self.push_bytes_lossy(&passthrough);
        } else if let Some(media) =
            terminal_media_from_dcs_payload(control_string.payload, control_string.truncated)
        {
            self.push_media(media);
        }
        control_string.next_index
    }

    fn consume_apc(&mut self, bytes: &[u8], index: usize) -> usize {
        let Some(control_string) = read_st_control_string(bytes, index) else {
            return bytes.len();
        };
        if let Some(media) = terminal_media_from_apc_payload(
            &mut self.kitty_chunk,
            control_string.payload,
            control_string.truncated,
        ) {
            self.push_media(media);
        }
        control_string.next_index
    }

    fn push_media(&mut self, media: ScreenLineMedia) {
        self.line_media.push(media);
    }

    fn push_side_effect(&mut self, side_effect: ScreenLineSideEffect) {
        self.line_side_effects.push(side_effect);
    }

    fn push_semantic_mark(&mut self, mark: ScreenLineSemanticMark) {
        self.line_semantic_marks.push(mark);
    }

    fn push_bytes_lossy(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        self.push_str(&text);
    }

    fn take_overstrike_cell(&mut self, ch: char) -> Option<(char, ScreenTextStyle)> {
        let previous = self.pending_backspace_cell.take()?;
        let (ch, mut style) = cell_for_overstrike(&previous, ch, &self.current_style)?;
        merge_missing_style_fields(&mut style, &previous.style);
        Some((ch, style))
    }

    fn current_col(&self) -> u16 {
        self.line_cursor_col.min(usize::from(u16::MAX)) as u16
    }

    fn active_graphic_charset(&self) -> GraphicCharset {
        match self.active_charset {
            ActiveCharset::G0 => self.g0_charset,
            ActiveCharset::G1 => self.g1_charset,
        }
    }

    fn map_active_charset_byte(&self, byte: u8, char_len: usize) -> Option<char> {
        if char_len != 1 || self.active_graphic_charset() != GraphicCharset::DecSpecialGraphics {
            return None;
        }
        dec_special_graphics_char(byte)
    }
}

fn terminal_char_width(ch: char) -> usize {
    UnicodeWidthChar::width(ch).unwrap_or(0)
}

fn next_terminal_tab_stop(
    col: usize,
    default_tab_stops: bool,
    explicit_tab_stops: &BTreeSet<usize>,
    cleared_default_tab_stops: &BTreeSet<usize>,
) -> usize {
    let explicit = explicit_tab_stops.range(col.saturating_add(1)..).next().copied();
    let default =
        default_tab_stops.then(|| next_default_terminal_tab_stop(col, cleared_default_tab_stops));
    match (explicit, default) {
        (Some(explicit), Some(default)) => explicit.min(default),
        (Some(explicit), None) => explicit,
        (None, Some(default)) => default,
        (None, None) => col,
    }
}

fn previous_terminal_tab_stop(
    col: usize,
    default_tab_stops: bool,
    explicit_tab_stops: &BTreeSet<usize>,
    cleared_default_tab_stops: &BTreeSet<usize>,
) -> usize {
    if col == 0 {
        return 0;
    }
    let explicit = explicit_tab_stops.range(..col).next_back().copied();
    let default = default_tab_stops
        .then(|| previous_default_terminal_tab_stop(col, cleared_default_tab_stops));
    match (explicit, default) {
        (Some(explicit), Some(default)) => explicit.max(default),
        (Some(explicit), None) => explicit,
        (None, Some(default)) => default,
        (None, None) => 0,
    }
}

fn next_default_terminal_tab_stop(
    col: usize,
    cleared_default_tab_stops: &BTreeSet<usize>,
) -> usize {
    let mut candidate = col.saturating_add(DEFAULT_TAB_WIDTH - (col % DEFAULT_TAB_WIDTH));
    while cleared_default_tab_stops.contains(&candidate) {
        candidate = candidate.saturating_add(DEFAULT_TAB_WIDTH);
    }
    candidate
}

fn previous_default_terminal_tab_stop(
    col: usize,
    cleared_default_tab_stops: &BTreeSet<usize>,
) -> usize {
    let mut candidate = ((col - 1) / DEFAULT_TAB_WIDTH) * DEFAULT_TAB_WIDTH;
    while candidate > 0 && cleared_default_tab_stops.contains(&candidate) {
        candidate = candidate.saturating_sub(DEFAULT_TAB_WIDTH);
    }
    candidate
}

fn is_default_terminal_tab_stop(col: usize) -> bool {
    col > 0 && col.is_multiple_of(DEFAULT_TAB_WIDTH)
}

fn styled_text_cell(ch: char, style: ScreenTextStyle, columns: usize) -> StyledCell {
    StyledCell { text: ch.to_string(), style, columns: columns.max(1), spacer: false }
}

fn styled_space_cell(style: ScreenTextStyle) -> StyledCell {
    StyledCell { text: " ".to_string(), style, columns: 1, spacer: false }
}

fn styled_spacer_cell(style: ScreenTextStyle) -> StyledCell {
    StyledCell { text: String::new(), style, columns: 1, spacer: true }
}

fn extend_cells_to_width_with_cell(
    cells: &mut Vec<StyledCell>,
    width: usize,
    blank_cell: &StyledCell,
) {
    while cells.len() < width {
        cells.push(blank_cell.clone());
    }
}

fn screen_line_cell_width(line: &ScreenLine) -> usize {
    if !line.spans.is_empty()
        && line.spans.iter().map(|span| span.text.as_str()).collect::<String>() == line.text
    {
        let mut cells = Vec::new();
        for span in &line.spans {
            push_text_cells(&mut cells, span.text.as_str(), span.style.clone());
        }
        return cells.len();
    }

    line.text.chars().map(terminal_char_width).sum()
}

fn push_text_cells(cells: &mut Vec<StyledCell>, text: &str, style: ScreenTextStyle) {
    for ch in text.chars() {
        let width = terminal_char_width(ch);
        if width == 0 {
            append_zero_width_to_last_visible_cell(cells, ch);
            continue;
        }

        cells.push(styled_text_cell(ch, style.clone(), width));
        for _ in 1..width {
            cells.push(styled_spacer_cell(style.clone()));
        }
    }
}

fn append_zero_width_to_last_visible_cell(cells: &mut [StyledCell], ch: char) -> bool {
    let Some(cell) = cells.iter_mut().rev().find(|cell| !cell.spacer) else {
        return false;
    };
    cell.text.push(ch);
    true
}

fn erase_cell_range(cells: &mut [StyledCell], start: usize, end: usize, blank_cell: &StyledCell) {
    let end = end.min(cells.len());
    for col in start..end {
        clear_wide_cluster_at_with_cell(cells, col, blank_cell);
    }
    for cell in cells.iter_mut().take(end).skip(start) {
        *cell = blank_cell.clone();
    }
}

fn erase_unprotected_cell_range(
    cells: &mut [StyledCell],
    start: usize,
    end: usize,
    row: usize,
    protected_cells: &BTreeSet<(usize, usize)>,
    blank_cell: &StyledCell,
) {
    let end = end.min(cells.len());
    for col in start..end {
        let cluster_start = wide_cluster_start_at(cells, col).unwrap_or(col);
        let cluster_end =
            cluster_start.saturating_add(cells[cluster_start].columns.max(1)).min(cells.len());
        if (cluster_start..cluster_end)
            .any(|cluster_col| protected_cells.contains(&(row, cluster_col)))
        {
            continue;
        }
        for cell in cells.iter_mut().take(cluster_end).skip(cluster_start) {
            *cell = blank_cell.clone();
        }
    }
}

fn clear_wide_cluster_at(cells: &mut [StyledCell], col: usize) {
    clear_wide_cluster_at_with_cell(cells, col, &styled_space_cell(ScreenTextStyle::default()));
}

fn clear_wide_cluster_at_with_cell(cells: &mut [StyledCell], col: usize, blank_cell: &StyledCell) {
    if col >= cells.len() {
        return;
    }
    let start = wide_cluster_start_at(cells, col).unwrap_or(col);
    let end = start.saturating_add(cells[start].columns.max(1)).min(cells.len());
    if end.saturating_sub(start) <= 1 {
        return;
    }
    for cell in cells.iter_mut().take(end).skip(start) {
        *cell = blank_cell.clone();
    }
}

fn apply_rectangular_attribute_to_cell(
    cells: &mut [StyledCell],
    col: usize,
    actions: &[RectangularAttributeAction],
    mode: RectangularAttributeMode,
) {
    if col >= cells.len() {
        return;
    }
    let start = wide_cluster_start_at(cells, col).unwrap_or(col);
    let end = start.saturating_add(cells[start].columns.max(1)).min(cells.len());
    for cell in cells.iter_mut().take(end).skip(start) {
        apply_rectangular_attributes_to_style(&mut cell.style, actions, mode);
    }
}

fn apply_rectangular_attributes_to_style(
    style: &mut ScreenTextStyle,
    actions: &[RectangularAttributeAction],
    mode: RectangularAttributeMode,
) {
    apply_terminal_rectangular_attribute_actions(style, actions, mode);
}

fn repeatable_cell_before_cursor(cells: &[StyledCell], cursor_col: usize) -> Option<StyledCell> {
    let previous_col = cursor_col.min(cells.len()).checked_sub(1)?;
    let start = if cells.get(previous_col)?.spacer {
        wide_cluster_start_at(cells, previous_col)?
    } else {
        previous_col
    };
    let cell = cells.get(start)?;
    (!cell.text.is_empty() && !cell.spacer).then(|| cell.clone())
}

fn rectangular_copy_cell_for_write(
    row: &[RectangularCopyCell],
    offset: usize,
) -> Option<StyledCell> {
    let Some(copied) = row.get(offset) else {
        return Some(styled_space_cell(ScreenTextStyle::default()));
    };
    let cell = &copied.cell;
    let width = cell.columns.max(1);
    if cell.spacer {
        let previous_wide_cell_is_copied = offset
            .checked_sub(1)
            .and_then(|previous_offset| row.get(previous_offset))
            .is_some_and(|previous| !previous.cell.spacer && previous.cell.columns > 1);
        return if previous_wide_cell_is_copied {
            None
        } else {
            Some(styled_space_cell(ScreenTextStyle::default()))
        };
    }
    if width > 1 && offset.saturating_add(width) > row.len() {
        return Some(styled_space_cell(ScreenTextStyle::default()));
    }
    Some(cell.clone())
}

fn write_styled_cell_at(cells: &mut Vec<StyledCell>, col: usize, cell: &StyledCell) {
    let width = cell.columns.max(1);
    for target_col in col..col.saturating_add(width) {
        clear_wide_cluster_at(cells, target_col);
    }

    let mut visible = cell.clone();
    visible.spacer = false;
    visible.columns = width;
    if col < cells.len() {
        cells[col] = visible;
    } else {
        cells.push(visible);
    }

    for target_col in col + 1..col.saturating_add(width) {
        let spacer = styled_spacer_cell(cell.style.clone());
        if target_col < cells.len() {
            cells[target_col] = spacer;
        } else {
            cells.push(spacer);
        }
    }
}

fn wide_cluster_start_at(cells: &[StyledCell], col: usize) -> Option<usize> {
    if col >= cells.len() {
        return None;
    }
    if !cells[col].spacer {
        return (cells[col].columns > 1).then_some(col);
    }

    for start in (0..col).rev() {
        if cells[start].spacer {
            continue;
        }
        let end = start.saturating_add(cells[start].columns.max(1));
        return (end > col).then_some(start);
    }
    None
}

fn cell_for_overstrike(
    previous: &BackspaceCell,
    ch: char,
    current_style: &ScreenTextStyle,
) -> Option<(char, ScreenTextStyle)> {
    let mut style = current_style.clone();
    let previous_ch = previous.text.chars().next()?;
    if previous_ch == ch {
        style.bold = true;
        return Some((ch, style));
    }
    if previous_ch == '_' && ch != '_' {
        style.underline = Some(ScreenUnderlineStyle::Single);
        return Some((ch, style));
    }
    if ch == '_' && previous_ch != '_' {
        style.underline = Some(ScreenUnderlineStyle::Single);
        return Some((previous_ch, style));
    }
    None
}

fn merge_missing_style_fields(style: &mut ScreenTextStyle, fallback: &ScreenTextStyle) {
    if style.foreground.is_none() {
        style.foreground = fallback.foreground.clone();
    }
    if style.background.is_none() {
        style.background = fallback.background.clone();
    }
    if style.underline_color.is_none() {
        style.underline_color = fallback.underline_color.clone();
    }
    if !style.bold {
        style.bold = fallback.bold;
    }
    if !style.dim {
        style.dim = fallback.dim;
    }
    if !style.italic {
        style.italic = fallback.italic;
    }
    if !style.blink {
        style.blink = fallback.blink;
    }
    if style.underline.is_none() {
        style.underline = fallback.underline;
    }
    if !style.overline {
        style.overline = fallback.overline;
    }
    if style.border.is_none() {
        style.border = fallback.border;
    }
    if style.baseline.is_none() {
        style.baseline = fallback.baseline;
    }
    if !style.inverse {
        style.inverse = fallback.inverse;
    }
    if !style.hidden {
        style.hidden = fallback.hidden;
    }
    if !style.strikethrough {
        style.strikethrough = fallback.strikethrough;
    }
    if style.hyperlink.is_none() {
        style.hyperlink = fallback.hyperlink.clone();
    }
}

fn normalized_spans(text: &str, spans: Vec<ScreenLineSpan>) -> Vec<ScreenLineSpan> {
    if spans.is_empty() {
        return spans;
    }
    if spans.iter().all(|span| span.style.is_plain()) {
        return Vec::new();
    }
    if spans.iter().map(|span| span.text.as_str()).collect::<String>() != text {
        return Vec::new();
    }
    spans
}

fn dec_special_graphics_char(byte: u8) -> Option<char> {
    match byte {
        b'_' => Some(' '),
        b'`' => Some('◆'),
        b'a' => Some('▒'),
        b'f' => Some('°'),
        b'g' => Some('±'),
        b'j' => Some('┘'),
        b'k' => Some('┐'),
        b'l' => Some('┌'),
        b'm' => Some('└'),
        b'n' => Some('┼'),
        b'o' => Some('⎺'),
        b'p' => Some('⎻'),
        b'q' => Some('─'),
        b'r' => Some('⎼'),
        b's' => Some('⎽'),
        b't' => Some('├'),
        b'u' => Some('┤'),
        b'v' => Some('┴'),
        b'w' => Some('┬'),
        b'x' => Some('│'),
        b'y' => Some('≤'),
        b'z' => Some('≥'),
        b'{' => Some('π'),
        b'|' => Some('≠'),
        b'}' => Some('£'),
        b'~' => Some('·'),
        _ => None,
    }
}

fn first_csi_numeric_parameter(payload: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(payload).ok()?;
    let first = text.split([';', ':']).next().unwrap_or_default();
    let digits = first.chars().filter(char::is_ascii_digit).collect::<String>();
    if digits.is_empty() {
        return None;
    }
    Some(digits.parse::<u16>().ok()?.min(i16::MAX as u16))
}

fn first_two_csi_numeric_parameters(payload: &[u8]) -> (Option<u16>, Option<u16>) {
    let Ok(text) = std::str::from_utf8(payload) else {
        return (None, None);
    };
    let mut parts = text.split([';', ':']);
    (parse_csi_numeric_part(parts.next()), parse_csi_numeric_part(parts.next()))
}

fn parse_csi_numeric_part(part: Option<&str>) -> Option<u16> {
    let digits = part?.chars().filter(char::is_ascii_digit).collect::<String>();
    if digits.is_empty() {
        return None;
    }
    Some(digits.parse::<u16>().ok()?.min(i16::MAX as u16))
}

fn csi_has_space_intermediate(payload: &[u8]) -> bool {
    payload.contains(&b' ')
}

fn csi_has_bang_intermediate(payload: &[u8]) -> bool {
    payload.contains(&b'!')
}

fn csi_has_dollar_intermediate(payload: &[u8]) -> bool {
    payload.contains(&b'$')
}

fn csi_has_apostrophe_intermediate(payload: &[u8]) -> bool {
    payload.contains(&b'\'')
}

fn csi_has_quote_intermediate(payload: &[u8]) -> bool {
    payload.contains(&b'"')
}

fn selective_csi_payload(payload: &[u8]) -> (bool, &[u8]) {
    match payload.strip_prefix(b"?") {
        Some(payload) => (true, payload),
        None => (false, payload),
    }
}

fn csi_modes(payload: &[u8]) -> Vec<u16> {
    let Ok(text) = std::str::from_utf8(payload) else {
        return Vec::new();
    };
    if text.starts_with('?') {
        return Vec::new();
    }
    text.split(';').filter_map(|part| parse_csi_numeric_part(Some(part))).collect()
}

fn csi_private_modes(payload: &[u8]) -> Vec<u16> {
    let Ok(text) = std::str::from_utf8(payload) else {
        return Vec::new();
    };
    let Some(private_modes) = text.strip_prefix('?') else {
        return Vec::new();
    };
    private_modes.split(';').filter_map(|part| parse_csi_numeric_part(Some(part))).collect()
}

fn cursor_shape_from_decscusr(code: u16) -> Option<(ScreenCursorShape, bool)> {
    match code {
        0 | 1 => Some((ScreenCursorShape::Block, true)),
        2 => Some((ScreenCursorShape::Block, false)),
        3 => Some((ScreenCursorShape::Underline, true)),
        4 => Some((ScreenCursorShape::Underline, false)),
        5 => Some((ScreenCursorShape::Beam, true)),
        6 => Some((ScreenCursorShape::Beam, false)),
        _ => None,
    }
}

fn read_csi(bytes: &[u8], mut index: usize) -> Option<(&[u8], u8, usize)> {
    let start = index;
    while index < bytes.len() && index - start <= MAX_CSI_SEQUENCE_BYTES {
        let byte = bytes[index];
        if (0x40..=0x7e).contains(&byte) {
            return Some((&bytes[start..index], byte, index + 1));
        }
        index += 1;
    }
    None
}

#[derive(Debug)]
struct ControlString<'a> {
    payload: &'a [u8],
    next_index: usize,
    truncated: bool,
}

fn read_osc_control_string(bytes: &[u8], index: usize) -> Option<ControlString<'_>> {
    read_control_string(bytes, index, true)
}

fn read_st_control_string(bytes: &[u8], index: usize) -> Option<ControlString<'_>> {
    read_control_string(bytes, index, false)
}

fn read_control_string(
    bytes: &[u8],
    mut index: usize,
    allow_bel_terminator: bool,
) -> Option<ControlString<'_>> {
    let start = index;
    let mut payload_end = index;
    let mut truncated = false;
    while index < bytes.len() {
        if let Some((C1Control::St, control_len)) = c1_control_at(bytes, index) {
            return Some(ControlString {
                payload: &bytes[start..payload_end],
                next_index: index + control_len,
                truncated,
            });
        }
        match bytes[index] {
            0x07 if allow_bel_terminator => {
                return Some(ControlString {
                    payload: &bytes[start..payload_end],
                    next_index: index + 1,
                    truncated,
                });
            }
            0x9c => {
                return Some(ControlString {
                    payload: &bytes[start..payload_end],
                    next_index: index + 1,
                    truncated,
                });
            }
            0x1b if bytes.get(index + 1) == Some(&b'\\') => {
                return Some(ControlString {
                    payload: &bytes[start..payload_end],
                    next_index: index + 2,
                    truncated,
                });
            }
            _ => {
                index += 1;
                if payload_end - start < MAX_CONTROL_STRING_BYTES {
                    payload_end = index;
                } else {
                    truncated = true;
                }
            }
        }
    }
    None
}

fn skip_control_string(bytes: &[u8], index: usize) -> usize {
    read_st_control_string(bytes, index).map(|control| control.next_index).unwrap_or(bytes.len())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum C1Control {
    Index,
    NextLine,
    ReverseIndex,
    HorizontalTabSet,
    SingleShiftTwo,
    SingleShiftThree,
    StartGuardedArea,
    EndGuardedArea,
    ReturnTerminalId,
    Sos,
    Dcs,
    Csi,
    St,
    Osc,
    Pm,
    Apc,
}

fn raw_c1_control(byte: u8) -> Option<char> {
    match byte {
        0x84 | 0x85 | 0x88 | 0x8d | 0x8e | 0x8f | 0x96 | 0x97 | 0x98 | 0x90 | 0x9a | 0x9b
        | 0x9c | 0x9d | 0x9e | 0x9f => Some(char::from_u32(u32::from(byte))?),
        _ => None,
    }
}

fn c1_control_at(bytes: &[u8], index: usize) -> Option<(C1Control, usize)> {
    let rest = bytes.get(index..)?;
    [
        (C1Control::Index, "\u{0084}"),
        (C1Control::NextLine, "\u{0085}"),
        (C1Control::HorizontalTabSet, "\u{0088}"),
        (C1Control::ReverseIndex, "\u{008d}"),
        (C1Control::SingleShiftTwo, "\u{008e}"),
        (C1Control::SingleShiftThree, "\u{008f}"),
        (C1Control::StartGuardedArea, "\u{0096}"),
        (C1Control::EndGuardedArea, "\u{0097}"),
        (C1Control::Sos, "\u{0098}"),
        (C1Control::ReturnTerminalId, "\u{009a}"),
        (C1Control::Dcs, "\u{0090}"),
        (C1Control::Csi, "\u{009b}"),
        (C1Control::St, "\u{009c}"),
        (C1Control::Osc, "\u{009d}"),
        (C1Control::Pm, "\u{009e}"),
        (C1Control::Apc, "\u{009f}"),
    ]
    .into_iter()
    .find_map(|(control, encoded)| {
        rest.starts_with(encoded.as_bytes()).then_some((control, encoded.len()))
    })
}

fn parse_osc8_hyperlink(payload: &[u8]) -> Option<Option<String>> {
    let text = std::str::from_utf8(payload).ok()?;
    let mut parts = text.splitn(3, ';');
    if parts.next()? != "8" {
        return None;
    }
    let _params = parts.next()?;
    let uri = parts.next().unwrap_or_default().trim();
    if uri.is_empty() {
        return Some(None);
    }
    if uri.chars().any(char::is_control) {
        return Some(None);
    }
    Some(Some(uri.chars().take(2048).collect()))
}

fn terminal_title_from_osc_payload(payload: &[u8]) -> Option<String> {
    let title = payload
        .strip_prefix(b"0;")
        .or_else(|| payload.strip_prefix(b"1;"))
        .or_else(|| payload.strip_prefix(b"2;"))?;
    terminal_metadata_string(title, 4096)
}

fn terminal_cursor_shape_from_osc_payload(payload: &[u8]) -> Option<ScreenCursorShape> {
    match payload.strip_prefix(b"1337;CursorShape=")? {
        b"0" => Some(ScreenCursorShape::Block),
        b"1" => Some(ScreenCursorShape::Beam),
        b"2" => Some(ScreenCursorShape::Underline),
        _ => None,
    }
}

fn terminal_working_directory_uri_from_osc_payload(payload: &[u8]) -> Option<Option<String>> {
    if let Some(working_directory) = payload.strip_prefix(b"7;") {
        return terminal_working_directory_uri(working_directory);
    }
    if let Some(working_directory) = payload.strip_prefix(b"9;9;") {
        return terminal_working_directory_uri(working_directory);
    }
    if let Some(working_directory) = terminal_vscode_working_directory(payload) {
        return terminal_working_directory_uri(working_directory);
    }
    if let Some(working_directory) = payload.strip_prefix(b"1337;CurrentDir=") {
        return terminal_working_directory_uri(working_directory);
    }
    None
}

fn terminal_vscode_working_directory(payload: &[u8]) -> Option<&[u8]> {
    let body = payload.strip_prefix(b"633;P;")?;
    body.split(|byte| *byte == b';').find_map(|property| property.strip_prefix(b"Cwd="))
}

fn terminal_working_directory_uri(value: &[u8]) -> Option<Option<String>> {
    let working_directory = terminal_metadata_text(value)?;
    if working_directory.is_empty() {
        return Some(None);
    }
    if working_directory.starts_with("file://") {
        return Some(Some(working_directory.to_string()));
    }
    terminal_working_directory_path_to_uri(working_directory).map(Some)
}

fn terminal_working_directory_path_to_uri(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//") {
        let (host, path) = rest.split_once('/')?;
        if host.is_empty() || path.is_empty() || host.chars().any(char::is_control) {
            return None;
        }
        return Some(format!("file://{host}/{}", percent_encode_file_path(path)));
    }
    if is_windows_drive_path(&normalized) {
        return Some(format!("file:///{}", percent_encode_file_path(&normalized)));
    }
    if normalized.starts_with('/') {
        return Some(format!("file://localhost{}", percent_encode_file_path(&normalized)));
    }
    None
}

fn is_windows_drive_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

fn percent_encode_file_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for &byte in path.as_bytes() {
        if is_file_uri_path_byte(byte) {
            encoded.push(char::from(byte));
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

fn is_file_uri_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'/' | b':'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b'@'
        )
}

fn terminal_user_variable_from_osc(payload: &[u8]) -> Option<(String, String)> {
    let body = payload.strip_prefix(b"1337;SetUserVar=")?;
    let (key, encoded_value) = split_once_byte(body, b'=')?;
    let key = terminal_user_variable_key(key)?;
    if estimated_base64_decoded_len(encoded_value) > 4096 {
        return None;
    }
    let decoded = BASE64_STANDARD.decode(encoded_value).ok()?;
    let value = std::str::from_utf8(&decoded).ok()?;
    if value.chars().any(char::is_control) {
        return None;
    }
    Some((key, value.chars().take(4096).collect()))
}

fn terminal_user_variable_key(value: &[u8]) -> Option<String> {
    let key = std::str::from_utf8(value).ok()?.trim();
    if key.is_empty() || key.len() > 96 {
        return None;
    }
    key.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .then(|| key.to_string())
}

fn terminal_progress_from_osc(payload: &[u8]) -> Option<ScreenProgress> {
    if payload == b"9;4" {
        return Some(ScreenProgress::default());
    }
    let body = payload.strip_prefix(b"9;4;")?;
    let mut parts = body.split(|byte| *byte == b';');
    let state = parse_terminal_progress_state(parts.next()?)?;
    let value = match state {
        ScreenProgressState::Inactive | ScreenProgressState::Indeterminate => None,
        ScreenProgressState::Normal | ScreenProgressState::Error | ScreenProgressState::Warning => {
            parts.next().and_then(parse_terminal_progress_value)
        }
    };
    Some(ScreenProgress { state, value })
}

fn parse_terminal_progress_state(value: &[u8]) -> Option<ScreenProgressState> {
    match value {
        b"0" => Some(ScreenProgressState::Inactive),
        b"1" => Some(ScreenProgressState::Normal),
        b"2" => Some(ScreenProgressState::Error),
        b"3" => Some(ScreenProgressState::Indeterminate),
        b"4" => Some(ScreenProgressState::Warning),
        _ => None,
    }
}

fn parse_terminal_progress_value(value: &[u8]) -> Option<u8> {
    if value.is_empty() || !value.iter().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let parsed = std::str::from_utf8(value).ok()?.parse::<u16>().ok()?;
    Some(parsed.min(100) as u8)
}

fn apply_surface_palette_osc(palette: &mut ScreenSurfacePalette, payload: &[u8]) {
    if let Some(operations) = parse_terminal_kitty_color_control(payload) {
        for operation in operations {
            match operation {
                TerminalKittyColorControlOperation::Update(target, color) => {
                    apply_surface_palette_target(palette, target, color);
                }
                TerminalKittyColorControlOperation::Reset(TerminalPaletteTarget::Foreground) => {
                    palette.foreground = None;
                }
                TerminalKittyColorControlOperation::Reset(TerminalPaletteTarget::Background) => {
                    palette.background = None;
                }
                TerminalKittyColorControlOperation::Reset(TerminalPaletteTarget::Cursor) => {
                    palette.cursor = None;
                }
                TerminalKittyColorControlOperation::Reset(TerminalPaletteTarget::Ansi(_))
                | TerminalKittyColorControlOperation::QueryKnown { .. }
                | TerminalKittyColorControlOperation::QueryUnknown { .. } => {}
            }
        }
        return;
    }

    if let Some((target, color)) = parse_iterm2_set_colors_update(payload) {
        apply_surface_palette_target(palette, target, color);
        return;
    }

    if let Some((target, color)) = parse_terminal_osc_p_palette_update(payload) {
        match target {
            TerminalPaletteTarget::Foreground => palette.foreground = Some(color),
            TerminalPaletteTarget::Background => palette.background = Some(color),
            TerminalPaletteTarget::Cursor => palette.cursor = Some(color),
            TerminalPaletteTarget::Ansi(_) => {}
        }
        return;
    }

    let fields = payload.split(|byte| *byte == b';').collect::<Vec<_>>();
    let Some(command) = fields.first().copied() else {
        return;
    };
    match command {
        b"110" => {
            palette.foreground = None;
            return;
        }
        b"111" => {
            palette.background = None;
            return;
        }
        b"112" => {
            palette.cursor = None;
            return;
        }
        b"4" => {
            apply_surface_palette_osc4_extended_slots(palette, &fields[1..]);
            return;
        }
        b"104" if fields.len() == 1 => {
            *palette = ScreenSurfacePalette::default();
            return;
        }
        b"104" => {
            reset_surface_palette_osc4_extended_slots(palette, &fields[1..]);
            return;
        }
        _ => {}
    }
    let Some(mut target) = terminal_default_palette_target_from_osc_code(command) else {
        return;
    };
    let mut index = 1;
    while index < fields.len() {
        if index + 1 < fields.len()
            && let Some(explicit_target) =
                terminal_default_palette_target_from_osc_code(fields[index])
        {
            target = explicit_target;
            index += 1;
        }

        if let Some(color) = terminal_surface_color_spec(fields[index]) {
            apply_surface_palette_target(palette, target, color);
        }
        let Some(next_target) = next_terminal_default_palette_target(target) else {
            break;
        };
        target = next_target;
        index += 1;
    }
}

fn apply_surface_palette_target(
    palette: &mut ScreenSurfacePalette,
    target: TerminalPaletteTarget,
    color: ScreenColor,
) {
    match target {
        TerminalPaletteTarget::Foreground => palette.foreground = Some(color),
        TerminalPaletteTarget::Background => palette.background = Some(color),
        TerminalPaletteTarget::Cursor => palette.cursor = Some(color),
        TerminalPaletteTarget::Ansi(_) => {}
    }
}

fn apply_surface_palette_osc4_extended_slots(palette: &mut ScreenSurfacePalette, fields: &[&[u8]]) {
    for pair in fields.chunks_exact(2) {
        let Some(slot) = terminal_surface_palette_osc4_default_slot(pair[0]) else {
            continue;
        };
        let Some(color) = terminal_surface_color_spec(pair[1]) else {
            continue;
        };
        match slot {
            256 => palette.foreground = Some(color),
            257 => palette.background = Some(color),
            258 => palette.cursor = Some(color),
            _ => {}
        }
    }
}

fn reset_surface_palette_osc4_extended_slots(palette: &mut ScreenSurfacePalette, fields: &[&[u8]]) {
    for field in fields {
        match terminal_surface_palette_osc4_default_slot(field) {
            Some(256) => palette.foreground = None,
            Some(257) => palette.background = None,
            Some(258) => palette.cursor = None,
            _ => {}
        }
    }
}

fn terminal_surface_palette_osc4_default_slot(value: &[u8]) -> Option<usize> {
    let index = terminal_osc4_palette_index(value)?;
    (256..=258).contains(&index).then_some(index)
}

fn apply_dynamic_palette_osc(palette: &mut BTreeMap<u8, ScreenColor>, payload: &[u8]) {
    if let Some(operations) = parse_terminal_kitty_color_control(payload) {
        for operation in operations {
            match operation {
                TerminalKittyColorControlOperation::Update(
                    TerminalPaletteTarget::Ansi(index),
                    color,
                ) => {
                    palette.insert(index, color);
                }
                TerminalKittyColorControlOperation::Reset(TerminalPaletteTarget::Ansi(index)) => {
                    palette.remove(&index);
                }
                TerminalKittyColorControlOperation::Update(_, _)
                | TerminalKittyColorControlOperation::Reset(_)
                | TerminalKittyColorControlOperation::QueryKnown { .. }
                | TerminalKittyColorControlOperation::QueryUnknown { .. } => {}
            }
        }
        return;
    }

    if let Some((TerminalPaletteTarget::Ansi(index), color)) =
        parse_iterm2_set_colors_update(payload)
    {
        palette.insert(index, color);
        return;
    }

    if let Some((index, color)) = parse_legacy_linux_console_palette_update(payload) {
        palette.insert(index, color);
        return;
    }
    if is_legacy_linux_console_palette_reset(payload) {
        for index in 0..16 {
            palette.remove(&index);
        }
        return;
    }

    let fields = payload.split(|byte| *byte == b';').collect::<Vec<_>>();
    let Some(command) = fields.first().copied() else {
        return;
    };
    match command {
        b"4" => {
            for pair in fields[1..].chunks_exact(2) {
                let Some(index) = terminal_palette_index(pair[0]) else {
                    continue;
                };
                let Some(color) = terminal_surface_color_spec(pair[1]) else {
                    continue;
                };
                palette.insert(index, color);
            }
        }
        b"104" if fields.len() == 1 => {
            palette.clear();
        }
        b"104" => {
            for index in &fields[1..] {
                if let Some(index) = terminal_palette_index(index) {
                    palette.remove(&index);
                }
            }
        }
        _ => {}
    }
}

fn terminal_palette_index(value: &[u8]) -> Option<u8> {
    let parsed = terminal_color_number(value)?;
    (parsed <= usize::from(u8::MAX)).then_some(parsed as u8)
}

fn terminal_color_number(value: &[u8]) -> Option<usize> {
    if value.is_empty() || !value.iter().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    std::str::from_utf8(value).ok()?.parse::<usize>().ok()
}

fn terminal_osc4_palette_index(value: &[u8]) -> Option<usize> {
    match value {
        b"-1" => Some(256),
        b"-2" => Some(257),
        _ => terminal_color_number(value),
    }
}

fn terminal_surface_color_spec(spec: &[u8]) -> Option<ScreenColor> {
    let text = terminal_metadata_text(spec)?;
    parse_terminal_color_spec(text)
}

fn terminal_media_from_osc_payload(payload: &[u8], truncated: bool) -> Option<ScreenLineMedia> {
    let file_payload = payload.strip_prefix(b"1337;File=")?;
    Some(terminal_iterm2_file_media(file_payload, truncated))
}

fn terminal_iterm2_multipart_file_media(
    multipart: &mut Option<TerminalIterm2MultipartFile>,
    payload: &[u8],
    truncated: bool,
) -> Option<ScreenLineMedia> {
    if let Some(arguments) = payload.strip_prefix(b"1337;MultipartFile=") {
        *multipart = Some(TerminalIterm2MultipartFile::new(arguments, truncated));
        return None;
    }

    if let Some(data) = payload.strip_prefix(b"1337;FilePart=") {
        if let Some(multipart) = multipart.as_mut() {
            multipart.push_part(data, truncated);
        }
        return None;
    }

    if payload == b"1337;FileEnd" {
        return multipart.take().map(|multipart| multipart.finish(truncated));
    }

    None
}

fn terminal_media_from_dcs_payload(payload: &[u8], truncated: bool) -> Option<ScreenLineMedia> {
    let final_byte = dcs_header_final_byte(payload)?;
    if final_byte != b'q' {
        return None;
    }
    let mut media = ScreenLineMedia::marker(ScreenLineMediaKind::Sixel);
    media.truncated = truncated;
    Some(media)
}

fn terminal_media_from_apc_payload(
    kitty_chunk: &mut Option<TerminalKittyGraphicsChunk>,
    payload: &[u8],
    truncated: bool,
) -> Option<ScreenLineMedia> {
    let kitty_payload = payload.strip_prefix(b"G")?;
    if terminal_kitty_graphics_is_query(kitty_payload, truncated) {
        *kitty_chunk = None;
        return None;
    }
    terminal_kitty_graphics_media(kitty_chunk, kitty_payload, truncated)
}

fn terminal_kitty_graphics_media(
    kitty_chunk: &mut Option<TerminalKittyGraphicsChunk>,
    payload: &[u8],
    truncated: bool,
) -> Option<ScreenLineMedia> {
    let item = terminal_kitty_graphics_payload(payload, truncated);

    if item.more_chunks {
        if let Some(chunk) = kitty_chunk.as_mut()
            && item.is_continuation_candidate()
        {
            chunk.merge_continuation(item);
            return None;
        }

        if item.is_inline_png_candidate() {
            *kitty_chunk = Some(TerminalKittyGraphicsChunk::new(item));
            return None;
        }

        *kitty_chunk = None;
        return Some(item.media);
    }

    if let Some(mut chunk) = kitty_chunk.take()
        && item.is_continuation_candidate()
    {
        chunk.merge_final(item);
        return Some(chunk.finish());
    }

    Some(terminal_kitty_graphics_media_from_payload(item))
}

fn terminal_tmux_passthrough_payload(payload: &[u8], truncated: bool) -> Option<Vec<u8>> {
    if truncated {
        return None;
    }
    let body = payload.strip_prefix(b"tmux;")?;
    let mut decoded = Vec::with_capacity(body.len());
    let mut index = 0;
    while index < body.len() {
        if body[index] == 0x1b && body.get(index + 1) == Some(&0x1b) {
            decoded.push(0x1b);
            index += 2;
        } else {
            decoded.push(body[index]);
            index += 1;
        }
    }
    Some(decoded)
}

fn dcs_header_final_byte(payload: &[u8]) -> Option<u8> {
    payload.iter().copied().find(|byte| (0x40..=0x7e).contains(byte))
}

fn terminal_iterm2_file_media(payload: &[u8], truncated: bool) -> ScreenLineMedia {
    let (arguments, data) = split_once_byte(payload, b':').unwrap_or((payload, &[]));
    let mut media = ScreenLineMedia::marker(ScreenLineMediaKind::Iterm2Image);
    media.truncated = truncated;

    for argument in arguments.split(|byte| *byte == b';') {
        let Some((key, value)) = split_once_byte(argument, b'=') else {
            continue;
        };
        match key {
            b"name" => {
                media.name = terminal_iterm2_file_name(value);
            }
            b"size" => {
                media.byte_size = terminal_u32_parameter(value);
            }
            b"width" => {
                media.width = terminal_media_text_parameter(value, 40);
            }
            b"height" => {
                media.height = terminal_media_text_parameter(value, 40);
            }
            b"preserveAspectRatio" => {
                media.preserve_aspect_ratio = terminal_bool_parameter(value);
            }
            b"inline" => {
                media.inline = value == b"1";
            }
            _ => {}
        }
    }

    if media.inline
        && !truncated
        && !data.is_empty()
        && estimated_base64_decoded_len(data) <= MAX_INLINE_IMAGE_BYTES
        && let Some((mime_type, data_base64)) = terminal_inline_image_payload(data)
    {
        media.mime_type = Some(mime_type.to_string());
        media.data_base64 = Some(data_base64);
    }

    media
}

#[derive(Debug)]
struct TerminalKittyGraphicsPayload {
    media: ScreenLineMedia,
    data: Vec<u8>,
    has_format: bool,
    is_png_format: bool,
    more_chunks: bool,
    compressed: bool,
    truncated: bool,
}

impl TerminalKittyGraphicsPayload {
    fn is_inline_png_candidate(&self) -> bool {
        self.is_png_format
            && !self.compressed
            && !self.truncated
            && !self.data.is_empty()
            && estimated_base64_decoded_len(&self.data) <= MAX_INLINE_IMAGE_BYTES
    }

    fn is_continuation_candidate(&self) -> bool {
        !self.compressed
            && !self.truncated
            && !self.data.is_empty()
            && (!self.has_format || self.is_png_format)
    }
}

#[derive(Debug)]
struct TerminalKittyGraphicsChunk {
    media: ScreenLineMedia,
    data: Vec<u8>,
    truncated: bool,
}

impl TerminalKittyGraphicsChunk {
    fn new(payload: TerminalKittyGraphicsPayload) -> Self {
        Self { media: payload.media, data: payload.data, truncated: payload.truncated }
    }

    fn push_payload(&mut self, payload: &[u8]) {
        let next_len = self.data.len().saturating_add(payload.len());
        if estimated_base64_decoded_len_for_bytes(next_len) > MAX_INLINE_IMAGE_BYTES {
            self.truncated = true;
            return;
        }
        self.data.extend_from_slice(payload);
    }

    fn merge_final(&mut self, payload: TerminalKittyGraphicsPayload) {
        self.merge_continuation(payload);
    }

    fn merge_continuation(&mut self, payload: TerminalKittyGraphicsPayload) {
        if payload.media.width.is_some() {
            self.media.width = payload.media.width;
        }
        if payload.media.height.is_some() {
            self.media.height = payload.media.height;
        }
        self.truncated |= payload.truncated;
        self.push_payload(&payload.data);
    }

    fn finish(mut self) -> ScreenLineMedia {
        self.media.truncated |= self.truncated;
        if !self.media.truncated
            && !self.data.is_empty()
            && let Some((mime_type, data_base64)) = terminal_inline_image_payload(&self.data)
            && mime_type == "image/png"
        {
            self.media.inline = true;
            self.media.mime_type = Some(mime_type.to_string());
            self.media.data_base64 = Some(data_base64);
        }
        self.media
    }
}

fn terminal_kitty_graphics_payload(
    payload: &[u8],
    truncated: bool,
) -> TerminalKittyGraphicsPayload {
    let (arguments, data) = split_once_byte(payload, b';').unwrap_or((payload, &[]));
    let mut media = ScreenLineMedia::marker(ScreenLineMediaKind::KittyGraphics);
    media.truncated = truncated;

    let mut format = None;
    let mut more_chunks = false;
    let mut compressed = false;
    for argument in arguments.split(|byte| *byte == b',') {
        let Some((key, value)) = split_once_byte(argument, b'=') else {
            continue;
        };
        match key {
            b"f" => format = Some(value),
            b"m" => more_chunks = value == b"1",
            b"o" => compressed = !value.is_empty(),
            b"c" => media.width = terminal_media_text_parameter(value, 40),
            b"r" => media.height = terminal_media_text_parameter(value, 40),
            _ => {}
        }
    }

    TerminalKittyGraphicsPayload {
        media,
        data: data.to_vec(),
        has_format: format.is_some(),
        is_png_format: format == Some(b"100"),
        more_chunks,
        compressed,
        truncated,
    }
}

fn terminal_kitty_graphics_media_from_payload(
    payload: TerminalKittyGraphicsPayload,
) -> ScreenLineMedia {
    if payload.is_inline_png_candidate() {
        return TerminalKittyGraphicsChunk::new(payload).finish();
    }
    let mut media = payload.media;
    media.truncated |= payload.truncated;
    media
}

fn terminal_kitty_graphics_is_query(payload: &[u8], truncated: bool) -> bool {
    if truncated {
        return false;
    }
    let (arguments, _) = split_once_byte(payload, b';').unwrap_or((payload, &[]));
    terminal_kitty_graphics_control_value(arguments, b"a") == Some(b"q".as_slice())
}

fn terminal_kitty_graphics_control_value<'a>(arguments: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    arguments
        .split(|byte| *byte == b',')
        .filter_map(|argument| split_once_byte(argument, b'='))
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

#[derive(Debug)]
struct TerminalIterm2MultipartFile {
    arguments: Vec<u8>,
    data: Vec<u8>,
    truncated: bool,
}

impl TerminalIterm2MultipartFile {
    fn new(arguments: &[u8], truncated: bool) -> Self {
        Self { arguments: arguments.to_vec(), data: Vec::new(), truncated }
    }

    fn push_part(&mut self, data: &[u8], truncated: bool) {
        self.truncated |= truncated;
        let next_len = self.data.len().saturating_add(data.len());
        if estimated_base64_decoded_len_for_bytes(next_len) > MAX_INLINE_IMAGE_BYTES {
            self.truncated = true;
            return;
        }
        self.data.extend_from_slice(data);
    }

    fn finish(self, truncated: bool) -> ScreenLineMedia {
        let mut payload = self.arguments;
        payload.push(b':');
        payload.extend_from_slice(&self.data);
        terminal_iterm2_file_media(&payload, self.truncated || truncated)
    }
}

fn terminal_side_effect_from_osc_payload(payload: &[u8]) -> Option<ScreenLineSideEffect> {
    terminal_clipboard_side_effect_from_osc(payload)
        .or_else(|| terminal_notification_side_effect_from_osc(payload))
}

fn terminal_clipboard_side_effect_from_osc(payload: &[u8]) -> Option<ScreenLineSideEffect> {
    let body = payload.strip_prefix(b"52;")?;
    let (clipboard, data) = split_once_byte(body, b';').unwrap_or((body, &[]));
    let clipboard = clipboard.first().copied().unwrap_or(b'c');
    let kind = if data == b"?" {
        ScreenLineSideEffectKind::ClipboardRead
    } else {
        ScreenLineSideEffectKind::ClipboardWrite
    };
    Some(ScreenLineSideEffect {
        kind,
        disposition: ScreenLineSideEffectDisposition::Blocked,
        target: screen_line_clipboard_target(clipboard),
        message: None,
    })
}

fn terminal_notification_side_effect_from_osc(payload: &[u8]) -> Option<ScreenLineSideEffect> {
    let message = payload.strip_prefix(b"9;")?;
    if message == b"4" || message.starts_with(b"4;") {
        return None;
    }
    let message = terminal_metadata_text(message)?;
    if message.is_empty() {
        return None;
    }
    Some(ScreenLineSideEffect {
        kind: ScreenLineSideEffectKind::DesktopNotification,
        disposition: ScreenLineSideEffectDisposition::Blocked,
        target: Some(ScreenLineSideEffectTarget::DesktopNotification),
        message: Some(truncate_terminal_metadata_message(message, 160)),
    })
}

fn screen_line_clipboard_target(clipboard: u8) -> Option<ScreenLineSideEffectTarget> {
    match clipboard {
        b'c' => Some(ScreenLineSideEffectTarget::Clipboard),
        b'p' | b's' => Some(ScreenLineSideEffectTarget::Selection),
        _ => Some(ScreenLineSideEffectTarget::Unknown),
    }
}

fn terminal_shell_integration_mark(payload: &[u8]) -> Option<ScreenLineSemanticMark> {
    let body = payload.strip_prefix(b"133;").or_else(|| payload.strip_prefix(b"633;"))?;
    let (command, params) = match body.split_first() {
        Some((command, params)) => (*command, params),
        None => return None,
    };

    let kind = match command {
        b'A' => ScreenLineSemanticMarkKind::PromptStart,
        b'B' => ScreenLineSemanticMarkKind::InputStart,
        b'C' => ScreenLineSemanticMarkKind::OutputStart,
        b'D' => ScreenLineSemanticMarkKind::CommandFinished,
        _ => return None,
    };
    let exit_code = (kind == ScreenLineSemanticMarkKind::CommandFinished)
        .then(|| terminal_shell_integration_exit_code(params))
        .flatten();

    Some(ScreenLineSemanticMark { kind, col: 0, exit_code })
}

fn terminal_shell_integration_exit_code(params: &[u8]) -> Option<u8> {
    let value = params.strip_prefix(b";")?.split(|byte| *byte == b';').next()?;
    if value.is_empty() || !value.iter().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    std::str::from_utf8(value).ok()?.parse::<u8>().ok()
}

fn terminal_metadata_text(value: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(value).ok()?.trim();
    let text = strip_matching_quotes(text);
    (!text.chars().any(char::is_control)).then_some(text)
}

fn terminal_metadata_string(value: &[u8], max_chars: usize) -> Option<String> {
    let text = terminal_metadata_text(value)?;
    (!text.is_empty()).then(|| text.chars().take(max_chars).collect())
}

fn strip_matching_quotes(value: &str) -> &str {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return value;
    };
    let Some(last) = value.chars().next_back() else {
        return value;
    };
    if value.len() >= 2 && ((first == '"' && last == '"') || (first == '\'' && last == '\'')) {
        let start = first.len_utf8();
        let end = value.len() - last.len_utf8();
        &value[start..end]
    } else {
        value
    }
}

fn truncate_terminal_metadata_message(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{truncated}...") } else { truncated }
}

fn split_once_byte(value: &[u8], separator: u8) -> Option<(&[u8], &[u8])> {
    let index = value.iter().position(|byte| *byte == separator)?;
    Some((&value[..index], &value[index + 1..]))
}

fn terminal_iterm2_file_name(value: &[u8]) -> Option<String> {
    if estimated_base64_decoded_len(value) > 512 {
        return None;
    }
    let decoded = BASE64_STANDARD.decode(value).ok()?;
    let name = std::str::from_utf8(&decoded).ok()?.trim();
    terminal_media_string(name, 120)
}

fn terminal_u32_parameter(value: &[u8]) -> Option<u32> {
    if value.is_empty() || !value.iter().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    std::str::from_utf8(value).ok()?.parse::<u32>().ok()
}

fn terminal_bool_parameter(value: &[u8]) -> Option<bool> {
    match value {
        b"0" => Some(false),
        b"1" => Some(true),
        _ => None,
    }
}

fn terminal_media_text_parameter(value: &[u8], max_chars: usize) -> Option<String> {
    let value = std::str::from_utf8(value).ok()?.trim();
    terminal_media_string(value, max_chars)
}

fn terminal_media_string(value: &str, max_chars: usize) -> Option<String> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

fn estimated_base64_decoded_len(value: &[u8]) -> usize {
    estimated_base64_decoded_len_for_bytes(value.len())
}

fn estimated_base64_decoded_len_for_bytes(bytes: usize) -> usize {
    bytes.saturating_mul(3) / 4
}

fn terminal_inline_image_payload(value: &[u8]) -> Option<(&'static str, String)> {
    let data_base64 = std::str::from_utf8(value).ok()?.trim();
    if data_base64
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')))
    {
        return None;
    }
    let decoded = BASE64_STANDARD.decode(data_base64).ok()?;
    let mime_type = terminal_image_mime_type(&decoded)?;
    Some((mime_type, data_base64.to_string()))
}

fn terminal_image_mime_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn apply_sgr_payload(
    style: &mut ScreenTextStyle,
    payload: &[u8],
    dynamic_palette: &BTreeMap<u8, ScreenColor>,
) {
    let Ok(payload) = std::str::from_utf8(payload) else {
        return;
    };
    if payload.is_empty() {
        reset_sgr_preserving_hyperlink(style);
        return;
    }

    let parts = payload.split(';').collect::<Vec<_>>();
    let mut index = 0;
    while index < parts.len() {
        let part = parts[index];
        if part.is_empty() {
            reset_sgr_preserving_hyperlink(style);
            index += 1;
            continue;
        }
        if apply_colon_sgr(style, part, dynamic_palette) {
            index += 1;
            continue;
        }
        let code = part.split(':').next().unwrap_or_default();
        match code.parse::<u16>() {
            Ok(0) => reset_sgr_preserving_hyperlink(style),
            Ok(1) => style.bold = true,
            Ok(2) => style.dim = true,
            Ok(3) => style.italic = true,
            Ok(4) => style.underline = Some(ScreenUnderlineStyle::Single),
            Ok(5) | Ok(6) => style.blink = true,
            Ok(7) => style.inverse = true,
            Ok(8) => style.hidden = true,
            Ok(9) => style.strikethrough = true,
            Ok(21) => style.underline = Some(ScreenUnderlineStyle::Double),
            Ok(22) => {
                style.bold = false;
                style.dim = false;
            }
            Ok(23) => style.italic = false,
            Ok(24) => style.underline = None,
            Ok(25) => style.blink = false,
            Ok(27) => style.inverse = false,
            Ok(28) => style.hidden = false,
            Ok(29) => style.strikethrough = false,
            Ok(value @ 30..=37) => style.foreground = named_sgr_color(value - 30, dynamic_palette),
            Ok(39) => style.foreground = None,
            Ok(value @ 40..=47) => style.background = named_sgr_color(value - 40, dynamic_palette),
            Ok(49) => style.background = None,
            Ok(51) => style.border = Some(ScreenTextBorderStyle::Framed),
            Ok(52) => style.border = Some(ScreenTextBorderStyle::Encircled),
            Ok(53) => style.overline = true,
            Ok(54) => style.border = None,
            Ok(55) => style.overline = false,
            Ok(58) => {
                if let Some((color, consumed)) =
                    parse_semicolon_sgr_color_fields(&parts[index + 1..])
                {
                    style.underline_color =
                        Some(resolve_dynamic_palette_color(color, dynamic_palette));
                    index += consumed;
                }
            }
            Ok(59) => style.underline_color = None,
            Ok(73) => style.baseline = Some(ScreenTextBaseline::Superscript),
            Ok(74) => style.baseline = Some(ScreenTextBaseline::Subscript),
            Ok(75) => style.baseline = None,
            Ok(value @ 90..=97) => {
                style.foreground = named_sgr_color(value - 90 + 8, dynamic_palette)
            }
            Ok(value @ 100..=107) => {
                style.background = named_sgr_color(value - 100 + 8, dynamic_palette)
            }
            Ok(38) => {
                if let Some((color, consumed)) =
                    parse_semicolon_sgr_color_fields(&parts[index + 1..])
                {
                    style.foreground = Some(resolve_dynamic_palette_color(color, dynamic_palette));
                    index += consumed;
                }
            }
            Ok(48) => {
                if let Some((color, consumed)) =
                    parse_semicolon_sgr_color_fields(&parts[index + 1..])
                {
                    style.background = Some(resolve_dynamic_palette_color(color, dynamic_palette));
                    index += consumed;
                }
            }
            _ => {}
        }
        index += 1;
    }
}

fn reset_sgr_preserving_hyperlink(style: &mut ScreenTextStyle) {
    let hyperlink = style.hyperlink.clone();
    *style = ScreenTextStyle::default();
    style.hyperlink = hyperlink;
}

fn restore_sgr_stack_attributes(
    target: &mut ScreenTextStyle,
    saved: &ScreenTextStyle,
    attributes: AnsiSgrStackAttributes,
) {
    if attributes.foreground {
        target.foreground = saved.foreground.clone();
    }
    if attributes.background {
        target.background = saved.background.clone();
    }
    if attributes.bold {
        target.bold = saved.bold;
    }
    if attributes.dim {
        target.dim = saved.dim;
    }
    if attributes.italic {
        target.italic = saved.italic;
    }
    if attributes.underline {
        target.underline = saved.underline;
        target.underline_color = saved.underline_color.clone();
    }
    if attributes.blink {
        target.blink = saved.blink;
    }
    if attributes.inverse {
        target.inverse = saved.inverse;
    }
    if attributes.hidden {
        target.hidden = saved.hidden;
    }
    if attributes.strikethrough {
        target.strikethrough = saved.strikethrough;
    }
    if attributes.overline {
        target.overline = saved.overline;
    }
    if attributes.border {
        target.border = saved.border;
    }
    if attributes.baseline {
        target.baseline = saved.baseline;
    }
}

fn apply_colon_sgr(
    style: &mut ScreenTextStyle,
    part: &str,
    dynamic_palette: &BTreeMap<u8, ScreenColor>,
) -> bool {
    let fields = part.split(':').collect::<Vec<_>>();
    let Some(code) = fields.first().and_then(|value| value.parse::<u16>().ok()) else {
        return false;
    };
    match code {
        4 if fields.len() == 2 || (fields.len() == 3 && fields.get(1) == Some(&"")) => {
            let Some(underline) = parse_colon_sgr_underline_style(part) else {
                return false;
            };
            style.underline = underline;
            true
        }
        38 | 48 | 58 => {
            let Some(color) = parse_colon_sgr_color_fields(&fields[1..]) else {
                return false;
            };
            match code {
                38 => {
                    style.foreground = Some(resolve_dynamic_palette_color(color, dynamic_palette))
                }
                48 => {
                    style.background = Some(resolve_dynamic_palette_color(color, dynamic_palette))
                }
                58 => {
                    style.underline_color =
                        Some(resolve_dynamic_palette_color(color, dynamic_palette))
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}

fn named_sgr_color(index: u16, dynamic_palette: &BTreeMap<u8, ScreenColor>) -> Option<ScreenColor> {
    const NAMES: [&str; 16] = [
        "black",
        "red",
        "green",
        "yellow",
        "blue",
        "magenta",
        "cyan",
        "white",
        "bright_black",
        "bright_red",
        "bright_green",
        "bright_yellow",
        "bright_blue",
        "bright_magenta",
        "bright_cyan",
        "bright_white",
    ];
    let index = u8::try_from(index).ok()?;
    dynamic_palette
        .get(&index)
        .cloned()
        .or_else(|| Some(ScreenColor::Named { name: NAMES.get(usize::from(index))?.to_string() }))
}

fn resolve_dynamic_palette_color(
    color: ScreenColor,
    dynamic_palette: &BTreeMap<u8, ScreenColor>,
) -> ScreenColor {
    match color {
        ScreenColor::Indexed { index } => {
            dynamic_palette.get(&index).cloned().unwrap_or(ScreenColor::Indexed { index })
        }
        ScreenColor::Named { name } => {
            let Some(index) = sgr_named_color_index(&name) else {
                return ScreenColor::Named { name };
            };
            dynamic_palette.get(&index).cloned().unwrap_or(ScreenColor::Named { name })
        }
        color => color,
    }
}

fn sgr_named_color_index(name: &str) -> Option<u8> {
    match name {
        "black" => Some(0),
        "red" => Some(1),
        "green" => Some(2),
        "yellow" => Some(3),
        "blue" => Some(4),
        "magenta" => Some(5),
        "cyan" => Some(6),
        "white" => Some(7),
        "bright_black" => Some(8),
        "bright_red" => Some(9),
        "bright_green" => Some(10),
        "bright_yellow" => Some(11),
        "bright_blue" => Some(12),
        "bright_magenta" => Some(13),
        "bright_cyan" => Some(14),
        "bright_white" => Some(15),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_plain_lines_without_rich_spans_for_plain_output() {
        assert_eq!(
            screen_lines_from_ansi_output("alpha\nbeta"),
            vec![ScreenLine::plain("alpha"), ScreenLine::plain("beta")]
        );
    }

    #[test]
    fn expands_tabs_to_default_terminal_tab_stops() {
        assert_eq!(
            screen_lines_from_ansi_output("a\tb\n12345678\tc"),
            vec![ScreenLine::plain("a       b"), ScreenLine::plain("12345678        c")]
        );
    }

    #[test]
    fn treats_vertical_tab_and_form_feed_as_line_feeds() {
        assert_eq!(
            screen_lines_from_ansi_output("alpha\x0bbeta\x0cgamma"),
            vec![ScreenLine::plain("alpha"), ScreenLine::plain("beta"), ScreenLine::plain("gamma")]
        );
    }

    #[test]
    fn preserves_style_across_expanded_tabs() {
        let lines = screen_lines_from_ansi_output("\x1b[31ma\tb\x1b[0m");

        assert_eq!(lines[0].text, "a       b");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "a       b"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn preserves_basic_and_truecolor_sgr_spans() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[31mred\x1b[0m plain \x1b[38;2;1;2;3;48:2::4:5:6mtrue\x1b[0m",
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "red plain true");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "true"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
                && span.style.background == Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 })
        }));
    }

    #[test]
    fn preserves_rgba_sgr_by_degrading_alpha_to_rgb_spans() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[38:6::12:34:56:128mfg-rgba\x1b[0m \
             \x1b[48:6:1:70:80:90:64mbg-rgba-cs\x1b[0m \
             \x1b[4m\x1b[58:6::9:8:7:255munder-rgba\x1b[0m \
             \x1b[38;6;3;2;1;128mfg-rgba-semi\x1b[0m",
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "fg-rgba bg-rgba-cs under-rgba fg-rgba-semi");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "fg-rgba"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bg-rgba-cs"
                && span.style.background == Some(ScreenColor::Rgb { r: 70, g: 80, b: 90 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "under-rgba"
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
                && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "fg-rgba-semi"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 3, g: 2, b: 1 })
        }));
    }

    #[test]
    fn preserves_cmy_and_cmyk_sgr_colors_as_rgb_spans() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[38:3::0:128:255mfg-cmy\x1b[0m \
             \x1b[48:4::0:128:255:64mbg-cmyk\x1b[0m \
             \x1b[4m\x1b[58:3::255:0:128munder-cmy\x1b[0m \
             \x1b[38;4;0;0;0;128mfg-cmyk-semi\x1b[0m",
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "fg-cmy bg-cmyk under-cmy fg-cmyk-semi");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "fg-cmy"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 127, b: 0 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bg-cmyk"
                && span.style.background == Some(ScreenColor::Rgb { r: 191, g: 95, b: 0 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "under-cmy"
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
                && span.style.underline_color == Some(ScreenColor::Rgb { r: 0, g: 255, b: 127 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "fg-cmyk-semi"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 127, g: 127, b: 127 })
        }));
    }

    #[test]
    fn preserves_semicolon_sgr_color_followed_by_later_attributes() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[38;3;0;128;255;1mcmy-bold\x1b[0m \
             \x1b[58;2;1;2;3;24munder-reset\x1b[0m",
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "cmy-bold under-reset");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "cmy-bold"
                && span.style.bold
                && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 127, b: 0 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "under-reset"
                && span.style.underline.is_none()
                && span.style.underline_color == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
        }));
    }

    #[test]
    fn preserves_semicolon_sgr_colors_with_empty_color_space_slot() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[38;2;;12;34;56mfg\x1b[0m \
             \x1b[48;5;;22mbg\x1b[0m \
             \x1b[4m\x1b[58;6;;9;8;7;128munder\x1b[0m",
        );

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "fg bg under");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "fg"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bg" && span.style.background == Some(ScreenColor::Indexed { index: 22 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "under"
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
                && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn osc4_palette_overrides_named_and_indexed_sgr_colors() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]4;1;rgb:12/34/56;196;#AABBCC\x07\
             \x1b[31mred\x1b[0m \x1b[38;5;196midx\x1b[0m",
        );

        assert_eq!(lines[0].text, "red idx");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x12, g: 0x34, b: 0x56 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "idx"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
        }));
    }

    #[test]
    fn osc4_palette_accepts_rgbi_color_specs() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]4;1;rgbi:1.0/0.5/0.0;196;rgbi:0.0/0.25/1.0\x07\
             \x1b[31mred\x1b[0m \x1b[38;5;196midx\x1b[0m",
        );

        assert_eq!(lines[0].text, "red idx");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "idx"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0, g: 64, b: 255 })
        }));
    }

    #[test]
    fn osc4_palette_accepts_rgba_color_specs() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]4;1;rgba:1212/3434/5656/7878;196;rgba(10, 11, 12, 0.5)\x07\
             \x1b[31mred\x1b[0m \x1b[38;5;196midx\x1b[0m",
        );

        assert_eq!(lines[0].text, "red idx");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "idx"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 10, g: 11, b: 12 })
        }));
    }

    #[test]
    fn osc4_palette_accepts_compact_hex_and_color_space_specs() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]4;1;rgb:abc;2;srgb:102030;196;p3:405060\x07\
             \x1b[31mred\x1b[0m \x1b[32mgreen\x1b[0m \x1b[38;5;196midx\x1b[0m",
        );

        assert_eq!(lines[0].text, "red green idx");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 170, g: 187, b: 204 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "green"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x10, g: 0x20, b: 0x30 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "idx"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x40, g: 0x50, b: 0x60 })
        }));
    }

    #[test]
    fn iterm2_set_colors_updates_dynamic_ansi_palette() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]1337;SetColors=red=00ff00\x07\
             \x1b]1337;SetColors=br_blue=p3:102030\x07\
             \x1b[31mred\x1b[0m \x1b[94mbright-blue\x1b[0m",
        );

        assert_eq!(lines[0].text, "red bright-blue");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0, g: 255, b: 0 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bright-blue"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 16, g: 32, b: 48 })
        }));
    }

    #[test]
    fn kitty_color_control_updates_dynamic_ansi_palette() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]21;1=#112233;12=rgbi:1.0/0.5/0.0\x1b\\\
             \x1b[31mred\x1b[0m \x1b[94mbright-blue\x1b[0m \
             \x1b]21;1\x1b\\\x1b[31mreset-red\x1b[0m",
        );

        assert_eq!(lines[0].text, "red bright-blue reset-red");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bright-blue"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "reset-red"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn kitty_color_stack_restores_surface_and_dynamic_palette() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]21;foreground=#010203;1=#112233\x1b\\\
             \x1b]30001\x1b\\\
             \x1b]21;foreground=#040506;1=#445566\x1b\\\
             \x1b[31mtemporary\x1b[0m \
             \x1b]30101\x1b\\\
             \x1b[31mrestored\x1b[0m",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: None,
                cursor: None,
            }
        );
        let line = surface.lines.first().expect("line should render");
        assert!(line.spans.iter().any(|span| {
            span.text == "temporary"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x44, g: 0x55, b: 0x66 })
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "restored"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
        }));
    }

    #[test]
    fn xterm_color_stack_restores_surface_and_dynamic_palette() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]10;#010203;#111111;#222222\x07\
             \x1b]4;1;#112233\x07\
             \x1b[#P\
             \x1b]10;#040506;#333333;#444444\x07\
             \x1b]4;1;#445566\x07\
             \x1b[31mtemporary\x1b[0m \
             \x1b[#Q\
             \x1b[31mrestored\x1b[0m",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 17, g: 17, b: 17 }),
                cursor: Some(ScreenColor::Rgb { r: 34, g: 34, b: 34 }),
            }
        );
        let line = surface.lines.first().expect("line should render");
        assert_eq!(line.text, "temporary restored");
        assert!(line.spans.iter().any(|span| {
            span.text == "temporary"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x44, g: 0x55, b: 0x66 })
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "restored"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
        }));
    }

    #[test]
    fn xterm_color_stack_addressed_slots_restore_without_popping() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]4;1;#112233\x07\
             \x1b[1#P\
             \x1b]4;1;#445566\x07\
             \x1b[2#P\
             \x1b[1#Q\x1b[31mone\x1b[0m \
             \x1b[2#Q\x1b[31mtwo\x1b[0m \
             \x1b[1#Q\x1b[31mone-again\x1b[0m",
            None,
        );

        let line = surface.lines.first().expect("line should render");
        assert_eq!(line.text, "one two one-again");
        assert!(line.spans.iter().any(|span| {
            span.text == "one"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "two"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x44, g: 0x55, b: 0x66 })
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "one-again"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
        }));
    }

    #[test]
    fn legacy_linux_console_palette_updates_ansi_colors() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]P1aabbcc\x07\x1b[31mred\x1b[0m \
             \x1b]PAddeeff\x07\x1b[92mbright-green\x1b[0m \
             \x1b]R\x07\x1b[31mreset-red\x1b[0m",
        );

        assert_eq!(lines[0].text, "red bright-green reset-red");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bright-green"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0xdd, g: 0xee, b: 0xff })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "reset-red"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn osc104_resets_dynamic_palette_overrides() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]4;1;#010203\x07\x1b[31mcustom\x1b[0m\
             \x1b]104;1\x07\x1b[31mnormal\x1b[0m",
        );

        assert_eq!(lines[0].text, "customnormal");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "custom"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "normal"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn osc4_palette_overrides_colon_sgr_and_underline_color() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]4;2;#102030;5;#405060\x07\
             \x1b[38:5::2mfg\x1b[0m \x1b[58:5::5;4:3munder\x1b[0m",
        );

        assert_eq!(lines[0].text, "fg under");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "fg"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x10, g: 0x20, b: 0x30 })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "under"
                && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                && span.style.underline_color
                    == Some(ScreenColor::Rgb { r: 0x40, g: 0x50, b: 0x60 })
        }));
    }

    #[test]
    fn preserves_extended_sgr_styles_and_resets() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[1;2;3;4:3;9;53;58:2::9:8:7mrich\x1b[22;23;24;29;55;59mplain \
             \x1b[4:99mfallback\x1b[0m \x1b[73msuper\x1b[75mregular \x1b[74msub\x1b[0mreset",
        );

        let rich =
            lines[0].spans.iter().find(|span| span.text == "rich").expect("rich span should exist");
        assert!(rich.style.bold);
        assert!(rich.style.dim);
        assert!(rich.style.italic);
        assert!(rich.style.strikethrough);
        assert!(rich.style.overline);
        assert_eq!(rich.style.underline, Some(ScreenUnderlineStyle::Curly));
        assert_eq!(rich.style.underline_color, Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 }));

        let plain = lines[0]
            .spans
            .iter()
            .find(|span| span.text.starts_with("plain"))
            .expect("plain span should exist");
        assert!(plain.style.is_plain());

        let fallback = lines[0]
            .spans
            .iter()
            .find(|span| span.text == "fallback")
            .expect("fallback underline span should exist");
        assert_eq!(fallback.style.underline, Some(ScreenUnderlineStyle::Single));

        let superscript = lines[0]
            .spans
            .iter()
            .find(|span| span.text == "super")
            .expect("superscript span should exist");
        assert_eq!(superscript.style.baseline, Some(ScreenTextBaseline::Superscript));

        let regular = lines[0]
            .spans
            .iter()
            .find(|span| span.text.starts_with("regular"))
            .expect("regular span should exist");
        assert_eq!(regular.style.baseline, None);

        let subscript = lines[0]
            .spans
            .iter()
            .find(|span| span.text == "sub")
            .expect("subscript span should exist");
        assert_eq!(subscript.style.baseline, Some(ScreenTextBaseline::Subscript));

        let reset = lines[0]
            .spans
            .iter()
            .find(|span| span.text.ends_with("reset"))
            .expect("reset span should exist");
        assert_eq!(reset.style.baseline, None);
    }

    #[test]
    fn preserves_tmux_terminfo_double_colon_sgr_forms() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[4::3;58::2::9::8::7munder\x1b[0m \
             \x1b[38::2::1::2::3mfg\x1b[0m \
             \x1b[48::5::196mbg\x1b[0m",
        );

        let line = &lines[0];
        assert!(
            line.spans.iter().any(|span| {
                span.text == "under"
                    && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            }),
            "double-colon underline span should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
            }),
            "double-colon foreground span should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg"
                    && span.style.background == Some(ScreenColor::Indexed { index: 196 })
            }),
            "double-colon background span should be preserved: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_mixed_terminal_visual_sequence_matrix() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]4;1;rgb:12/34/56;22;#0A0B0C\x07\
             \x1b]10;rgb:ee/ee/ee;11;rgb:00/00/00;12;rgb:01/02/03\x07\
             \x1b[31mred\x1b[0m \
             \x1b[38;5;22midx\x1b[0m \
             \x1b[38:2::1:2:3;48:2::4:5:6mtrue\x1b[0m \
             \x1b[7minverse\x1b[27mplain \
             \x1b]8;;https://example.test/log\x07\x1b[4:3;58:2::9:8:7mlink\x1b]8;;\x07\x1b[0m",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 0xee, g: 0xee, b: 0xee }),
                background: Some(ScreenColor::Rgb { r: 0, g: 0, b: 0 }),
                cursor: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
            }
        );

        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red idx true inverseplain link"))
            .expect("mixed terminal visual sequence line should render");

        assert!(line.spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x12, g: 0x34, b: 0x56 })
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "idx"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 0x0a, g: 0x0b, b: 0x0c })
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "true"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
                && span.style.background == Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 })
        }));
        assert!(line.spans.iter().any(|span| span.text == "inverse" && span.style.inverse));
        assert!(line.spans.iter().any(|span| {
            span.text.starts_with("plain") && !span.style.inverse && span.style.hyperlink.is_none()
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "link"
                && span.style.hyperlink.as_deref() == Some("https://example.test/log")
                && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn preserves_xterm_sgr_stack_for_nested_rich_output() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[31;4:3mouter \x1b[#{\x1b[32;4:5minner\x1b[#} outer2\x1b[0m plain",
        );

        assert_eq!(lines[0].text, "outer inner outer2 plain");

        let outer = lines[0]
            .spans
            .iter()
            .find(|span| span.text == "outer ")
            .expect("outer pushed style span should exist");
        assert_eq!(outer.style.foreground, Some(ScreenColor::Named { name: "red".to_string() }));
        assert_eq!(outer.style.underline, Some(ScreenUnderlineStyle::Curly));

        let inner = lines[0]
            .spans
            .iter()
            .find(|span| span.text == "inner")
            .expect("inner override style span should exist");
        assert_eq!(inner.style.foreground, Some(ScreenColor::Named { name: "green".to_string() }));
        assert_eq!(inner.style.underline, Some(ScreenUnderlineStyle::Dashed));

        let restored = lines[0]
            .spans
            .iter()
            .find(|span| span.text == " outer2")
            .expect("restored style span should exist");
        assert_eq!(restored.style.foreground, Some(ScreenColor::Named { name: "red".to_string() }));
        assert_eq!(restored.style.underline, Some(ScreenUnderlineStyle::Curly));

        let plain = lines[0]
            .spans
            .iter()
            .find(|span| span.text == " plain")
            .expect("reset span should exist");
        assert!(plain.style.is_plain());
    }

    #[test]
    fn preserves_xterm_selective_sgr_stack_attributes() {
        let lines =
            screen_lines_from_ansi_output("\x1b[31mred \x1b[30#{\x1b[32;1mgreen\x1b[#} red2");

        assert_eq!(lines[0].text, "red green red2");

        let restored = lines[0]
            .spans
            .iter()
            .find(|span| span.text == " red2")
            .expect("selectively restored foreground span should exist");
        assert_eq!(restored.style.foreground, Some(ScreenColor::Named { name: "red".to_string() }));
        assert!(
            restored.style.bold,
            "selective foreground restore must leave non-selected bold state untouched"
        );
    }

    #[test]
    fn preserves_xterm_sgr_stack_aliases_without_restoring_hyperlinks() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]8;;https://outer.example\x1b\\\x1b[34mblue\x1b[#p\
             \x1b]8;;https://inner.example\x1b\\\x1b[31mred\x1b[#qblue2",
        );

        assert_eq!(lines[0].text, "blueredblue2");

        let restored = lines[0]
            .spans
            .iter()
            .find(|span| span.text == "blue2")
            .expect("restored alias span should exist");
        assert_eq!(
            restored.style.foreground,
            Some(ScreenColor::Named { name: "blue".to_string() })
        );
        assert_eq!(restored.style.hyperlink.as_deref(), Some("https://inner.example"));
    }

    #[test]
    fn preserves_osc8_hyperlinks_until_closed() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\ plain",
        );

        assert_eq!(lines[0].text, "link plain");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "link" && span.style.hyperlink.as_deref() == Some("https://example.com")
        }));
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|span| { span.text == " plain" && span.style.hyperlink.is_none() })
        );
    }

    #[test]
    fn preserves_osc8_hyperlink_params_and_switches_without_leaking_payloads() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]8;id=one;https://one.example\x1b\\one\
             \x1b]8;id=two;https://two.example\x1b\\two\
             \x1b]8;;\x1b\\ plain",
        );

        assert_eq!(lines[0].text, "onetwo plain");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "one" && span.style.hyperlink.as_deref() == Some("https://one.example")
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "two" && span.style.hyperlink.as_deref() == Some("https://two.example")
        }));
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|span| { span.text == " plain" && span.style.hyperlink.is_none() })
        );
        assert!(!lines[0].text.contains("id=one"));
        assert!(!lines[0].text.contains("id=two"));
    }

    #[test]
    fn strips_unknown_control_strings_without_leaking_payloads() {
        let lines = screen_lines_from_ansi_output(
            "before \x1bPq#0;2;0;0;0#1~~@@\x1b\\ middle \x1b]52;c;SGVsbG8=\x07 after",
        );

        assert_eq!(lines[0].text, "before  middle  after");
        assert_eq!(lines[0].media, vec![ScreenLineMedia::marker(ScreenLineMediaKind::Sixel)]);
        assert_eq!(
            lines[0].side_effects,
            vec![ScreenLineSideEffect {
                kind: ScreenLineSideEffectKind::ClipboardWrite,
                disposition: ScreenLineSideEffectDisposition::Blocked,
                target: Some(ScreenLineSideEffectTarget::Clipboard),
                message: None,
            }]
        );
        assert!(!lines[0].text.contains("SGVsbG8"));
        assert!(!lines[0].text.contains("#1~~@@"));
    }

    #[test]
    fn preserves_c1_encoded_sgr_and_osc8_sequences() {
        let lines = screen_lines_from_ansi_output(
            "\u{009b}32mgreen\u{009b}0m \u{009d}8;;https://example.com\u{009c}link\u{009d}8;;\u{009c}",
        );

        assert_eq!(lines[0].text, "green link");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "green"
                && span.style.foreground == Some(ScreenColor::Named { name: "green".to_string() })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "link" && span.style.hyperlink.as_deref() == Some("https://example.com")
        }));
    }

    #[test]
    fn preserves_raw_c1_control_bytes_from_terminal_capture() {
        let lines = screen_lines_from_ansi_bytes(
            b"\x9b35mmagenta\x9b0m \x9d8;;https://example.com\x9clink\x9d8;;\x9c",
        );

        assert_eq!(lines[0].text, "magenta link");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "magenta"
                && span.style.foreground == Some(ScreenColor::Named { name: "magenta".to_string() })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "link" && span.style.hyperlink.as_deref() == Some("https://example.com")
        }));
    }

    #[test]
    fn preserves_raw_c1_index_and_next_line_controls() {
        let lines = screen_lines_from_ansi_bytes(b"alpha\x84beta\x85gamma");

        assert_eq!(
            lines,
            vec![
                ScreenLine::plain("alpha"),
                ScreenLine::plain("     beta"),
                ScreenLine::plain("gamma"),
            ]
        );
    }

    #[test]
    fn preserves_raw_c1_single_shift_controls_without_visible_garbage() {
        let lines = screen_lines_from_ansi_bytes(b"before \x8eA middle \x8fB after");

        assert_eq!(lines, vec![ScreenLine::plain("before A middle B after")]);
    }

    #[test]
    fn preserves_escaped_single_shift_controls_without_visible_garbage() {
        let lines = screen_lines_from_ansi_output("before \x1bNA middle \x1bOB after");

        assert_eq!(lines, vec![ScreenLine::plain("before A middle B after")]);
    }

    #[test]
    fn preserves_raw_c1_guard_and_terminal_id_controls_without_visible_garbage() {
        let lines = screen_lines_from_ansi_bytes(b"before \x96guarded\x97 \x9aafter");

        assert_eq!(lines, vec![ScreenLine::plain("before guarded after")]);
    }

    #[test]
    fn preserves_escaped_guard_and_terminal_id_controls_without_visible_garbage() {
        let lines = screen_lines_from_ansi_output("before \x1bVguarded\x1bW \x1bZafter");

        assert_eq!(lines, vec![ScreenLine::plain("before guarded after")]);
    }

    #[test]
    fn strips_raw_c1_privacy_control_strings_without_leaking_payloads() {
        let lines =
            screen_lines_from_ansi_bytes(b"before \x98secret\x9c middle \x9eprivate\x9c after");

        assert_eq!(lines, vec![ScreenLine::plain("before  middle  after")]);
    }

    #[test]
    fn screen_surface_preserves_title_bell_and_terminal_metadata() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]1;Icon shell\x07\x1b]2;Build shell\x07\x1b]7;file://localhost/tmp/project\x07\x07ready",
            Some("fallback".to_string()),
        );

        assert_eq!(surface.title.as_deref(), Some("Build shell"));
        assert_eq!(surface.working_directory_uri.as_deref(), Some("file://localhost/tmp/project"));
        assert_eq!(surface.bell_count, 1);
        assert_eq!(surface.lines, vec![ScreenLine::plain("ready")]);
    }

    #[test]
    fn screen_surface_uses_osc1_title_when_no_window_title_follows() {
        let surface = screen_surface_from_ansi_output("\x1b]1;Icon shell\x07ready", None);

        assert_eq!(surface.title.as_deref(), Some("Icon shell"));
        assert_eq!(surface.lines, vec![ScreenLine::plain("ready")]);
    }

    #[test]
    fn screen_surface_converts_terminal_working_directory_paths_to_file_uris() {
        let unix = screen_surface_from_ansi_output("\x1b]1337;CurrentDir=/tmp/dev space\x07", None);
        let windows =
            screen_surface_from_ansi_output("\x1b]9;9;\"C:\\Users\\belief\\dev space\"\x07", None);
        let vscode =
            screen_surface_from_ansi_output("\x1b]633;P;Cwd=/work/repo;IsWindows=False\x07", None);

        assert_eq!(unix.working_directory_uri.as_deref(), Some("file://localhost/tmp/dev%20space"));
        assert_eq!(
            windows.working_directory_uri.as_deref(),
            Some("file:///C:/Users/belief/dev%20space")
        );
        assert_eq!(vscode.working_directory_uri.as_deref(), Some("file://localhost/work/repo"));
    }

    #[test]
    fn screen_surface_preserves_progress_user_variables_and_palette() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]9;4;4;150\x07\
             \x1b]1337;SetUserVar=WEZTERM_PROG=Y2FyZ28gdGVzdA==\x07\
             \x1b]10;rgb:01/02/03;#0A0B0C;Light Blue\x07ok",
            None,
        );

        assert_eq!(
            surface.progress,
            ScreenProgress { state: ScreenProgressState::Warning, value: Some(100) }
        );
        assert_eq!(surface.user_variables.get("WEZTERM_PROG"), Some(&"cargo test".to_string()));
        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 10, g: 11, b: 12 }),
                cursor: Some(ScreenColor::Named { name: "Light Blue".to_string() }),
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_accepts_rgbi_color_specs() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]10;rgbi:1.0/0.5/0.0;rgbi:0.0/0.25/1.0;rgbi:0.1/0.2/0.3\x07ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 }),
                background: Some(ScreenColor::Rgb { r: 0, g: 64, b: 255 }),
                cursor: Some(ScreenColor::Rgb { r: 26, g: 51, b: 77 }),
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_accepts_rgba_color_specs() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]10;rgba:1212/3434/5656/7878;rgba:0a0a/0b0b/0c0c/ffff;rgba(13, 14, 15, 50%)\x07ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 }),
                background: Some(ScreenColor::Rgb { r: 10, g: 11, b: 12 }),
                cursor: Some(ScreenColor::Rgb { r: 13, g: 14, b: 15 }),
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_accepts_osc4_extended_default_slots() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]4;256;rgb:01/02/03;257;rgb:04/05/06;258;rgb:07/08/09\x07ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: Some(ScreenColor::Rgb { r: 7, g: 8, b: 9 }),
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_accepts_iterm2_set_colors_default_slots() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]1337;SetColors=fg=srgb:112233\x07\
             \x1b]1337;SetColors=bg=445566\x07\
             \x1b]1337;SetColors=curbg=p3:778899\x07ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 17, g: 34, b: 51 }),
                background: Some(ScreenColor::Rgb { r: 68, g: 85, b: 102 }),
                cursor: Some(ScreenColor::Rgb { r: 119, g: 136, b: 153 }),
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_accepts_kitty_color_control_default_slots() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]21;foreground=#112233;background=rgb:44/55/66;cursor=rgba(119, 136, 153, 0.5)\x1b\\ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 17, g: 34, b: 51 }),
                background: Some(ScreenColor::Rgb { r: 68, g: 85, b: 102 }),
                cursor: Some(ScreenColor::Rgb { r: 119, g: 136, b: 153 }),
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_accepts_iterm2_osc4_default_aliases() {
        let surface =
            screen_surface_from_ansi_output("\x1b]4;-1;rgb:01/02/03;-2;rgb:04/05/06\x07ok", None);

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: None,
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_accepts_iterm2_osc_p_default_slots() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]Pg112233\x07\x1b]Ph445566\x07\x1b]Pl778899\x07ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 }),
                background: Some(ScreenColor::Rgb { r: 0x44, g: 0x55, b: 0x66 }),
                cursor: Some(ScreenColor::Rgb { r: 0x77, g: 0x88, b: 0x99 }),
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_resets_dynamic_default_palette_entries() {
        let foreground_reset = screen_surface_from_ansi_output(
            "\x1b]10;rgb:01/02/03;rgb:04/05/06;rgb:07/08/09\x07\x1b]110\x07ok",
            None,
        );
        let background_reset = screen_surface_from_ansi_output(
            "\x1b]10;rgb:01/02/03;rgb:04/05/06;rgb:07/08/09\x07\x1b]111\x07ok",
            None,
        );
        let cursor_reset = screen_surface_from_ansi_output(
            "\x1b]10;rgb:01/02/03;rgb:04/05/06;rgb:07/08/09\x07\x1b]112\x07ok",
            None,
        );

        assert_eq!(
            foreground_reset.palette,
            ScreenSurfacePalette {
                foreground: None,
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: Some(ScreenColor::Rgb { r: 7, g: 8, b: 9 }),
            }
        );
        assert_eq!(
            background_reset.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: None,
                cursor: Some(ScreenColor::Rgb { r: 7, g: 8, b: 9 }),
            }
        );
        assert_eq!(
            cursor_reset.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: None,
            }
        );
    }

    #[test]
    fn screen_surface_palette_resets_osc4_extended_default_slots() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]4;256;rgb:01/02/03;257;rgb:04/05/06;258;rgb:07/08/09\x07\
             \x1b]104;256;258\x07ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: None,
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: None,
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_resets_iterm2_osc4_default_aliases() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]4;-1;rgb:01/02/03;-2;rgb:04/05/06\x07\
             \x1b]104;-1\x07ok",
            None,
        );

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: None,
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: None,
            }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_palette_reset_all_clears_osc4_extended_default_slots() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]4;256;rgb:01/02/03;257;rgb:04/05/06;258;rgb:07/08/09\x07\
             \x1b]104\x07ok",
            None,
        );

        assert_eq!(surface.palette, ScreenSurfacePalette::default());
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_tracks_explicit_cursor_position() {
        let surface = screen_surface_from_ansi_output("one\ntwo\x1b[1;3H", None);

        assert_eq!(surface.lines, vec![ScreenLine::plain("one"), ScreenLine::plain("two")]);
        assert_eq!(
            surface.cursor,
            Some(ScreenCursor { row: 0, col: 2, shape: None, blinking: false })
        );
    }

    #[test]
    fn screen_surface_tracks_cursor_shape_and_visibility_sequences() {
        let shaped = screen_surface_from_ansi_output("\x1b[5 qbar", None);
        let iterm_beam = screen_surface_from_ansi_output("\x1b]1337;CursorShape=1\x07bar", None);
        let iterm_underline =
            screen_surface_from_ansi_output("\x1b]1337;CursorShape=2\x07under", None);
        let hidden = screen_surface_from_ansi_output("abc\x1b[?25l", None);
        let visible = screen_surface_from_ansi_output("\x1b[?25l\x1b[?25h", None);
        let blinking = screen_surface_from_ansi_output("\x1b[?12habc", None);
        let steady = screen_surface_from_ansi_output("\x1b[?12h\x1b[?12labc", None);

        assert_eq!(
            shaped.cursor,
            Some(ScreenCursor {
                row: 0,
                col: 3,
                shape: Some(ScreenCursorShape::Beam),
                blinking: true,
            })
        );
        assert_eq!(
            iterm_beam.cursor,
            Some(ScreenCursor {
                row: 0,
                col: 3,
                shape: Some(ScreenCursorShape::Beam),
                blinking: false,
            })
        );
        assert_eq!(
            iterm_underline.cursor,
            Some(ScreenCursor {
                row: 0,
                col: 5,
                shape: Some(ScreenCursorShape::Underline),
                blinking: false,
            })
        );
        assert_eq!(
            hidden.cursor,
            Some(ScreenCursor {
                row: 0,
                col: 3,
                shape: Some(ScreenCursorShape::Hidden),
                blinking: false,
            })
        );
        assert_eq!(visible.cursor, Some(ScreenCursor::at(0, 0)));
        assert_eq!(
            blinking.cursor,
            Some(ScreenCursor { row: 0, col: 3, shape: None, blinking: true })
        );
        assert_eq!(steady.cursor, Some(ScreenCursor::at(0, 3)));
    }

    #[test]
    fn alternate_screen_switch_restores_normal_screen_after_exit() {
        let lines = screen_lines_from_ansi_output("normal\x1b[?1049hALT\x1b[?1049lback");

        assert_eq!(lines, vec![ScreenLine::plain("normalback")]);
    }

    #[test]
    fn alternate_screen_switch_exposes_active_alternate_buffer_until_exit() {
        let lines = screen_lines_from_ansi_output("normal\x1b[?1049hALT");

        assert_eq!(lines, vec![ScreenLine::plain("ALT")]);
    }

    #[test]
    fn alternate_screen_switch_restores_normal_rows_and_cursor_column() {
        let lines = screen_lines_from_ansi_output("one\ntwo\x1b[?1049hfull\x1b[?1049lback");

        assert_eq!(lines, vec![ScreenLine::plain("one"), ScreenLine::plain("twoback")]);
    }

    #[test]
    fn legacy_private_47_alternate_screen_switches_buffers() {
        let active = screen_lines_from_ansi_output("normal\x1b[?47hlegacy");
        let restored = screen_lines_from_ansi_output("normal\x1b[?47hlegacy\x1b[?47lback");

        assert_eq!(active, vec![ScreenLine::plain("legacy")]);
        assert_eq!(restored, vec![ScreenLine::plain("normalback")]);
    }

    #[test]
    fn private_1048_mode_saves_and_restores_cursor_without_switching_buffer() {
        let lines = screen_lines_from_ansi_output("abc\x1b[?1048hXYZ\x1b[?1048lQ");

        assert_eq!(lines, vec![ScreenLine::plain("abcQYZ")]);
    }

    #[test]
    fn screen_surface_ignores_invalid_metadata_without_clearing_previous_values() {
        let surface = screen_surface_from_ansi_output(
            "\x1b]9;4;1;55\x07\
             \x1b]7;file://localhost/tmp/project\x07\
             \x1b]9;4;bad;99\x07\
             \x1b]7;relative/project\x07\
             \x1b]1337;SetUserVar=BAD=not-valid-base64\x07ok",
            None,
        );

        assert_eq!(
            surface.progress,
            ScreenProgress { state: ScreenProgressState::Normal, value: Some(55) }
        );
        assert_eq!(surface.working_directory_uri.as_deref(), Some("file://localhost/tmp/project"));
        assert!(surface.user_variables.is_empty());
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    #[test]
    fn screen_surface_preserves_tmux_wrapped_metadata() {
        let surface =
            screen_surface_from_ansi_output("\x1bPtmux;\x1b\x1b]9;4;1;42\x07\x1b\\build", None);

        assert_eq!(
            surface.progress,
            ScreenProgress { state: ScreenProgressState::Normal, value: Some(42) }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("build")]);
    }

    #[test]
    fn preserves_escaped_index_and_next_line_controls() {
        let lines = screen_lines_from_ansi_output("alpha\x1bDbeta\x1bEgamma");

        assert_eq!(
            lines,
            vec![
                ScreenLine::plain("alpha"),
                ScreenLine::plain("     beta"),
                ScreenLine::plain("gamma"),
            ]
        );
    }

    #[test]
    fn index_preserves_column_while_next_line_returns_to_column_zero() {
        let escaped = screen_lines_from_ansi_output("ab\x1bDxy\nab\x1bExy");
        let raw_c1 = screen_lines_from_ansi_bytes(b"ab\x84xy\nab\x85xy");

        assert_eq!(
            escaped,
            vec![
                ScreenLine::plain("ab"),
                ScreenLine::plain("  xy"),
                ScreenLine::plain("ab"),
                ScreenLine::plain("xy"),
            ]
        );
        assert_eq!(raw_c1, escaped);
    }

    #[test]
    fn reset_terminal_state_clears_prior_output_and_rich_state() {
        let lines = screen_lines_from_ansi_output("old \x1b[31mred\x1bcnew");

        assert_eq!(lines, vec![ScreenLine::plain("new")]);
    }

    #[test]
    fn preserves_dec_special_graphics_line_drawing_output() {
        let lines =
            screen_lines_from_ansi_output("\x1b(0lqk\x1b(B\n\x1b(0x x\x1b(B\n\x1b(0mqj\x1b(B");

        assert_eq!(
            lines,
            vec![ScreenLine::plain("┌─┐"), ScreenLine::plain("│ │"), ScreenLine::plain("└─┘")]
        );
    }

    #[test]
    fn preserves_shifted_g1_dec_special_graphics_line_drawing_output() {
        let lines = screen_lines_from_ansi_output("\x1b)0\x0elqk\x0f ascii");

        assert_eq!(lines, vec![ScreenLine::plain("┌─┐ ascii")]);
    }

    #[test]
    fn strips_dec_and_iso_charset_designations_without_leaking_final_bytes() {
        let lines = screen_lines_from_ansi_output(
            "a\x1b#6b\x1b#8c\x1b%Gd\x1b%@e\x1b$)Cf\x1b(Bg\x1b*0h\x1b Fz",
        );

        assert_eq!(lines, vec![ScreenLine::plain("abcdefghz")]);
    }

    #[test]
    fn replaces_invalid_plain_bytes_without_losing_escape_sequences() {
        let lines = screen_lines_from_ansi_bytes(b"ok \xff \x1b[31mred\x1b[0m");

        assert_eq!(lines[0].text, "ok \u{fffd} red");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn preserves_iterm2_inline_image_metadata_without_payload_text() {
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
        let lines = screen_lines_from_ansi_output(&format!(
            "before \x1b]1337;File=name=dGlueS5wbmc=;size=68;width=2;height=1;preserveAspectRatio=1;inline=1:{PNG_1X1}\x07 after"
        ));

        assert_eq!(lines[0].text, "before  after");
        assert_eq!(
            lines[0].media,
            vec![ScreenLineMedia {
                kind: ScreenLineMediaKind::Iterm2Image,
                name: Some("tiny.png".to_string()),
                byte_size: Some(68),
                width: Some("2".to_string()),
                height: Some("1".to_string()),
                preserve_aspect_ratio: Some(true),
                inline: true,
                mime_type: Some("image/png".to_string()),
                data_base64: Some(PNG_1X1.to_string()),
                truncated: false,
            }]
        );
        assert!(!lines[0].text.contains(PNG_1X1));
    }

    #[test]
    fn preserves_iterm2_multipart_inline_image_metadata_without_payload_text() {
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
        let split_at = PNG_1X1.len() / 2;
        let lines = screen_lines_from_ansi_output(&format!(
            "before \x1b]1337;MultipartFile=name=dGlueS5wbmc=;size=68;width=2;height=1;preserveAspectRatio=1;inline=1\x07\
             \x1b]1337;FilePart={}\x07\
             \x1b]1337;FilePart={}\x07\
             \x1b]1337;FileEnd\x07 after",
            &PNG_1X1[..split_at],
            &PNG_1X1[split_at..],
        ));

        assert_eq!(lines[0].text, "before  after");
        assert_eq!(lines[0].media.len(), 1);
        assert_eq!(lines[0].media[0].kind, ScreenLineMediaKind::Iterm2Image);
        assert_eq!(lines[0].media[0].name.as_deref(), Some("tiny.png"));
        assert_eq!(lines[0].media[0].data_base64.as_deref(), Some(PNG_1X1));
        assert!(!lines[0].text.contains(PNG_1X1));
    }

    #[test]
    fn preserves_tmux_passthrough_rich_sequences() {
        let lines = screen_lines_from_ansi_output(
            "before \x1bPtmux;\x1b\x1b[31mred\x1b\x1b[0m\x1b\\ after",
        );

        assert_eq!(lines[0].text, "before red after");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn preserves_tmux_passthrough_dynamic_palette_colors() {
        let lines = screen_lines_from_ansi_output(
            "before \x1bPtmux;\x1b\x1b]4;1;#010203\x07\x1b\x1b[31mred\x1b\x1b[0m\x1b\\ after",
        );

        assert_eq!(lines[0].text, "before red after");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "red"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
        }));
    }

    #[test]
    fn preserves_kitty_graphics_chunks_and_hides_payload() {
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
        let (first_chunk, second_chunk) = PNG_1X1.split_at(44);
        let lines = screen_lines_from_ansi_output(&format!(
            "before \x1b_Ga=T,f=100,c=4,r=2,m=1;{first_chunk}\x1b\\\x1b_Gm=0;{second_chunk}\x1b\\ after"
        ));

        assert_eq!(lines[0].text, "before  after");
        assert_eq!(
            lines[0].media,
            vec![ScreenLineMedia {
                kind: ScreenLineMediaKind::KittyGraphics,
                name: None,
                byte_size: None,
                width: Some("4".to_string()),
                height: Some("2".to_string()),
                preserve_aspect_ratio: None,
                inline: true,
                mime_type: Some("image/png".to_string()),
                data_base64: Some(PNG_1X1.to_string()),
                truncated: false,
            }]
        );
        assert!(!lines[0].text.contains(first_chunk));
        assert!(!lines[0].text.contains(second_chunk));
    }

    #[test]
    fn strips_kitty_graphics_query_without_media_or_payload() {
        let lines = screen_lines_from_ansi_output(
            "before \x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\ after",
        );

        assert_eq!(lines[0].text, "before  after");
        assert!(lines[0].media.is_empty());
        assert!(!lines[0].text.contains("AAAA"));
    }

    #[test]
    fn preserves_blocked_notifications_and_shell_marks() {
        let lines = screen_lines_from_ansi_output(
            "\x1b]133;A\x07$ \x1b]133;B\x07echo hi\n\x1b]9;build finished\x07\x1b]133;D;2\x07",
        );

        assert_eq!(lines[0].text, "$ echo hi");
        assert!(
            lines[0]
                .semantic_marks
                .iter()
                .any(|mark| mark.kind == ScreenLineSemanticMarkKind::PromptStart && mark.col == 0)
        );
        assert!(
            lines[0]
                .semantic_marks
                .iter()
                .any(|mark| mark.kind == ScreenLineSemanticMarkKind::InputStart && mark.col == 2)
        );
        assert_eq!(
            lines[1].side_effects,
            vec![ScreenLineSideEffect {
                kind: ScreenLineSideEffectKind::DesktopNotification,
                disposition: ScreenLineSideEffectDisposition::Blocked,
                target: Some(ScreenLineSideEffectTarget::DesktopNotification),
                message: Some("build finished".to_string()),
            }]
        );
        assert!(lines[1].semantic_marks.iter().any(|mark| {
            mark.kind == ScreenLineSemanticMarkKind::CommandFinished && mark.exit_code == Some(2)
        }));
    }

    #[test]
    fn handles_carriage_return_progress_rewrites() {
        let lines = screen_lines_from_ansi_output("progress 10%\rprogress 90%\ndone");

        assert_eq!(lines, vec![ScreenLine::plain("progress 90%"), ScreenLine::plain("done")]);
    }

    #[test]
    fn preserves_colored_carriage_return_rewrites_through_sgr_controls() {
        let lines =
            screen_lines_from_ansi_output("progress 10%\r\x1b[32mprogress 90%\x1b[0m\ncomplete");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "progress 90%");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "progress 90%"
                && span.style.foreground == Some(ScreenColor::Named { name: "green".to_string() })
        }));
        assert_eq!(lines[1], ScreenLine::plain("complete"));
    }

    #[test]
    fn applies_erase_in_line_after_carriage_return() {
        let lines = screen_lines_from_ansi_output("old status\r\x1b[Knew");

        assert_eq!(lines, vec![ScreenLine::plain("new")]);
    }

    #[test]
    fn applies_full_line_erase_before_rewriting_output() {
        let lines = screen_lines_from_ansi_output("old status\r\x1b[2Knew");

        assert_eq!(lines, vec![ScreenLine::plain("new")]);
    }

    #[test]
    fn applies_clear_screen_before_redrawing_output() {
        let lines = screen_lines_from_ansi_output("old\nscreen\x1b[2Jfresh");

        assert_eq!(lines, vec![ScreenLine::plain("fresh")]);
    }

    #[test]
    fn rewrites_from_horizontal_absolute_start_column() {
        let lines = screen_lines_from_ansi_output("old status\x1b[1G\x1b[Knew");

        assert_eq!(lines, vec![ScreenLine::plain("new")]);
    }

    #[test]
    fn rewrites_after_large_cursor_backward_to_start() {
        let lines = screen_lines_from_ansi_output("old status\x1b[999D\x1b[Knew");

        assert_eq!(lines, vec![ScreenLine::plain("new")]);
    }

    #[test]
    fn overwrites_colored_text_after_cursor_backward_without_duplicate_output() {
        let lines = screen_lines_from_ansi_output("\x1b[31mabcdef\x1b[3D\x1b[32mXYZ\x1b[0m");

        assert_eq!(lines[0].text, "abcXYZ");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "abc"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "XYZ"
                && span.style.foreground == Some(ScreenColor::Named { name: "green".to_string() })
        }));
    }

    #[test]
    fn overwrites_from_horizontal_absolute_column_without_dropping_suffix() {
        let lines = screen_lines_from_ansi_output("abcdef\x1b[4GXY");

        assert_eq!(lines, vec![ScreenLine::plain("abcXYf")]);
    }

    #[test]
    fn cursor_forward_extends_with_blank_cells_before_colored_output() {
        let lines = screen_lines_from_ansi_output("a\x1b[3C\x1b[34mz");

        assert_eq!(lines[0].text, "a   z");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "z"
                && span.style.foreground == Some(ScreenColor::Named { name: "blue".to_string() })
        }));
    }

    #[test]
    fn erase_line_modes_apply_from_the_projected_cursor_column() {
        let erase_right = screen_lines_from_ansi_output("abcdef\x1b[3G\x1b[KZ");
        let erase_left = screen_lines_from_ansi_output("abcdef\x1b[4G\x1b[1KZ");

        assert_eq!(erase_right, vec![ScreenLine::plain("abZ")]);
        assert_eq!(erase_left, vec![ScreenLine::plain("   Zef")]);
    }

    #[test]
    fn selective_erase_without_protected_cells_blanks_visible_cells() {
        let line = screen_lines_from_ansi_output("abcdef\x1b[3G\x1b[?KZ");
        let display = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[2;2H\x1b[?JX");

        assert_eq!(line, vec![ScreenLine::plain("abZ")]);
        assert_eq!(
            display,
            vec![ScreenLine::plain("one"), ScreenLine::plain("tX "), ScreenLine::plain("")]
        );
    }

    #[test]
    fn decsca_marks_cells_that_decsel_preserves() {
        let lines = screen_lines_from_ansi_output("ab\x1b[1\"qCD\x1b[0\"qef\x1b[1G\x1b[?KZ");

        assert_eq!(lines, vec![ScreenLine::plain("Z CD  ")]);
    }

    #[test]
    fn spa_and_epa_guard_cells_that_decsel_preserves() {
        let lines = screen_lines_from_ansi_output("ab\x1bVCD\x1bWef\x1b[1G\x1b[?KZ");

        assert_eq!(lines, vec![ScreenLine::plain("Z CD  ")]);
    }

    #[test]
    fn decsed_preserves_protected_cells_across_display_clear() {
        let lines =
            screen_lines_from_ansi_output("one\nab\x1b[1\"qCD\x1b[0\"qef\nthree\x1b[2;1H\x1b[?JZ");

        assert_eq!(
            lines,
            vec![ScreenLine::plain("one"), ScreenLine::plain("Z CD  "), ScreenLine::plain("")]
        );
    }

    #[test]
    fn protected_cells_move_with_scroll_region_content() {
        let lines = screen_lines_from_ansi_output(
            "one\n\x1b[1\"qTWO\x1b[0\"q\nthree\x1b[1;3r\x1b[1S\x1b[1;1H\x1b[?2J",
        );

        assert_eq!(
            lines,
            vec![ScreenLine::plain("TWO"), ScreenLine::plain(""), ScreenLine::plain("")]
        );
    }

    #[test]
    fn insert_delete_and_erase_character_sequences_edit_current_line_cells() {
        let inserted = screen_lines_from_ansi_output("ab\x1b[31mcd\x1b[0mef\x1b[3G\x1b[2@XY");
        let deleted = screen_lines_from_ansi_output("abcdef\x1b[3G\x1b[2P");
        let erased = screen_lines_from_ansi_output("\x1b[31mabcdef\x1b[0m\x1b[3G\x1b[2X");

        assert_eq!(inserted[0].text, "abXYcdef");
        assert!(inserted[0].spans.iter().any(|span| {
            span.text == "cd"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert_eq!(deleted, vec![ScreenLine::plain("abef")]);
        assert_eq!(erased[0].text, "ab  ef");
        assert!(erased[0].spans.iter().any(|span| {
            span.text == "ab"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert!(erased[0].spans.iter().any(|span| {
            span.text == "ef"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn erase_character_sequence_uses_current_background_color_for_blank_cells() {
        let lines = screen_lines_from_ansi_output("abcdef\x1b[3G\x1b[48;2;9;8;7m\x1b[2X");

        assert_eq!(lines[0].text, "ab  ef");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "  "
                && span.style.background == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn space_intermediate_scroll_sequences_apply_without_falling_through_to_other_csi_commands() {
        let scroll_left = screen_lines_from_ansi_output("abc\x1b[2 @Z");
        let scroll_right = screen_lines_from_ansi_output("one\ntwo\x1b[1 AZ");

        assert_eq!(scroll_left, vec![ScreenLine::plain("c  Z")]);
        assert_eq!(scroll_right, vec![ScreenLine::plain(" on"), ScreenLine::plain(" twZ")]);
    }

    #[test]
    fn insert_mode_inserts_printable_output_at_cursor() {
        let lines = screen_lines_from_ansi_output("abcd\x1b[2G\x1b[4hXY\x1b[4lZ");

        assert_eq!(lines, vec![ScreenLine::plain("aXYZcd")]);
    }

    #[test]
    fn soft_terminal_reset_resets_insert_mode_style_and_saved_cursor_state() {
        let lines = screen_lines_from_ansi_output("abcd\x1b[2G\x1b[4h\x1b[31m\x1b7\x1b[!pZ\x1b8Q");

        assert_eq!(lines[0].text, "aZQd");
        assert!(lines[0].spans.iter().all(|span| span.style.is_plain()));
    }

    #[test]
    fn repeat_preceding_character_sequence_repeats_plain_and_styled_cells() {
        let plain = screen_lines_from_ansi_output("A\x1b[3b");
        let styled = screen_lines_from_ansi_output("\x1b[31mA\x1b[3b\x1b[0m");
        let default_count = screen_lines_from_ansi_output("x\x1b[b");

        assert_eq!(plain, vec![ScreenLine::plain("AAAA")]);
        assert_eq!(default_count, vec![ScreenLine::plain("xx")]);
        assert_eq!(styled[0].text, "AAAA");
        assert!(styled[0].spans.iter().any(|span| {
            span.text == "AAAA"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn repeat_preceding_character_sequence_preserves_fullwidth_cell_width() {
        let lines = screen_lines_from_ansi_output("表\x1b[2bZ");

        assert_eq!(lines, vec![ScreenLine::plain("表表表Z")]);
    }

    #[test]
    fn deccara_changes_attributes_in_rectangular_area() {
        let lines = screen_lines_from_ansi_output("abcdef\nuvwxyz\x1b[1;2;2;4;1;4$r");

        assert_eq!(lines[0].text, "abcdef");
        assert_eq!(lines[1].text, "uvwxyz");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bcd"
                && span.style.bold
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
        }));
        assert!(lines[1].spans.iter().any(|span| {
            span.text == "vwx"
                && span.style.bold
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
        }));
    }

    #[test]
    fn deccara_resets_selected_attributes_without_losing_colors() {
        let lines = screen_lines_from_ansi_output("\x1b[31;1;4mabcdef\x1b[0m\x1b[1;2;1;4;22;24$r");

        assert_eq!(lines[0].text, "abcdef");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "a"
                && span.style.bold
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "bcd"
                && !span.style.bold
                && span.style.underline.is_none()
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn decrara_reverses_attributes_in_rectangular_area() {
        let lines = screen_lines_from_ansi_output("abcdef\x1b[1;2;1;4;1;7$t");

        assert_eq!(lines[0].text, "abcdef");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|span| { span.text == "bcd" && span.style.bold && span.style.inverse })
        );
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|span| { span.text == "a" && !span.style.bold && !span.style.inverse })
        );
    }

    #[test]
    fn decfra_fills_rectangular_area_with_current_style_without_moving_cursor() {
        let lines = screen_lines_from_ansi_output("ab\ncd\x1b[31m\x1b[88;1;1;2;2$xZ\x1b[0m");

        assert_eq!(lines[0].text, "XX");
        assert_eq!(lines[1].text, "XXZ");
        assert!(lines.iter().all(|line| {
            line.spans.iter().any(|span| {
                span.text.contains('X')
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            })
        }));
    }

    #[test]
    fn decera_erases_rectangular_area_to_blank_cells() {
        let lines = screen_lines_from_ansi_output("abcdef\nuvwxyz\x1b[1;2;2;4$z");

        assert_eq!(lines, vec![ScreenLine::plain("a   ef"), ScreenLine::plain("u   yz")]);
    }

    #[test]
    fn decera_uses_current_background_color_for_blank_cells() {
        let lines = screen_lines_from_ansi_output("abcdef\nuvwxyz\x1b[48;2;9;8;7m\x1b[1;2;2;4$z");

        assert_eq!(lines[0].text, "a   ef");
        assert_eq!(lines[1].text, "u   yz");
        assert!(lines.iter().all(|line| {
            line.spans.iter().any(|span| {
                span.text == "   "
                    && span.style.background == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            })
        }));
    }

    #[test]
    fn decsera_preserves_protected_cells_in_rectangular_area() {
        let lines =
            screen_lines_from_ansi_output("abcdef\x1b[1;3H\x1b[1\"qCD\x1b[0\"q\x1b[1;1;1;6${");

        assert_eq!(lines, vec![ScreenLine::plain("  CD  ")]);
    }

    #[test]
    fn deccra_copies_rectangular_area_with_rich_styles_without_moving_cursor() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[31;48;2;1;2;3mAB\x1b[0mcd\n\
             \x1b[4:3;58;2;9;8;7mEF\x1b[0mgh\
             \x1b[1;1;2;2;1;1;3;1$vZ",
        );

        assert_eq!(lines[0].text, "ABAB");
        assert_eq!(lines[1].text, "EFEFZ");
        assert!(lines[0].spans.iter().any(|span| {
            span.text.contains("AB")
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
                && span.style.background == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
        }));
        assert!(lines[1].spans.iter().any(|span| {
            span.text.contains("EF")
                && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn deccra_uses_snapshot_when_source_and_destination_overlap() {
        let lines = screen_lines_from_ansi_output("abcdef\x1b[1;1;1;4;1;1;3;1$v");

        assert_eq!(lines, vec![ScreenLine::plain("ababcd")]);
    }

    #[test]
    fn deccra_preserves_protected_attributes_for_later_selective_erase() {
        let lines = screen_lines_from_ansi_output(
            "abcdef\x1b[1;2H\x1b[1\"qBC\x1b[0\"q\x1b[1;2;1;3;1;1;5;1$v\x1b[1;1;1;6${",
        );

        assert_eq!(lines, vec![ScreenLine::plain(" BC BC")]);
    }

    #[test]
    fn fullwidth_output_advances_cursor_by_terminal_cells() {
        let lines = screen_lines_from_ansi_output("表\x1b[2CZ");

        assert_eq!(lines, vec![ScreenLine::plain("表  Z")]);
    }

    #[test]
    fn horizontal_absolute_uses_terminal_cell_columns_for_fullwidth_text() {
        let lines = screen_lines_from_ansi_output("表A\x1b[3GZ");

        assert_eq!(lines, vec![ScreenLine::plain("表Z")]);
    }

    #[test]
    fn fullwidth_spacers_stay_hidden_in_rich_spans() {
        let lines = screen_lines_from_ansi_output("\x1b[31m表\x1b[0mA");

        assert_eq!(lines[0].text, "表A");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "表"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert!(lines[0].spans.iter().all(|span| !span.text.contains(' ')));
    }

    #[test]
    fn zero_width_marks_attach_to_previous_styled_cell() {
        let lines = screen_lines_from_ansi_output("\x1b[32me\u{0301}\x1b[0mZ");

        assert_eq!(lines[0].text, "e\u{0301}Z");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "e\u{0301}"
                && span.style.foreground == Some(ScreenColor::Named { name: "green".to_string() })
        }));
    }

    #[test]
    fn tabs_advance_from_terminal_cell_columns_after_fullwidth_text() {
        let lines = screen_lines_from_ansi_output("表\tX");

        assert_eq!(lines, vec![ScreenLine::plain("表      X")]);
    }

    #[test]
    fn cursor_tabulation_sequences_move_by_terminal_tab_stops() {
        let forward = screen_lines_from_ansi_output("A\x1b[IZ");
        let forward_twice = screen_lines_from_ansi_output("A\x1b[2IZ");
        let backward = screen_lines_from_ansi_output("A\x1b[10GZ\x1b[ZQ");

        assert_eq!(forward, vec![ScreenLine::plain("A       Z")]);
        assert_eq!(forward_twice, vec![ScreenLine::plain("A               Z")]);
        assert_eq!(backward, vec![ScreenLine::plain("A       QZ")]);
    }

    #[test]
    fn horizontal_tab_set_sequences_define_extra_tab_stops() {
        let escaped = screen_lines_from_ansi_output("ab\x1b[6G\x1bH\x1b[1Ga\tZ");
        let raw_c1 = screen_lines_from_ansi_bytes(b"ab\x1b[6G\x88\x1b[1Ga\tZ");

        assert_eq!(escaped, vec![ScreenLine::plain("a    Z")]);
        assert_eq!(raw_c1, escaped);
    }

    #[test]
    fn tab_clear_sequences_remove_current_or_all_tab_stops() {
        let clear_current = screen_lines_from_ansi_output("ab\x1b[6G\x1bH\x1b[g\x1b[1Ga\tZ");
        let clear_all = screen_lines_from_ansi_output("ab\x1b[6G\x1bH\x1b[3g\x1b[1Ga\tZ");
        let reset_restores_defaults =
            screen_lines_from_ansi_output("ab\x1b[6G\x1bH\x1b[3g\x1bcA\tZ");

        assert_eq!(clear_current, vec![ScreenLine::plain("a       Z")]);
        assert_eq!(clear_all, vec![ScreenLine::plain("aZ")]);
        assert_eq!(reset_restores_defaults, vec![ScreenLine::plain("A       Z")]);
    }

    #[test]
    fn cursor_backward_tabulation_respects_custom_tab_stops() {
        let lines = screen_lines_from_ansi_output("ab\x1b[6G\x1bH\x1b[8GZ\x1b[ZQ");

        assert_eq!(lines, vec![ScreenLine::plain("ab   Q Z")]);
    }

    #[test]
    fn erase_character_sequence_clears_fullwidth_clusters() {
        let lines = screen_lines_from_ansi_output("A表B\x1b[2G\x1b[2X");

        assert_eq!(lines, vec![ScreenLine::plain("A  B")]);
    }

    #[test]
    fn erase_line_from_inside_fullwidth_cluster_drops_partial_cell() {
        let lines = screen_lines_from_ansi_output("表B\x1b[2G\x1b[K");

        assert_eq!(lines, vec![ScreenLine::plain(" ")]);
    }

    #[test]
    fn delete_character_from_inside_fullwidth_cluster_does_not_leave_stale_spacer() {
        let lines = screen_lines_from_ansi_output("A表B\x1b[3G\x1b[1P");

        assert_eq!(lines, vec![ScreenLine::plain("A B")]);
    }

    #[test]
    fn dec_save_and_restore_cursor_preserves_line_cursor_and_style() {
        let lines = screen_lines_from_ansi_output("\x1b[31mab\x1b7\x1b[32mcd\x1b8XY");

        assert_eq!(lines[0].text, "abXY");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "abXY"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn csi_save_and_restore_cursor_supports_dynamic_overwrite_output() {
        let lines = screen_lines_from_ansi_output("load: \x1b[s10%\x1b[u90%");

        assert_eq!(lines, vec![ScreenLine::plain("load: 90%")]);
    }

    #[test]
    fn cursor_up_rewrites_previous_line_without_dropping_following_lines() {
        let lines = screen_lines_from_ansi_output("first\nsecond\x1b[1A\x1b[1G\x1b[Kupdated");

        assert_eq!(lines, vec![ScreenLine::plain("updated"), ScreenLine::plain("second")]);
    }

    #[test]
    fn cursor_position_rewrites_existing_rows_with_rich_styles() {
        let lines = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[2;2H\x1b[31mXX\x1b[0m");

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], ScreenLine::plain("one"));
        assert_eq!(lines[1].text, "tXX");
        assert_eq!(lines[2], ScreenLine::plain("three"));
        assert!(lines[1].spans.iter().any(|span| {
            span.text == "XX"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn erase_display_from_cursor_removes_rows_below() {
        let lines = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[2;2H\x1b[JX");

        assert_eq!(lines, vec![ScreenLine::plain("one"), ScreenLine::plain("tX")]);
    }

    #[test]
    fn cursor_next_and_previous_line_move_to_column_zero() {
        let previous = screen_lines_from_ansi_output("one\ntwo\x1b[1FXX");
        let next = screen_lines_from_ansi_output("one\x1b[1EXX");

        assert_eq!(previous, vec![ScreenLine::plain("XXe"), ScreenLine::plain("two")]);
        assert_eq!(next, vec![ScreenLine::plain("one"), ScreenLine::plain("XX")]);
    }

    #[test]
    fn cursor_down_creates_blank_rows_before_late_output() {
        let lines = screen_lines_from_ansi_output("\x1b[2Bbottom");

        assert_eq!(
            lines,
            vec![ScreenLine::plain(""), ScreenLine::plain(""), ScreenLine::plain("bottom")]
        );
    }

    #[test]
    fn cursor_down_preserves_column_when_creating_blank_rows() {
        let lines = screen_lines_from_ansi_output("abc\x1b[2D\x1b[1Bz");

        assert_eq!(lines, vec![ScreenLine::plain("abc"), ScreenLine::plain(" z")]);
    }

    #[test]
    fn insert_and_delete_lines_edit_rows_from_cursor_row() {
        let inserted = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[2;1H\x1b[1Lnew");
        let deleted = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[2;1H\x1b[1M");

        assert_eq!(
            inserted,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain("new"),
                ScreenLine::plain("two"),
                ScreenLine::plain("three"),
            ]
        );
        assert_eq!(
            deleted,
            vec![ScreenLine::plain("one"), ScreenLine::plain("three"), ScreenLine::plain("")]
        );
    }

    #[test]
    fn scroll_up_and_down_preserve_screen_height_with_blank_rows() {
        let scrolled_up = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[1S");
        let scrolled_down = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[1T");
        let ecma_scrolled_down = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[1^");

        assert_eq!(
            scrolled_up,
            vec![ScreenLine::plain("two"), ScreenLine::plain("three"), ScreenLine::plain("")]
        );
        assert_eq!(
            scrolled_down,
            vec![ScreenLine::plain(""), ScreenLine::plain("one"), ScreenLine::plain("two")]
        );
        assert_eq!(
            ecma_scrolled_down,
            vec![ScreenLine::plain(""), ScreenLine::plain("one"), ScreenLine::plain("two")]
        );
    }

    #[test]
    fn scroll_region_linefeed_scrolls_only_margins() {
        let lines = screen_lines_from_ansi_output("one\ntwo\nthree\nfour\x1b[2;3r\x1b[3;1H\nX");

        assert_eq!(
            lines,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain("three"),
                ScreenLine::plain("X"),
                ScreenLine::plain("four"),
            ]
        );
    }

    #[test]
    fn scroll_region_reverse_index_scrolls_only_margins() {
        let lines = screen_lines_from_ansi_output("one\ntwo\nthree\nfour\x1b[2;3r\x1b[2;1H\x1bMX");

        assert_eq!(
            lines,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain("X"),
                ScreenLine::plain("two"),
                ScreenLine::plain("four"),
            ]
        );
    }

    #[test]
    fn scroll_region_limits_insert_and_delete_lines() {
        let inserted =
            screen_lines_from_ansi_output("one\ntwo\nthree\nfour\x1b[2;4r\x1b[3;1H\x1b[L");
        let deleted =
            screen_lines_from_ansi_output("one\ntwo\nthree\nfour\x1b[2;4r\x1b[3;1H\x1b[M");

        assert_eq!(
            inserted,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain("two"),
                ScreenLine::plain(""),
                ScreenLine::plain("three"),
            ]
        );
        assert_eq!(
            deleted,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain("two"),
                ScreenLine::plain("four"),
                ScreenLine::plain(""),
            ]
        );
    }

    #[test]
    fn scroll_region_limits_scroll_up_and_down() {
        let up = screen_lines_from_ansi_output("one\ntwo\nthree\nfour\x1b[2;3r\x1b[1S");
        let down = screen_lines_from_ansi_output("one\ntwo\nthree\nfour\x1b[2;3r\x1b[1T");

        assert_eq!(
            up,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain("three"),
                ScreenLine::plain(""),
                ScreenLine::plain("four"),
            ]
        );
        assert_eq!(
            down,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain(""),
                ScreenLine::plain("two"),
                ScreenLine::plain("four"),
            ]
        );
    }

    #[test]
    fn decic_inserts_columns_inside_projected_screen_width() {
        let lines = screen_lines_from_ansi_output("abcdef\n123456\x1b[1;3H\x1b[2'}");

        assert_eq!(lines[0].text, "ab  cd");
        assert_eq!(lines[1].text, "12  34");
    }

    #[test]
    fn decdc_deletes_columns_inside_projected_screen_width() {
        let lines = screen_lines_from_ansi_output("abcdef\n123456\x1b[1;3H\x1b[2'~");

        assert_eq!(lines[0].text, "abef  ");
        assert_eq!(lines[1].text, "1256  ");
    }

    #[test]
    fn decic_and_decdc_preserve_cursor_position() {
        let inserted = screen_surface_from_ansi_output("abcdef\x1b[1;3H\x1b[2'}", None);
        let deleted = screen_surface_from_ansi_output("abcdef\x1b[1;3H\x1b[2'~", None);

        assert_eq!(inserted.cursor.map(|cursor| (cursor.row, cursor.col)), Some((0, 2)));
        assert_eq!(deleted.cursor.map(|cursor| (cursor.row, cursor.col)), Some((0, 2)));
    }

    #[test]
    fn decic_and_decdc_only_affect_rows_inside_scroll_margins() {
        let inserted = screen_lines_from_ansi_output(
            "one111\ntwo222\nthr333\nfor444\x1b[2;3r\x1b[2;4H\x1b[2'}",
        );
        let deleted = screen_lines_from_ansi_output(
            "one111\ntwo222\nthr333\nfor444\x1b[2;3r\x1b[2;4H\x1b[2'~",
        );

        assert_eq!(
            inserted,
            vec![
                ScreenLine::plain("one111"),
                ScreenLine::plain("two  2"),
                ScreenLine::plain("thr  3"),
                ScreenLine::plain("for444"),
            ]
        );
        assert_eq!(
            deleted,
            vec![
                ScreenLine::plain("one111"),
                ScreenLine::plain("two2  "),
                ScreenLine::plain("thr3  "),
                ScreenLine::plain("for444"),
            ]
        );
    }

    #[test]
    fn decdc_moves_protected_cells_with_shifted_content() {
        let lines = screen_lines_from_ansi_output(
            "\x1b[1\"qabcdef\x1b[0\"q\x1b[1;3H\x1b[2'~\x1b[1;1H\x1b[?2K",
        );

        assert_eq!(lines, vec![ScreenLine::plain("abef  ")]);
    }

    #[test]
    fn sl_scrolls_left_inside_projected_screen_width() {
        let lines = screen_lines_from_ansi_output("abcdef\n123456\x1b[2 @");

        assert_eq!(lines[0].text, "cdef  ");
        assert_eq!(lines[1].text, "3456  ");
    }

    #[test]
    fn sr_scrolls_right_inside_projected_screen_width() {
        let lines = screen_lines_from_ansi_output("abcdef\n123456\x1b[2 A");

        assert_eq!(lines[0].text, "  abcd");
        assert_eq!(lines[1].text, "  1234");
    }

    #[test]
    fn sr_uses_current_background_color_for_inserted_blank_cells() {
        let lines = screen_lines_from_ansi_output("abcdef\x1b[48;2;9;8;7m\x1b[2 A");

        assert_eq!(lines[0].text, "  abcd");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "  "
                && span.style.background == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn decbi_and_decfi_move_cursor_inside_projected_screen_width() {
        let back = screen_lines_from_ansi_output("abc\x1b6Z");
        let forward = screen_lines_from_ansi_output("abc\x1b[1;2H\x1b9Z");

        assert_eq!(back, vec![ScreenLine::plain("abZ")]);
        assert_eq!(forward, vec![ScreenLine::plain("abZ")]);
    }

    #[test]
    fn decbi_and_decfi_shift_content_at_projected_screen_edges() {
        let back = screen_lines_from_ansi_output("abcdef\n123456\x1b[1;1H\x1b6");
        let forward = screen_lines_from_ansi_output("abcdef\n123456\x1b[1;6H\x1b9");

        assert_eq!(back[0].text, " abcde");
        assert_eq!(back[1].text, " 12345");
        assert_eq!(forward[0].text, "bcdef ");
        assert_eq!(forward[1].text, "23456 ");
    }

    #[test]
    fn decbi_and_decfi_preserve_cursor_position_when_shifting_content() {
        let back = screen_surface_from_ansi_output("abcdef\x1b[1;1H\x1b6", None);
        let forward = screen_surface_from_ansi_output("abcdef\x1b[1;6H\x1b9", None);

        assert_eq!(back.cursor.map(|cursor| (cursor.row, cursor.col)), Some((0, 0)));
        assert_eq!(forward.cursor.map(|cursor| (cursor.row, cursor.col)), Some((0, 5)));
    }

    #[test]
    fn decbi_uses_current_background_color_for_inserted_blank_cells() {
        let lines = screen_lines_from_ansi_output("abcdef\x1b[48;2;9;8;7m\x1b[1;1H\x1b6");

        assert_eq!(lines[0].text, " abcde");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == " " && span.style.background == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn left_right_margins_limit_scroll_left_and_right_columns() {
        let left = screen_lines_from_ansi_output("abcdef\n123456\x1b[?69h\x1b[2;5s\x1b[1 @");
        let right = screen_lines_from_ansi_output("abcdef\n123456\x1b[?69h\x1b[2;5s\x1b[1 A");

        assert_eq!(left[0].text, "acde f");
        assert_eq!(left[1].text, "1345 6");
        assert_eq!(right[0].text, "a bcdf");
        assert_eq!(right[1].text, "1 2346");
    }

    #[test]
    fn left_right_margins_limit_insert_and_delete_columns() {
        let inserted =
            screen_lines_from_ansi_output("abcdef\n123456\x1b[?69h\x1b[2;5s\x1b[1;3H\x1b[1'}");
        let deleted =
            screen_lines_from_ansi_output("abcdef\n123456\x1b[?69h\x1b[2;5s\x1b[1;3H\x1b[1'~");

        assert_eq!(inserted[0].text, "ab cdf");
        assert_eq!(inserted[1].text, "12 346");
        assert_eq!(deleted[0].text, "abde f");
        assert_eq!(deleted[1].text, "1245 6");
    }

    #[test]
    fn left_right_margins_limit_back_and_forward_index() {
        let back = screen_lines_from_ansi_output("abcdef\n123456\x1b[?69h\x1b[2;5s\x1b[1;2H\x1b6");
        let forward =
            screen_lines_from_ansi_output("abcdef\n123456\x1b[?69h\x1b[2;5s\x1b[1;5H\x1b9");

        assert_eq!(back[0].text, "a bcdf");
        assert_eq!(back[1].text, "1 2346");
        assert_eq!(forward[0].text, "acde f");
        assert_eq!(forward[1].text, "1345 6");
    }

    #[test]
    fn resetting_left_right_margin_mode_restores_full_width_column_operations() {
        let lines =
            screen_lines_from_ansi_output("abcdef\n123456\x1b[?69h\x1b[2;5s\x1b[?69l\x1b[1 @");

        assert_eq!(lines[0].text, "bcdef ");
        assert_eq!(lines[1].text, "23456 ");
    }

    #[test]
    fn sl_and_sr_preserve_cursor_position() {
        let left = screen_surface_from_ansi_output("abcdef\x1b[1;3H\x1b[2 @", None);
        let right = screen_surface_from_ansi_output("abcdef\x1b[1;3H\x1b[2 A", None);

        assert_eq!(left.cursor.map(|cursor| (cursor.row, cursor.col)), Some((0, 2)));
        assert_eq!(right.cursor.map(|cursor| (cursor.row, cursor.col)), Some((0, 2)));
    }

    #[test]
    fn sl_and_sr_only_affect_rows_inside_scroll_margins() {
        let left = screen_lines_from_ansi_output("one111\ntwo222\nthr333\nfor444\x1b[2;3r\x1b[2 @");
        let right =
            screen_lines_from_ansi_output("one111\ntwo222\nthr333\nfor444\x1b[2;3r\x1b[2 A");

        assert_eq!(
            left,
            vec![
                ScreenLine::plain("one111"),
                ScreenLine::plain("o222  "),
                ScreenLine::plain("r333  "),
                ScreenLine::plain("for444"),
            ]
        );
        assert_eq!(
            right,
            vec![
                ScreenLine::plain("one111"),
                ScreenLine::plain("  two2"),
                ScreenLine::plain("  thr3"),
                ScreenLine::plain("for444"),
            ]
        );
    }

    #[test]
    fn sl_moves_protected_cells_with_shifted_content() {
        let lines =
            screen_lines_from_ansi_output("\x1b[1\"qabcdef\x1b[0\"q\x1b[2 @\x1b[1;1H\x1b[?2K");

        assert_eq!(lines, vec![ScreenLine::plain("cdef  ")]);
    }

    #[test]
    fn scroll_region_origin_mode_makes_cursor_position_relative_to_margins() {
        let lines = screen_lines_from_ansi_output(
            "one\ntwo\nthree\nfour\x1b[2;3r\x1b[?6h\x1b[1;1HX\x1b[?6l\x1b[1;1HY",
        );

        assert_eq!(
            lines,
            vec![
                ScreenLine::plain("Yne"),
                ScreenLine::plain("Xwo"),
                ScreenLine::plain("three"),
                ScreenLine::plain("four"),
            ]
        );
    }

    #[test]
    fn scroll_region_origin_mode_clamps_relative_cursor_motion_to_margins() {
        let lines = screen_lines_from_ansi_output("one\ntwo\nthree\nfour\x1b[2;3r\x1b[?6h\x1b[5BX");

        assert_eq!(
            lines,
            vec![
                ScreenLine::plain("one"),
                ScreenLine::plain("two"),
                ScreenLine::plain("Xhree"),
                ScreenLine::plain("four"),
            ]
        );
    }

    #[test]
    fn scroll_region_defaults_omitted_bottom_to_projected_screen_height() {
        let lines = screen_lines_from_ansi_output("one\ntwo\nthree\x1b[2;r\x1b[?6h\x1b[2;1HX");

        assert_eq!(
            lines,
            vec![ScreenLine::plain("one"), ScreenLine::plain("two"), ScreenLine::plain("Xhree"),]
        );
    }

    #[test]
    fn reset_terminal_state_clears_saved_cursor_state() {
        let lines = screen_lines_from_ansi_output("abc\x1b7\x1bc\x1b8Z");

        assert_eq!(lines, vec![ScreenLine::plain("Z")]);
    }

    #[test]
    fn preserves_nroff_backspace_overstrike_bold_output() {
        let lines = screen_lines_from_ansi_output("N\x08NA\x08AM\x08ME\x08E\nplain");

        assert_eq!(lines[0].text, "NAME");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "NAME" && span.style.bold && span.style.underline.is_none()
        }));
        assert_eq!(lines[1], ScreenLine::plain("plain"));
    }

    #[test]
    fn preserves_nroff_backspace_overstrike_underline_output() {
        let lines = screen_lines_from_ansi_output("_\x08N_\x08A_\x08M_\x08E and P\x08_");

        assert_eq!(lines[0].text, "NAME and P");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "NAME"
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
                && !span.style.bold
        }));
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "P"
                && span.style.underline == Some(ScreenUnderlineStyle::Single)
                && !span.style.bold
        }));
    }

    #[test]
    fn keeps_plain_backspace_rewrite_behavior_when_not_overstriking() {
        let lines = screen_lines_from_ansi_output("abc\x08X");

        assert_eq!(lines, vec![ScreenLine::plain("abX")]);
    }
}
