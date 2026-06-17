use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use alacritty_terminal::{
    event::{Event, EventListener, WindowSize},
    grid::Dimensions,
    index::{Column, Line},
    term::{
        Config, Term, TermMode,
        cell::{Cell, Flags},
        color::{COUNT as ALACRITTY_COLOR_COUNT, Colors},
    },
    vte::ansi::{self, Color, CursorShape as AlacrittyCursorShape, NamedColor, Rgb},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use terminal_projection::{
    ScreenBufferKind, ScreenColor, ScreenCursor, ScreenCursorShape, ScreenLine, ScreenLineMedia,
    ScreenLineMediaKind, ScreenLineSemanticMark, ScreenLineSemanticMarkKind, ScreenLineSideEffect,
    ScreenLineSideEffectDisposition, ScreenLineSideEffectKind, ScreenLineSideEffectTarget,
    ScreenLineSpan, ScreenProgress, ScreenProgressState, ScreenSurface, ScreenSurfacePalette,
    ScreenTextBaseline, ScreenTextBorderStyle, ScreenTextStyle, ScreenUnderlineStyle,
    ansi_color::{
        AnsiSgrColorTarget as SgrColorTarget, TerminalKittyColorControlOperation,
        TerminalKittyColorStackOperation, TerminalPaletteTarget, TerminalXtermColorStackOperation,
        is_legacy_linux_console_palette_reset, next_terminal_default_palette_target,
        parse_colon_sgr_color_part, parse_iterm2_set_colors_update,
        parse_legacy_linux_console_palette_update, parse_semicolon_sgr_color_fields,
        parse_terminal_color_spec, parse_terminal_kitty_color_control,
        parse_terminal_kitty_color_stack, parse_terminal_osc_p_palette_update,
        parse_terminal_xterm_color_stack, terminal_default_palette_target_from_osc_code,
    },
    ansi_rect::{
        TerminalRectangularArea, TerminalRectangularCopyRequest, TerminalRectangularFillRequest,
        parse_terminal_rectangular_area, parse_terminal_rectangular_copy_request,
        parse_terminal_rectangular_fill_request,
    },
    ansi_sgr::{
        AnsiSgrStackAttributes, TerminalRectangularAttributeMode,
        apply_terminal_rectangular_attribute_actions, parse_colon_sgr_underline_style,
        parse_terminal_rectangular_attribute_request, parse_xterm_sgr_stack_attributes,
    },
};
use unicode_width::UnicodeWidthChar;

const MAX_EXTRA_SGR_STACK_DEPTH: usize = 32;
const MAX_COLOR_STACK_DEPTH: usize = 32;
const FALLBACK_TERMINAL_CELL_WIDTH_PX: u16 = 8;
const FALLBACK_TERMINAL_CELL_HEIGHT_PX: u16 = 16;
const ALACRITTY_TERMINAL_SECONDARY_DEVICE_ATTRIBUTES_VERSION: usize = 2600;
const TERMINAL_PLATFORM_PROGRAM_NAME: &str = "terminal-platform";
const NATIVE_SGR_COLOR_NAMES: [&str; 16] = [
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

pub(super) struct EmulatorBuffer {
    inner: Mutex<EmulatorState>,
}

struct EmulatorState {
    term: Term<EmulatorEventListener>,
    parser: ansi::Processor,
    bell_count: Arc<AtomicU64>,
    response_bytes: Arc<Mutex<Vec<Vec<u8>>>>,
    response_colors: Arc<Mutex<Colors>>,
    emulator_title: Arc<Mutex<Option<String>>>,
    window_size: Arc<Mutex<WindowSize>>,
    working_directory_uri: Option<String>,
    user_variables: BTreeMap<String, String>,
    progress: ScreenProgress,
    extra_sgr_tracker: ExtraSgrTracker,
    extra_styles: ExtraTextStyleOverlay,
    rectangular_attribute_tracker: TerminalRectangularAttributeTracker,
    rectangular_area_tracker: TerminalRectangularAreaTracker,
    scroll_region_tracker: TerminalScrollRegionTracker,
    horizontal_margin_tracker: TerminalHorizontalMarginTracker,
    column_control_tracker: TerminalColumnControlTracker,
    protection_tracker: TerminalProtectionTracker,
    protected_cells: TerminalProtectedCellOverlay,
    metadata_tracker: TerminalMetadataSequenceTracker,
    visual_passthrough_tracker: TerminalTmuxPassthroughVisualTracker,
    color_sequence_tracker: TerminalColorSequenceTracker,
    color_stack: TerminalColorStack,
    synchronized_output_tracker: TerminalSynchronizedOutputTracker,
    synchronized_output_snapshot: Option<RenderedEmulator>,
    media_tracker: TerminalMediaSequenceTracker,
    media_overlay: TerminalMediaOverlay,
    side_effect_overlay: TerminalSideEffectOverlay,
    semantic_mark_overlay: TerminalSemanticMarkOverlay,
}

#[derive(Debug, Clone)]
pub(super) struct RenderedEmulator {
    pub buffer_kind: ScreenBufferKind,
    pub surface: ScreenSurface,
}

#[derive(Clone, Copy)]
struct TerminalDimensions {
    rows: usize,
    cols: usize,
}

impl TerminalDimensions {
    fn new(rows: u16, cols: u16) -> Self {
        Self { rows: usize::from(rows.max(1)), cols: usize::from(cols.max(1)) }
    }
}

impl Dimensions for TerminalDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

impl EmulatorBuffer {
    pub(super) fn new(rows: u16, cols: u16) -> Self {
        let dimensions = TerminalDimensions::new(rows, cols);
        let bell_count = Arc::new(AtomicU64::new(0));
        let response_bytes = Arc::new(Mutex::new(Vec::new()));
        let response_colors = Arc::new(Mutex::new(Colors::default()));
        let emulator_title = Arc::new(Mutex::new(None));
        let window_size = Arc::new(Mutex::new(window_size_from_dimensions(dimensions)));
        let term = Term::new(
            Config::default(),
            &dimensions,
            EmulatorEventListener {
                bell_count: Arc::clone(&bell_count),
                response_bytes: Arc::clone(&response_bytes),
                response_colors: Arc::clone(&response_colors),
                emulator_title: Arc::clone(&emulator_title),
                window_size: Arc::clone(&window_size),
            },
        );

        Self {
            inner: Mutex::new(EmulatorState {
                term,
                parser: ansi::Processor::new(),
                bell_count,
                response_bytes,
                response_colors,
                emulator_title,
                window_size,
                working_directory_uri: None,
                user_variables: Default::default(),
                progress: ScreenProgress::default(),
                extra_sgr_tracker: ExtraSgrTracker::default(),
                extra_styles: ExtraTextStyleOverlay::default(),
                rectangular_attribute_tracker: TerminalRectangularAttributeTracker::default(),
                rectangular_area_tracker: TerminalRectangularAreaTracker::default(),
                scroll_region_tracker: TerminalScrollRegionTracker::default(),
                horizontal_margin_tracker: TerminalHorizontalMarginTracker::default(),
                column_control_tracker: TerminalColumnControlTracker::default(),
                protection_tracker: TerminalProtectionTracker::default(),
                protected_cells: TerminalProtectedCellOverlay::default(),
                metadata_tracker: TerminalMetadataSequenceTracker::default(),
                visual_passthrough_tracker: TerminalTmuxPassthroughVisualTracker::default(),
                color_sequence_tracker: TerminalColorSequenceTracker::default(),
                color_stack: TerminalColorStack::default(),
                synchronized_output_tracker: TerminalSynchronizedOutputTracker::default(),
                synchronized_output_snapshot: None,
                media_tracker: TerminalMediaSequenceTracker::default(),
                media_overlay: TerminalMediaOverlay::default(),
                side_effect_overlay: TerminalSideEffectOverlay::default(),
                semantic_mark_overlay: TerminalSemanticMarkOverlay::default(),
            }),
        }
    }

    pub(super) fn advance(&self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }

        if let Ok(mut state) = self.inner.lock() {
            for &byte in chunk {
                let synchronized_output_event = state.synchronized_output_tracker.advance(byte);
                if matches!(
                    synchronized_output_event.as_ref(),
                    Some(TerminalSynchronizedOutputEvent::Begin)
                ) && state.synchronized_output_snapshot.is_none()
                {
                    state.synchronized_output_snapshot = Some(render_emulator_state(&state, None));
                }
                push_synchronized_output_control_query_response(
                    &state,
                    synchronized_output_event.as_ref(),
                );
                let sync_ended = matches!(
                    synchronized_output_event.as_ref(),
                    Some(TerminalSynchronizedOutputEvent::End)
                );
                let visual_parser_byte = synchronized_output_visual_parser_byte(
                    synchronized_output_event.as_ref(),
                    byte,
                );
                advance_emulator_state_byte(&mut state, visual_parser_byte);
                if sync_ended {
                    state.synchronized_output_snapshot = None;
                }
            }
        }
    }

    pub(super) fn resize(&self, rows: u16, cols: u16) {
        if let Ok(mut state) = self.inner.lock() {
            let dimensions = TerminalDimensions::new(rows, cols);
            state.term.resize(dimensions);
            if let Ok(mut window_size) = state.window_size.lock() {
                *window_size = window_size_from_dimensions(dimensions);
            }
            state.extra_styles.clear();
            state.protected_cells.clear();
            state.media_overlay.clear();
            state.side_effect_overlay.clear();
            state.semantic_mark_overlay.clear();
            state.scroll_region_tracker = TerminalScrollRegionTracker::default();
            state.horizontal_margin_tracker = TerminalHorizontalMarginTracker::default();
            state.column_control_tracker = TerminalColumnControlTracker::default();
            state.synchronized_output_tracker = TerminalSynchronizedOutputTracker::default();
            state.synchronized_output_snapshot = None;
        }
    }

    pub(super) fn take_response_bytes(&self) -> Vec<Vec<u8>> {
        self.inner
            .lock()
            .ok()
            .and_then(|state| {
                state.response_bytes.lock().ok().map(|mut bytes| bytes.drain(..).collect())
            })
            .unwrap_or_default()
    }

    pub(super) fn bracketed_paste_enabled(&self) -> bool {
        self.inner
            .lock()
            .map(|state| state.term.mode().contains(TermMode::BRACKETED_PASTE))
            .unwrap_or(false)
    }

    pub(super) fn render(&self, title: Option<String>) -> RenderedEmulator {
        let Ok(state) = self.inner.lock() else {
            return RenderedEmulator {
                buffer_kind: ScreenBufferKind::Unknown,
                surface: ScreenSurface {
                    title,
                    working_directory_uri: None,
                    user_variables: Default::default(),
                    cursor: Some(ScreenCursor::at(0, 0)),
                    palette: Default::default(),
                    bell_count: 0,
                    progress: Default::default(),
                    lines: vec![ScreenLine::plain(String::new())],
                },
            };
        };

        state
            .synchronized_output_snapshot
            .as_ref()
            .map(|snapshot| synchronized_output_snapshot_with_title(snapshot, title.clone()))
            .unwrap_or_else(|| render_emulator_state(&state, title))
    }
}

fn advance_emulator_state_byte(state: &mut EmulatorState, byte: u8) {
    let EmulatorState {
        term,
        parser,
        bell_count: _,
        response_bytes,
        response_colors,
        emulator_title,
        window_size,
        working_directory_uri,
        user_variables,
        progress,
        extra_sgr_tracker,
        extra_styles,
        rectangular_attribute_tracker,
        rectangular_area_tracker,
        scroll_region_tracker,
        horizontal_margin_tracker,
        column_control_tracker,
        protection_tracker,
        protected_cells,
        metadata_tracker,
        visual_passthrough_tracker,
        color_sequence_tracker,
        color_stack,
        synchronized_output_tracker: _,
        synchronized_output_snapshot: _,
        media_tracker,
        media_overlay,
        side_effect_overlay,
        semantic_mark_overlay,
    } = state;
    let mut visual_parser = VisualParserContext {
        term,
        parser,
        response_colors,
        response_bytes,
        window_size,
        color_sequence_tracker,
        color_stack,
        extra_sgr_tracker,
        extra_styles,
        rectangular_attribute_tracker,
        rectangular_area_tracker,
        scroll_region_tracker,
        horizontal_margin_tracker,
        column_control_tracker,
        protection_tracker,
        protected_cells,
        media_overlay,
        side_effect_overlay,
        semantic_mark_overlay,
    };
    let visual_passthrough = visual_passthrough_tracker.advance(byte);
    if let Some(update) = metadata_tracker.advance(byte) {
        match update {
            TerminalMetadataUpdate::Title(title) => {
                if let Ok(mut current_title) = emulator_title.lock() {
                    *current_title = Some(title);
                }
            }
            TerminalMetadataUpdate::WorkingDirectoryUri(uri) => {
                *working_directory_uri = uri;
            }
            TerminalMetadataUpdate::UserVariable { key, value } => {
                user_variables.insert(key, value);
            }
            TerminalMetadataUpdate::Progress(next_progress) => {
                *progress = next_progress;
            }
            TerminalMetadataUpdate::CursorShape(shape) => {
                ansi::Handler::set_cursor_shape(visual_parser.term, shape);
            }
            TerminalMetadataUpdate::SideEffect(side_effect) => {
                if let Some(row) = terminal_cursor_row(visual_parser.term) {
                    visual_parser.side_effect_overlay.push(row, side_effect);
                }
            }
            TerminalMetadataUpdate::SemanticMark(mut mark) => {
                if let Some((row, col)) = terminal_cursor_position(visual_parser.term) {
                    mark.col = col.min(usize::from(u16::MAX)) as u16;
                    visual_parser.semantic_mark_overlay.push(row, mark);
                }
            }
        }
    }
    let media_event = media_tracker.advance(byte);
    advance_visual_parser_byte(byte, &mut visual_parser);
    if let Some(decoded) = visual_passthrough {
        for decoded_byte in decoded {
            advance_visual_parser_byte(decoded_byte, &mut visual_parser);
        }
    }
    if let Some(event) = media_event {
        if let Some(response) = event.response {
            push_response_bytes(response_bytes, response);
        }
        if let Some(media) = event.media
            && let Some(row) = terminal_cursor_row(visual_parser.term)
        {
            visual_parser.media_overlay.push(row, media);
        }
    }
}

fn synchronized_output_snapshot_with_title(
    snapshot: &RenderedEmulator,
    title: Option<String>,
) -> RenderedEmulator {
    let mut snapshot = snapshot.clone();
    if snapshot.surface.title.is_none() {
        snapshot.surface.title = title;
    }
    snapshot
}

fn render_emulator_state(state: &EmulatorState, title: Option<String>) -> RenderedEmulator {
    let content = state.term.renderable_content();
    let colors = state.term.colors();
    let rows = state.term.screen_lines();
    let cols = state.term.columns();
    let resolved_title = state
        .emulator_title
        .lock()
        .ok()
        .and_then(|emulator_title| emulator_title.clone())
        .or(title);
    let working_directory_uri = state.working_directory_uri.clone();
    let user_variables = state.user_variables.clone();
    let progress = state.progress.clone();
    let mut lines = (0..rows).map(|_| RichScreenLineBuilder::default()).collect::<Vec<_>>();

    for indexed in content.display_iter {
        let Ok(row) = usize::try_from(indexed.point.line.0) else {
            continue;
        };
        if row >= rows {
            continue;
        }
        let col = indexed.point.column.0;
        if col >= cols {
            continue;
        }

        let extra_style = state.extra_styles.style_for_cell(row, col, indexed.cell.c);
        lines[row].push_cell_at_col(col, indexed.cell, extra_style, colors);
    }
    for (row, line) in lines.iter_mut().enumerate().take(rows) {
        line.push_media(state.media_overlay.items_for_row(row));
        line.push_side_effects(state.side_effect_overlay.items_for_row(row));
        line.push_semantic_marks(state.semantic_mark_overlay.items_for_row(row));
    }

    let rendered_lines = lines.into_iter().map(RichScreenLineBuilder::finish).collect();
    let buffer_kind = if state.term.mode().contains(TermMode::ALT_SCREEN) {
        ScreenBufferKind::Alternate
    } else {
        ScreenBufferKind::Normal
    };
    let cursor_style = state.term.cursor_style();
    let cursor = usize::try_from(content.cursor.point.line.0).ok().map(|row| ScreenCursor {
        row: row as u16,
        col: content.cursor.point.column.0 as u16,
        shape: Some(screen_cursor_shape_from_alacritty(content.cursor.shape)),
        blinking: content.cursor.shape != AlacrittyCursorShape::Hidden && cursor_style.blinking,
    });

    RenderedEmulator {
        buffer_kind,
        surface: ScreenSurface {
            title: resolved_title,
            working_directory_uri,
            user_variables,
            cursor,
            palette: screen_surface_palette_from_colors(colors),
            bell_count: state.bell_count.load(Ordering::SeqCst),
            progress,
            lines: rendered_lines,
        },
    }
}

struct VisualParserContext<'a> {
    term: &'a mut Term<EmulatorEventListener>,
    parser: &'a mut ansi::Processor,
    response_colors: &'a Arc<Mutex<Colors>>,
    response_bytes: &'a Arc<Mutex<Vec<Vec<u8>>>>,
    window_size: &'a Arc<Mutex<WindowSize>>,
    color_sequence_tracker: &'a mut TerminalColorSequenceTracker,
    color_stack: &'a mut TerminalColorStack,
    extra_sgr_tracker: &'a mut ExtraSgrTracker,
    extra_styles: &'a mut ExtraTextStyleOverlay,
    rectangular_attribute_tracker: &'a mut TerminalRectangularAttributeTracker,
    rectangular_area_tracker: &'a mut TerminalRectangularAreaTracker,
    scroll_region_tracker: &'a mut TerminalScrollRegionTracker,
    horizontal_margin_tracker: &'a mut TerminalHorizontalMarginTracker,
    column_control_tracker: &'a mut TerminalColumnControlTracker,
    protection_tracker: &'a mut TerminalProtectionTracker,
    protected_cells: &'a mut TerminalProtectedCellOverlay,
    media_overlay: &'a mut TerminalMediaOverlay,
    side_effect_overlay: &'a mut TerminalSideEffectOverlay,
    semantic_mark_overlay: &'a mut TerminalSemanticMarkOverlay,
}

fn advance_visual_parser_byte(byte: u8, context: &mut VisualParserContext<'_>) {
    let color_operations = context.color_sequence_tracker.advance(byte);
    if let Some(translated) = terminal_c1_control_translation(byte) {
        for &translated_byte in translated {
            advance_visual_parser_raw_byte(translated_byte, context);
        }
    } else {
        advance_visual_parser_raw_byte(byte, context);
    }
    if let Some(operations) = color_operations {
        apply_terminal_color_sequence(
            context.term,
            context.response_colors,
            context.response_bytes,
            context.color_stack,
            operations,
        );
    }
}

fn apply_terminal_color_sequence(
    term: &mut Term<EmulatorEventListener>,
    response_colors: &Arc<Mutex<Colors>>,
    response_bytes: &Arc<Mutex<Vec<Vec<u8>>>>,
    color_stack: &mut TerminalColorStack,
    operations: Vec<TerminalColorSequenceOperation>,
) {
    for operation in operations {
        match operation {
            TerminalColorSequenceOperation::Update(update) => {
                ansi::Handler::set_color(term, update.index, update.color);
            }
            TerminalColorSequenceOperation::Reset(index) => {
                ansi::Handler::reset_color(term, index);
            }
            TerminalColorSequenceOperation::Query(query) => {
                if let Some(color) = terminal_response_rgb_for_index(query.index, term.colors()) {
                    push_response_bytes(
                        response_bytes,
                        terminal_color_query_response_bytes(query, color),
                    );
                }
            }
            TerminalColorSequenceOperation::KittyQuery(query) => {
                push_response_bytes(
                    response_bytes,
                    terminal_kitty_color_query_response_bytes(query, term.colors()),
                );
            }
            TerminalColorSequenceOperation::PushColors => {
                color_stack.push(*term.colors());
            }
            TerminalColorSequenceOperation::PopColors => {
                if let Some(colors) = color_stack.pop() {
                    restore_terminal_colors(term, colors);
                }
            }
            TerminalColorSequenceOperation::StoreColors(slot) => {
                color_stack.store(slot, *term.colors());
            }
            TerminalColorSequenceOperation::RestoreColors(slot) => {
                if let Some(colors) = color_stack.restore(slot) {
                    restore_terminal_colors(term, colors);
                }
            }
            TerminalColorSequenceOperation::ReportColors => {
                push_response_bytes(response_bytes, color_stack.report_response_bytes());
            }
        }
    }
    if let Ok(mut colors) = response_colors.lock() {
        *colors = *term.colors();
    }
}

fn restore_terminal_colors(term: &mut Term<EmulatorEventListener>, colors: Colors) {
    for index in 0..ALACRITTY_COLOR_COUNT {
        if let Some(color) = colors[index] {
            ansi::Handler::set_color(term, index, color);
        } else {
            ansi::Handler::reset_color(term, index);
        }
    }
}

fn terminal_c1_control_translation(byte: u8) -> Option<&'static [u8]> {
    match byte {
        0x84 => Some(b"\x1bD"),
        0x85 => Some(b"\x1bE"),
        0x88 => Some(b"\x1bH"),
        0x8d => Some(b"\x1bM"),
        0x8e => Some(b"\x1bN"),
        0x8f => Some(b"\x1bO"),
        0x96 => Some(b"\x1bV"),
        0x97 => Some(b"\x1bW"),
        0x98 => Some(b"\x1bX"),
        0x90 => Some(b"\x1bP"),
        0x9a => Some(b"\x1bZ"),
        0x9b => Some(b"\x1b["),
        0x9c => Some(b"\x1b\\"),
        0x9d => Some(b"\x1b]"),
        0x9e => Some(b"\x1b^"),
        0x9f => Some(b"\x1b_"),
        _ => None,
    }
}

fn advance_visual_parser_raw_byte(byte: u8, context: &mut VisualParserContext<'_>) {
    context.scroll_region_tracker.advance(byte, context.term.screen_lines());
    context.horizontal_margin_tracker.advance(byte, context.term.columns());
    let column_control = context.column_control_tracker.advance(byte);
    let protection_control = context.protection_tracker.advance(byte);
    let rectangular_area_control = context.rectangular_area_tracker.advance(byte);
    let protected_snapshot = protection_control
        .filter(|control| control.is_selective_erase())
        .map(|_| context.protected_cells.clone());
    if let Some(control) = context.extra_sgr_tracker.advance(byte) {
        match control {
            ExtraSgrControl::Update(update) => context.extra_styles.apply_sgr_update(update),
            ExtraSgrControl::Push(attributes) => context.extra_styles.push_sgr_state(attributes),
            ExtraSgrControl::Pop => context.extra_styles.pop_sgr_state(),
        }
    }
    if let Some(control) = context.rectangular_attribute_tracker.advance(byte) {
        context.extra_styles.apply_rectangular_attributes(context.term, control);
    }
    if let Ok(mut colors) = context.response_colors.lock() {
        *colors = *context.term.colors();
    }
    {
        let mut handler = ExtraTextStyleHandler {
            term: context.term,
            response_bytes: context.response_bytes,
            window_size: context.window_size,
            horizontal_margin_mode: context.horizontal_margin_tracker.mode,
            extra_styles: context.extra_styles,
            protected_cells: context.protected_cells,
            media_overlay: context.media_overlay,
            side_effect_overlay: context.side_effect_overlay,
            semantic_mark_overlay: context.semantic_mark_overlay,
        };
        context.parser.advance(&mut handler, &[byte]);
    }
    if let Some(control) = rectangular_area_control {
        apply_terminal_rectangular_area_control(
            context.term,
            context.extra_styles,
            context.protected_cells,
            control,
        );
    }
    if let Some(control) = column_control {
        apply_terminal_column_control(
            context.term,
            context.extra_styles,
            context.protected_cells,
            context.scroll_region_tracker.active_region(context.term.screen_lines()),
            context.horizontal_margin_tracker.active_region(context.term.columns()),
            control,
        );
    }
    if let Some(control) = protection_control {
        match control {
            TerminalProtectionControl::ProtectedMode(protected) => {
                context.protected_cells.set_protected_mode(protected);
            }
            TerminalProtectionControl::SelectiveLineErase(mode) => {
                if let Some(snapshot) = protected_snapshot {
                    *context.protected_cells = snapshot;
                }
                context.protected_cells.apply_selective_line_erase(
                    context.term,
                    context.extra_styles,
                    mode,
                );
            }
            TerminalProtectionControl::SelectiveDisplayErase(mode) => {
                if let Some(snapshot) = protected_snapshot {
                    *context.protected_cells = snapshot;
                }
                context.protected_cells.apply_selective_display_erase(
                    context.term,
                    context.extra_styles,
                    mode,
                );
            }
        }
    }
}

#[derive(Clone)]
struct EmulatorEventListener {
    bell_count: Arc<AtomicU64>,
    response_bytes: Arc<Mutex<Vec<Vec<u8>>>>,
    response_colors: Arc<Mutex<Colors>>,
    emulator_title: Arc<Mutex<Option<String>>>,
    window_size: Arc<Mutex<WindowSize>>,
}

impl EventListener for EmulatorEventListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::Bell => {
                self.bell_count.fetch_add(1, Ordering::SeqCst);
            }
            Event::Title(title) => {
                if let Ok(mut current_title) = self.emulator_title.lock() {
                    *current_title = Some(title);
                }
            }
            Event::ResetTitle => {
                if let Ok(mut current_title) = self.emulator_title.lock() {
                    *current_title = None;
                }
            }
            Event::PtyWrite(text) => self.push_response(text.into_bytes()),
            Event::ColorRequest(index, formatter) => {
                if let Ok(colors) = self.response_colors.lock()
                    && let Some(color) = terminal_response_rgb_for_index(index, &colors)
                {
                    self.push_response(formatter(color).into_bytes());
                }
            }
            Event::TextAreaSizeRequest(formatter) => {
                if let Ok(window_size) = self.window_size.lock() {
                    self.push_response(formatter(*window_size).into_bytes());
                }
            }
            _ => {}
        }
    }
}

impl EmulatorEventListener {
    fn push_response(&self, response: Vec<u8>) {
        push_response_bytes(&self.response_bytes, response);
    }
}

fn push_response_bytes(response_bytes: &Arc<Mutex<Vec<Vec<u8>>>>, response: Vec<u8>) {
    if response.is_empty() {
        return;
    }
    if let Ok(mut response_bytes) = response_bytes.lock()
        && response_bytes.len() < 64
    {
        response_bytes.push(response);
    }
}

fn window_size_from_dimensions(dimensions: TerminalDimensions) -> WindowSize {
    WindowSize {
        num_lines: dimensions.rows as u16,
        num_cols: dimensions.cols as u16,
        cell_width: FALLBACK_TERMINAL_CELL_WIDTH_PX,
        cell_height: FALLBACK_TERMINAL_CELL_HEIGHT_PX,
    }
}

fn terminal_pixel_size_response_bytes(response_code: u8, window_size: WindowSize) -> Vec<u8> {
    format!(
        "\x1b[{};{};{}t",
        response_code,
        u32::from(window_size.num_lines) * u32::from(window_size.cell_height),
        u32::from(window_size.num_cols) * u32::from(window_size.cell_width)
    )
    .into_bytes()
}

fn terminal_char_size_response_bytes(response_code: u8, window_size: WindowSize) -> Vec<u8> {
    format!("\x1b[{};{};{}t", response_code, window_size.num_lines, window_size.num_cols)
        .into_bytes()
}

fn terminal_cell_size_response_bytes(window_size: WindowSize) -> Vec<u8> {
    format!("\x1b[6;{};{}t", window_size.cell_height, window_size.cell_width).into_bytes()
}

fn terminal_iterm2_cell_size_response_bytes(window_size: WindowSize) -> Vec<u8> {
    format!(
        "\x1b]1337;ReportCellSize={:.2};{:.2};1.0\x1b\\",
        f32::from(window_size.cell_height),
        f32::from(window_size.cell_width)
    )
    .into_bytes()
}

fn terminal_primary_device_attributes_response_bytes() -> Vec<u8> {
    b"\x1b[?6c".to_vec()
}

fn terminal_secondary_device_attributes_response_bytes() -> Vec<u8> {
    format!("\x1b[>0;{};1c", ALACRITTY_TERMINAL_SECONDARY_DEVICE_ATTRIBUTES_VERSION).into_bytes()
}

fn terminal_xterm_version_response_bytes() -> Vec<u8> {
    format!("\x1bP>|{}({})\x1b\\", TERMINAL_PLATFORM_PROGRAM_NAME, env!("CARGO_PKG_VERSION"))
        .into_bytes()
}

fn terminal_feature_report_response_bytes() -> Vec<u8> {
    format!("\x1b]1337;Capabilities={}\x1b\\", crate::TERMINAL_FEATURE_REPORT).into_bytes()
}

fn terminal_decrqss_valid_response_bytes(value: String) -> Vec<u8> {
    format!("\x1bP1$r{value}\x1b\\").into_bytes()
}

fn terminal_sgr_status_string_response_bytes(style: &ExtraTextStyle) -> Vec<u8> {
    terminal_decrqss_valid_response_bytes(terminal_sgr_status_string(style))
}

fn terminal_xtgettcap_response_bytes(query: &TerminalTermcapQuery) -> Vec<u8> {
    query.names.iter().flat_map(|name| terminal_xtgettcap_single_response_bytes(name)).collect()
}

fn terminal_xtgettcap_single_response_bytes(name: &[u8]) -> Vec<u8> {
    let name_hex = encode_terminal_hex_bytes(name);
    if let Some(value) = terminal_xtgettcap_capability_value(name) {
        return match value {
            TerminalTermcapCapabilityValue::Boolean => {
                format!("\x1bP1+r{name_hex}\x1b\\").into_bytes()
            }
            TerminalTermcapCapabilityValue::String(value) => {
                format!("\x1bP1+r{}={}\x1b\\", name_hex, encode_terminal_hex_bytes(value))
                    .into_bytes()
            }
        };
    }
    format!("\x1bP0+r{name_hex}\x1b\\").into_bytes()
}

enum TerminalTermcapCapabilityValue {
    Boolean,
    String(&'static [u8]),
}

fn terminal_xtgettcap_capability_value(name: &[u8]) -> Option<TerminalTermcapCapabilityValue> {
    match name {
        b"RGB" => Some(TerminalTermcapCapabilityValue::String(b"8")),
        b"CO" => Some(TerminalTermcapCapabilityValue::String(b"8")),
        b"Tc" => Some(TerminalTermcapCapabilityValue::Boolean),
        b"setrgbf" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[38;2;%p1%d;%p2%d;%p3%dm")),
        b"setrgbb" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[48;2;%p1%d;%p2%d;%p3%dm")),
        b"setf24" => Some(TerminalTermcapCapabilityValue::String(
            b"\x1b[38;2;%p1%{65536}%/%d;%p1%{256}%/%{255}%&%d;%p1%{255}%&%dm",
        )),
        b"setb24" => Some(TerminalTermcapCapabilityValue::String(
            b"\x1b[48;2;%p1%{65536}%/%d;%p1%{256}%/%{255}%&%d;%p1%{255}%&%dm",
        )),
        b"setaf" => Some(TerminalTermcapCapabilityValue::String(
            b"\x1b[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m",
        )),
        b"setab" => Some(TerminalTermcapCapabilityValue::String(
            b"\x1b[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m",
        )),
        b"op" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[39;49m")),
        b"initc" => Some(TerminalTermcapCapabilityValue::String(
            b"\x1b]4;%p1%d;rgb:%p2%{255}%*%{1000}%/%2.2X/%p3%{255}%*%{1000}%/%2.2X/%p4%{255}%*%{1000}%/%2.2X\x1b\\",
        )),
        b"oc" => Some(TerminalTermcapCapabilityValue::String(b"\x1b]104\x1b\\")),
        b"Smulx" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[4::%p1%dm")),
        b"Setulc" => Some(TerminalTermcapCapabilityValue::String(
            b"\x1b[58::2::%p1%{65536}%/%d::%p1%{256}%/%{255}%&%d::%p1%{255}%&%d%;m",
        )),
        b"Setulc1" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[58::5::%p1%dm")),
        b"ol" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[59m")),
        b"smul" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[4m")),
        b"rmul" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[24m")),
        b"sitm" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[3m")),
        b"ritm" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[23m")),
        b"smxx" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[9m")),
        b"rmxx" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[29m")),
        b"Smol" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[53m")),
        b"Rmol" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[55m")),
        b"smacs" => Some(TerminalTermcapCapabilityValue::String(b"\x1b(0")),
        b"rmacs" => Some(TerminalTermcapCapabilityValue::String(b"\x1b(B")),
        b"acsc" => Some(TerminalTermcapCapabilityValue::String(
            b"``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~",
        )),
        b"Ss" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[%p1%d q")),
        b"Se" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[2 q")),
        b"Cs" => Some(TerminalTermcapCapabilityValue::String(b"\x1b]12;%p1%s\x1b\\")),
        b"Cr" => Some(TerminalTermcapCapabilityValue::String(b"\x1b]112\x1b\\")),
        b"Hls" => {
            Some(TerminalTermcapCapabilityValue::String(b"\x1b]8;%?%p1%l%tid=%p1%s%;;%p2%s\x1b\\"))
        }
        b"Swd" => Some(TerminalTermcapCapabilityValue::String(b"\x1b]7;")),
        b"Spb" => Some(TerminalTermcapCapabilityValue::String(b"\x1b]9;4;%p1%d;%p2%d\x1b\\")),
        b"tsl" => Some(TerminalTermcapCapabilityValue::String(b"\x1b]0;")),
        b"fsl" => Some(TerminalTermcapCapabilityValue::String(b"\x07")),
        b"bold" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[1m")),
        b"sgr0" => Some(TerminalTermcapCapabilityValue::String(b"\x1b(B\x1b[0m")),
        b"dim" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[2m")),
        b"blink" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[5m")),
        b"rev" | b"smso" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[7m")),
        b"rmso" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[27m")),
        b"invis" => Some(TerminalTermcapCapabilityValue::String(b"\x1b[8m")),
        b"Co" | b"colors" => Some(TerminalTermcapCapabilityValue::String(b"256")),
        b"pairs" => Some(TerminalTermcapCapabilityValue::String(b"65536")),
        b"TN" => Some(TerminalTermcapCapabilityValue::String(b"xterm-256color")),
        b"AX" | b"XT" | b"bce" | b"ccc" | b"mir" | b"msgr" | b"am" | b"xenl" => {
            Some(TerminalTermcapCapabilityValue::Boolean)
        }
        _ => None,
    }
}

fn encode_terminal_hex_bytes(value: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn terminal_sgr_status_string(style: &ExtraTextStyle) -> String {
    let mut params = Vec::<String>::new();

    if style.bold == Some(true) {
        params.push("1".to_string());
    }
    if style.dim == Some(true) {
        params.push("2".to_string());
    }
    if style.italic == Some(true) {
        params.push("3".to_string());
    }
    if let Some(Some(underline)) = style.underline {
        params.push(terminal_underline_sgr_parameter(underline).to_string());
    }
    if style.blink {
        params.push("5".to_string());
    }
    if style.inverse == Some(true) {
        params.push("7".to_string());
    }
    if style.hidden == Some(true) {
        params.push("8".to_string());
    }
    if style.strikethrough == Some(true) {
        params.push("9".to_string());
    }
    if let Some(Some(color)) = style.foreground.as_ref() {
        terminal_sgr_color_parameters(&mut params, 38, color);
    }
    if let Some(Some(color)) = style.background.as_ref() {
        terminal_sgr_color_parameters(&mut params, 48, color);
    }
    if let Some(Some(color)) = style.underline_color.as_ref() {
        terminal_sgr_color_parameters(&mut params, 58, color);
    }
    if style.overline {
        params.push("53".to_string());
    }
    if let Some(border) = style.border {
        params.push(terminal_border_sgr_parameter(border).to_string());
    }
    if let Some(baseline) = style.baseline {
        params.push(terminal_baseline_sgr_parameter(baseline).to_string());
    }

    if params.is_empty() { "0m".to_string() } else { format!("{}m", params.join(";")) }
}

fn terminal_underline_sgr_parameter(underline: ScreenUnderlineStyle) -> &'static str {
    match underline {
        ScreenUnderlineStyle::Single => "4",
        ScreenUnderlineStyle::Double => "21",
        ScreenUnderlineStyle::Curly => "4:3",
        ScreenUnderlineStyle::Dotted => "4:4",
        ScreenUnderlineStyle::Dashed => "4:5",
    }
}

fn terminal_border_sgr_parameter(border: ScreenTextBorderStyle) -> &'static str {
    match border {
        ScreenTextBorderStyle::Framed => "51",
        ScreenTextBorderStyle::Encircled => "52",
    }
}

fn terminal_baseline_sgr_parameter(baseline: ScreenTextBaseline) -> &'static str {
    match baseline {
        ScreenTextBaseline::Superscript => "73",
        ScreenTextBaseline::Subscript => "74",
    }
}

fn terminal_sgr_color_parameters(params: &mut Vec<String>, target: u16, color: &ScreenColor) {
    match color {
        ScreenColor::Named { name } => {
            if let Some(index) = native_named_sgr_index(name) {
                match target {
                    38 => params.push(native_named_foreground_sgr_parameter(index).to_string()),
                    48 => params.push(native_named_background_sgr_parameter(index).to_string()),
                    58 => {
                        params.push("58".to_string());
                        params.push("5".to_string());
                        params.push(index.to_string());
                    }
                    _ => {}
                }
            }
        }
        ScreenColor::Indexed { index } => {
            params.push(target.to_string());
            params.push("5".to_string());
            params.push(index.to_string());
        }
        ScreenColor::Rgb { r, g, b } => {
            params.push(target.to_string());
            params.push("2".to_string());
            params.push(r.to_string());
            params.push(g.to_string());
            params.push(b.to_string());
        }
    }
}

fn native_named_foreground_sgr_parameter(index: u8) -> u16 {
    if index < 8 { 30 + u16::from(index) } else { 90 + u16::from(index - 8) }
}

fn native_named_background_sgr_parameter(index: u8) -> u16 {
    if index < 8 { 40 + u16::from(index) } else { 100 + u16::from(index - 8) }
}

fn terminal_response_rgb_for_index(index: usize, colors: &Colors) -> Option<Rgb> {
    if index >= ALACRITTY_COLOR_COUNT {
        return None;
    }

    colors[index].or_else(|| default_terminal_response_rgb(index))
}

fn default_terminal_response_rgb(index: usize) -> Option<Rgb> {
    match index {
        0 => Some(rgb(0x11, 0x18, 0x27)),
        1 => Some(rgb(0xef, 0x44, 0x44)),
        2 => Some(rgb(0x22, 0xc5, 0x5e)),
        3 => Some(rgb(0xea, 0xb3, 0x08)),
        4 => Some(rgb(0x3b, 0x82, 0xf6)),
        5 => Some(rgb(0xa8, 0x55, 0xf7)),
        6 => Some(rgb(0x06, 0xb6, 0xd4)),
        7 => Some(rgb(0xe5, 0xe7, 0xeb)),
        8 => Some(rgb(0x6b, 0x72, 0x80)),
        9 => Some(rgb(0xf8, 0x71, 0x71)),
        10 => Some(rgb(0x4a, 0xde, 0x80)),
        11 => Some(rgb(0xfa, 0xcc, 0x15)),
        12 => Some(rgb(0x60, 0xa5, 0xfa)),
        13 => Some(rgb(0xc0, 0x84, 0xfc)),
        14 => Some(rgb(0x22, 0xd3, 0xee)),
        15 => Some(rgb(0xf9, 0xfa, 0xfb)),
        16..=231 => {
            let offset = index - 16;
            let steps = [0, 95, 135, 175, 215, 255];
            Some(rgb(steps[offset / 36], steps[(offset % 36) / 6], steps[offset % 6]))
        }
        232..=255 => {
            let gray = 8 + ((index - 232) as u8) * 10;
            Some(rgb(gray, gray, gray))
        }
        256 => Some(rgb(0xe8, 0xed, 0xf6)),
        257 => Some(rgb(0x05, 0x07, 0x0b)),
        258 => Some(rgb(0x7d, 0xd3, 0xfc)),
        259 => Some(rgb(0x03, 0x07, 0x12)),
        260 => Some(rgb(0x99, 0x1b, 0x1b)),
        261 => Some(rgb(0x16, 0x65, 0x34)),
        262 => Some(rgb(0x85, 0x4d, 0x0e)),
        263 => Some(rgb(0x1d, 0x4e, 0xd8)),
        264 => Some(rgb(0x7e, 0x22, 0xce)),
        265 => Some(rgb(0x0e, 0x74, 0x90)),
        266 => Some(rgb(0x9c, 0xa3, 0xaf)),
        267 => Some(rgb(0xe8, 0xed, 0xf6)),
        268 => Some(rgb(0x05, 0x07, 0x0b)),
        _ => None,
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

fn terminal_cursor_row(term: &Term<EmulatorEventListener>) -> Option<usize> {
    terminal_cursor_position(term).map(|(row, _)| row)
}

fn terminal_cursor_position(term: &Term<EmulatorEventListener>) -> Option<(usize, usize)> {
    let content = term.renderable_content();
    let row = usize::try_from(content.cursor.point.line.0).ok()?;
    Some((row, content.cursor.point.column.0))
}

#[derive(Default)]
struct RichScreenLineBuilder {
    text: String,
    spans: Vec<ScreenLineSpan>,
    media: Vec<ScreenLineMedia>,
    side_effects: Vec<ScreenLineSideEffect>,
    semantic_marks: Vec<ScreenLineSemanticMark>,
    cols: usize,
    wrapped: bool,
    last_style: ScreenTextStyle,
}

impl RichScreenLineBuilder {
    fn push_cell_at_col(
        &mut self,
        col: usize,
        cell: &Cell,
        extra_style: ExtraTextStyle,
        colors: &Colors,
    ) {
        if cell.flags.contains(Flags::WRAPLINE) {
            self.wrapped = true;
        }
        self.pad_to_col(col);
        if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
            return;
        }

        self.push_cell(cell, extra_style, colors);
        if let Some(zerowidth) = cell.zerowidth() {
            for ch in zerowidth {
                self.push_zerowidth(*ch);
            }
        }
    }

    fn pad_to_col(&mut self, col: usize) {
        while self.cols < col {
            self.push_text(" ", ScreenTextStyle::default(), 1);
        }
    }

    fn push_cell(&mut self, cell: &Cell, extra_style: ExtraTextStyle, colors: &Colors) {
        let style = screen_text_style_from_cell(cell, extra_style, colors);
        let width = if cell.flags.contains(Flags::WIDE_CHAR) { 2 } else { 1 };
        let mut text = String::new();
        text.push(cell.c);
        self.push_text(&text, style, width);
    }

    fn push_zerowidth(&mut self, ch: char) {
        let mut text = String::new();
        text.push(ch);
        self.text.push(ch);
        if let Some(span) = self.spans.last_mut() {
            span.text.push(ch);
        } else {
            self.spans.push(ScreenLineSpan { text, style: self.last_style.clone() });
        }
    }

    fn push_text(&mut self, text: &str, style: ScreenTextStyle, cols: usize) {
        self.text.push_str(text);
        if let Some(span) = self.spans.last_mut()
            && span.style == style
        {
            span.text.push_str(text);
        } else {
            self.spans.push(ScreenLineSpan { text: text.to_string(), style: style.clone() });
            self.last_style = style;
        }
        self.cols = self.cols.saturating_add(cols);
    }

    fn push_media(&mut self, media: &[ScreenLineMedia]) {
        self.media.extend_from_slice(media);
    }

    fn push_side_effects(&mut self, side_effects: &[ScreenLineSideEffect]) {
        self.side_effects.extend_from_slice(side_effects);
    }

    fn push_semantic_marks(&mut self, semantic_marks: &[ScreenLineSemanticMark]) {
        self.semantic_marks.extend_from_slice(semantic_marks);
    }

    fn finish(mut self) -> ScreenLine {
        trim_rich_line_end(&mut self.text, &mut self.spans);
        if self.spans.iter().all(|span| span.style.is_plain()) {
            self.spans.clear();
        }
        ScreenLine {
            text: self.text,
            spans: self.spans,
            media: self.media,
            side_effects: self.side_effects,
            semantic_marks: self.semantic_marks,
            wrapped: self.wrapped,
        }
    }
}

#[derive(Debug)]
struct TerminalLineOverlay<T> {
    rows: HashMap<usize, Vec<T>>,
}

type TerminalMediaOverlay = TerminalLineOverlay<ScreenLineMedia>;
type TerminalSideEffectOverlay = TerminalLineOverlay<ScreenLineSideEffect>;
type TerminalSemanticMarkOverlay = TerminalLineOverlay<ScreenLineSemanticMark>;

impl<T> Default for TerminalLineOverlay<T> {
    fn default() -> Self {
        Self { rows: HashMap::new() }
    }
}

impl<T> TerminalLineOverlay<T> {
    fn clear(&mut self) {
        self.rows.clear();
    }

    fn clear_row(&mut self, row: usize) {
        self.rows.remove(&row);
    }

    fn clear_rows_above(&mut self, row: usize) {
        self.rows.retain(|key, _| *key >= row);
    }

    fn clear_rows_below(&mut self, row: usize) {
        self.rows.retain(|key, _| *key <= row);
    }

    fn push(&mut self, row: usize, item: T) {
        let row_items = self.rows.entry(row).or_default();
        if row_items.len() < 16 {
            row_items.push(item);
        }
    }

    fn items_for_row(&self, row: usize) -> &[T] {
        self.rows.get(&row).map(Vec::as_slice).unwrap_or_default()
    }
}

impl TerminalLineOverlay<ScreenLineSemanticMark> {
    fn clear_mark_range(&mut self, row: usize, start_col: usize, end_col: usize) {
        if start_col >= end_col {
            return;
        }
        if let Some(marks) = self.rows.get_mut(&row) {
            marks.retain(|mark| {
                let col = usize::from(mark.col);
                col < start_col || col >= end_col
            });
        }
        if self.rows.get(&row).is_some_and(Vec::is_empty) {
            self.rows.remove(&row);
        }
    }
}

#[derive(Debug, Default)]
struct TerminalMetadataSequenceTracker {
    state: TerminalMetadataSequenceState,
}

#[derive(Debug, Default)]
enum TerminalMetadataSequenceState {
    #[default]
    Ground,
    Escape,
    Osc {
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
    Dcs {
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum TerminalMetadataUpdate {
    Title(String),
    WorkingDirectoryUri(Option<String>),
    UserVariable { key: String, value: String },
    Progress(ScreenProgress),
    CursorShape(AlacrittyCursorShape),
    SideEffect(ScreenLineSideEffect),
    SemanticMark(ScreenLineSemanticMark),
}

impl TerminalMetadataSequenceTracker {
    const MAX_OSC_METADATA_BYTES: usize = 2048;

    fn advance(&mut self, byte: u8) -> Option<TerminalMetadataUpdate> {
        match &mut self.state {
            TerminalMetadataSequenceState::Ground => {
                self.state = match byte {
                    0x1b => TerminalMetadataSequenceState::Escape,
                    0x90 => TerminalMetadataSequenceState::Dcs {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x9d => TerminalMetadataSequenceState::Osc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    _ => TerminalMetadataSequenceState::Ground,
                };
                None
            }
            TerminalMetadataSequenceState::Escape => {
                self.state = match byte {
                    b']' => TerminalMetadataSequenceState::Osc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    b'P' => TerminalMetadataSequenceState::Dcs {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x1b => TerminalMetadataSequenceState::Escape,
                    _ => TerminalMetadataSequenceState::Ground,
                };
                None
            }
            TerminalMetadataSequenceState::Osc { payload, saw_escape, truncated } => {
                if byte == 0x9c {
                    let update = terminal_metadata_update_from_osc(payload, *truncated);
                    self.state = TerminalMetadataSequenceState::Ground;
                    return update;
                }
                if *saw_escape {
                    let update = (byte == b'\\')
                        .then(|| terminal_metadata_update_from_osc(payload, *truncated));
                    self.state = TerminalMetadataSequenceState::Ground;
                    return update.flatten();
                }
                if byte == 0x07 {
                    let update = terminal_metadata_update_from_osc(payload, *truncated);
                    self.state = TerminalMetadataSequenceState::Ground;
                    return update;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                } else if payload.len() < Self::MAX_OSC_METADATA_BYTES {
                    payload.push(byte);
                } else {
                    *truncated = true;
                }
                None
            }
            TerminalMetadataSequenceState::Dcs { payload, saw_escape, truncated } => {
                if byte == 0x9c {
                    let update = terminal_metadata_update_from_tmux_dcs(payload, *truncated);
                    self.state = TerminalMetadataSequenceState::Ground;
                    return update;
                }
                if *saw_escape {
                    if byte == b'\\' {
                        let update = terminal_metadata_update_from_tmux_dcs(payload, *truncated);
                        self.state = TerminalMetadataSequenceState::Ground;
                        return update;
                    }
                    if byte == 0x1b {
                        if payload.len() < Self::MAX_OSC_METADATA_BYTES {
                            payload.push(0x1b);
                        } else {
                            *truncated = true;
                        }
                    } else {
                        if payload.len() + 2 <= Self::MAX_OSC_METADATA_BYTES {
                            payload.push(0x1b);
                            payload.push(byte);
                        } else {
                            *truncated = true;
                        }
                    }
                    *saw_escape = false;
                    return None;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                } else if payload.len() < Self::MAX_OSC_METADATA_BYTES {
                    payload.push(byte);
                } else {
                    *truncated = true;
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalColorUpdate {
    index: usize,
    color: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalColorQuery {
    index: usize,
    response_index: TerminalColorResponseIndex,
    terminator: TerminalColorSequenceTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TerminalKittyColorQuery {
    key: String,
    index: Option<usize>,
    terminator: TerminalColorSequenceTerminator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalColorResponseIndex {
    Numeric(usize),
    ItermForeground,
    ItermBackground,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TerminalColorSequenceOperation {
    Update(TerminalColorUpdate),
    Reset(usize),
    Query(TerminalColorQuery),
    KittyQuery(TerminalKittyColorQuery),
    PushColors,
    PopColors,
    StoreColors(usize),
    RestoreColors(usize),
    ReportColors,
}

#[derive(Default)]
struct TerminalColorStack {
    palettes: Vec<Colors>,
    used: usize,
    last: usize,
}

impl TerminalColorStack {
    fn push(&mut self, colors: Colors) {
        if self.palettes.len() >= MAX_COLOR_STACK_DEPTH {
            self.palettes.remove(0);
            self.used = self.used.saturating_sub(1);
            self.last = self.last.saturating_sub(1);
        }
        if self.used < self.palettes.len() {
            self.palettes[self.used] = colors;
        } else {
            self.palettes.push(colors);
        }
        self.used = self.used.saturating_add(1).min(MAX_COLOR_STACK_DEPTH);
        self.last = self.last.max(self.used);
    }

    fn pop(&mut self) -> Option<Colors> {
        let actual = self.used.checked_sub(1)?;
        let colors = self.palettes.get(actual).copied()?;
        self.used = actual;
        Some(colors)
    }

    fn store(&mut self, slot: usize, colors: Colors) {
        if slot == 0 || slot > MAX_COLOR_STACK_DEPTH {
            return;
        }
        let index = slot - 1;
        if self.palettes.len() <= index {
            self.palettes.resize_with(index + 1, Colors::default);
        }
        self.palettes[index] = colors;
        self.used = slot;
        self.last = self.last.max(self.used);
    }

    fn restore(&mut self, slot: usize) -> Option<Colors> {
        if slot == 0 || slot > MAX_COLOR_STACK_DEPTH {
            return None;
        }
        let actual = slot - 1;
        let colors = self.palettes.get(actual).copied()?;
        self.used = actual;
        Some(colors)
    }

    fn report_response_bytes(&self) -> Vec<u8> {
        format!("\x1b[?{};{}#Q", self.used, self.last).into_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalColorSequenceTerminator {
    Bel,
    St,
}

#[derive(Debug, Default)]
struct TerminalColorSequenceTracker {
    state: TerminalColorSequenceState,
}

#[derive(Debug, Default)]
enum TerminalColorSequenceState {
    #[default]
    Ground,
    Escape,
    Osc {
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
    Csi {
        payload: Vec<u8>,
    },
    Dcs {
        saw_escape: bool,
    },
}

impl TerminalColorSequenceTracker {
    const MAX_OSC_COLOR_BYTES: usize = 4096;

    fn advance(&mut self, byte: u8) -> Option<Vec<TerminalColorSequenceOperation>> {
        match &mut self.state {
            TerminalColorSequenceState::Ground => {
                self.state = match byte {
                    0x1b => TerminalColorSequenceState::Escape,
                    0x9b => TerminalColorSequenceState::Csi { payload: Vec::new() },
                    0x90 => TerminalColorSequenceState::Dcs { saw_escape: false },
                    0x9d => TerminalColorSequenceState::Osc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    _ => TerminalColorSequenceState::Ground,
                };
                None
            }
            TerminalColorSequenceState::Escape => {
                self.state = match byte {
                    b']' => TerminalColorSequenceState::Osc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    b'[' => TerminalColorSequenceState::Csi { payload: Vec::new() },
                    b'P' => TerminalColorSequenceState::Dcs { saw_escape: false },
                    0x1b => TerminalColorSequenceState::Escape,
                    _ => TerminalColorSequenceState::Ground,
                };
                None
            }
            TerminalColorSequenceState::Csi { payload } => {
                if byte == 0x1b {
                    self.state = TerminalColorSequenceState::Escape;
                    return None;
                }
                if matches!(byte, b'0'..=b'9' | b';' | b':' | b'#' | b'?') {
                    if payload.len() < 128 {
                        payload.push(byte);
                    }
                    return None;
                }
                let operations = terminal_color_sequence_operations_from_csi(payload, byte);
                self.state = TerminalColorSequenceState::Ground;
                operations
            }
            TerminalColorSequenceState::Osc { payload, saw_escape, truncated } => {
                if byte == 0x9c {
                    let operations = terminal_color_sequence_operations_from_osc(
                        payload,
                        *truncated,
                        TerminalColorSequenceTerminator::St,
                    );
                    self.state = TerminalColorSequenceState::Ground;
                    return operations;
                }
                if *saw_escape {
                    let operations = (byte == b'\\').then(|| {
                        terminal_color_sequence_operations_from_osc(
                            payload,
                            *truncated,
                            TerminalColorSequenceTerminator::St,
                        )
                    });
                    self.state = TerminalColorSequenceState::Ground;
                    return operations.flatten();
                }
                if byte == 0x07 {
                    let operations = terminal_color_sequence_operations_from_osc(
                        payload,
                        *truncated,
                        TerminalColorSequenceTerminator::Bel,
                    );
                    self.state = TerminalColorSequenceState::Ground;
                    return operations;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                } else if payload.len() < Self::MAX_OSC_COLOR_BYTES {
                    payload.push(byte);
                } else {
                    *truncated = true;
                }
                None
            }
            TerminalColorSequenceState::Dcs { saw_escape } => {
                if byte == 0x9c {
                    self.state = TerminalColorSequenceState::Ground;
                    return None;
                }
                if *saw_escape {
                    if byte == b'\\' {
                        self.state = TerminalColorSequenceState::Ground;
                    } else {
                        *saw_escape = byte == 0x1b;
                    }
                    return None;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TerminalSynchronizedOutputEvent {
    Begin,
    End,
    ActiveDeviceAttributesQuery(TerminalDeviceAttributesQuery),
    ActiveDeviceStatusQuery,
    CursorPositionQuery(TerminalCursorPositionQuery),
    ActiveTextAreaSizeQuery,
    ScreenPixelSizeQuery,
    CharacterCellSizeQuery,
    Iterm2CellSizeQuery,
    ScreenCharSizeQuery,
    XtermVersionQuery,
    FeatureReportQuery,
    StatusStringQuery(TerminalStatusStringQuery),
    TermcapQuery(TerminalTermcapQuery),
    ActiveModeQuery(TerminalModeQuery),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalDeviceAttributesQuery {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalCursorPositionQuery {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalStatusStringQuery {
    Sgr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TerminalTermcapQuery {
    names: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalModeQuery {
    Public(u16),
    Private(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalModeState {
    NotSupported = 0,
    Set = 1,
    Reset = 2,
}

impl From<bool> for TerminalModeState {
    fn from(value: bool) -> Self {
        if value { Self::Set } else { Self::Reset }
    }
}

#[derive(Debug, Default)]
struct TerminalSynchronizedOutputTracker {
    state: TerminalSynchronizedOutputState,
    active: bool,
    active_bytes: usize,
}

#[derive(Debug, Default)]
enum TerminalSynchronizedOutputState {
    #[default]
    Ground,
    Escape,
    Csi {
        payload: Vec<u8>,
    },
    ControlString {
        kind: TerminalControlStringKind,
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalControlStringKind {
    Dcs,
    Osc,
    Ignored,
}

impl TerminalSynchronizedOutputTracker {
    const MAX_CSI_BYTES: usize = 32;
    const MAX_CONTROL_STRING_QUERY_BYTES: usize = 128;
    const MAX_SYNCHRONIZED_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

    fn advance(&mut self, byte: u8) -> Option<TerminalSynchronizedOutputEvent> {
        if self.active {
            self.active_bytes = self.active_bytes.saturating_add(1);
            if self.active_bytes > Self::MAX_SYNCHRONIZED_OUTPUT_BYTES {
                self.active = false;
                self.active_bytes = 0;
                self.state = TerminalSynchronizedOutputState::Ground;
                return Some(TerminalSynchronizedOutputEvent::End);
            }
        }

        match &mut self.state {
            TerminalSynchronizedOutputState::Ground => {
                self.state = match byte {
                    0x1b => TerminalSynchronizedOutputState::Escape,
                    0x90 | 0x9e | 0x9f => TerminalSynchronizedOutputState::ControlString {
                        kind: if byte == 0x90 {
                            TerminalControlStringKind::Dcs
                        } else {
                            TerminalControlStringKind::Ignored
                        },
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x9d => TerminalSynchronizedOutputState::ControlString {
                        kind: TerminalControlStringKind::Osc,
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x9b => TerminalSynchronizedOutputState::Csi { payload: Vec::new() },
                    _ => TerminalSynchronizedOutputState::Ground,
                };
                None
            }
            TerminalSynchronizedOutputState::Escape => {
                self.state = match byte {
                    b'[' => TerminalSynchronizedOutputState::Csi { payload: Vec::new() },
                    b']' => TerminalSynchronizedOutputState::ControlString {
                        kind: TerminalControlStringKind::Osc,
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    b'P' | b'^' | b'_' => TerminalSynchronizedOutputState::ControlString {
                        kind: if byte == b'P' {
                            TerminalControlStringKind::Dcs
                        } else {
                            TerminalControlStringKind::Ignored
                        },
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x1b => TerminalSynchronizedOutputState::Escape,
                    _ => TerminalSynchronizedOutputState::Ground,
                };
                None
            }
            TerminalSynchronizedOutputState::Csi { payload } => {
                if matches!(byte, 0x18 | 0x1a) {
                    self.state = TerminalSynchronizedOutputState::Ground;
                    return None;
                }
                if (0x40..=0x7e).contains(&byte) {
                    let event =
                        terminal_synchronized_output_event_from_csi(payload, byte, self.active);
                    self.state = TerminalSynchronizedOutputState::Ground;
                    if let Some(event) = event.as_ref() {
                        match event {
                            TerminalSynchronizedOutputEvent::Begin => {
                                self.active = true;
                                self.active_bytes = 0;
                            }
                            TerminalSynchronizedOutputEvent::End => {
                                self.active = false;
                                self.active_bytes = 0;
                            }
                            TerminalSynchronizedOutputEvent::ActiveDeviceAttributesQuery(_)
                            | TerminalSynchronizedOutputEvent::ActiveDeviceStatusQuery
                            | TerminalSynchronizedOutputEvent::CursorPositionQuery(_)
                            | TerminalSynchronizedOutputEvent::ActiveTextAreaSizeQuery
                            | TerminalSynchronizedOutputEvent::ScreenPixelSizeQuery
                            | TerminalSynchronizedOutputEvent::CharacterCellSizeQuery
                            | TerminalSynchronizedOutputEvent::Iterm2CellSizeQuery
                            | TerminalSynchronizedOutputEvent::ScreenCharSizeQuery
                            | TerminalSynchronizedOutputEvent::XtermVersionQuery
                            | TerminalSynchronizedOutputEvent::FeatureReportQuery
                            | TerminalSynchronizedOutputEvent::StatusStringQuery(_)
                            | TerminalSynchronizedOutputEvent::TermcapQuery(_)
                            | TerminalSynchronizedOutputEvent::ActiveModeQuery(_) => {}
                        }
                    }
                    return event;
                }
                if payload.len() < Self::MAX_CSI_BYTES {
                    payload.push(byte);
                } else {
                    self.state = TerminalSynchronizedOutputState::Ground;
                }
                None
            }
            TerminalSynchronizedOutputState::ControlString {
                kind,
                payload,
                saw_escape,
                truncated,
            } => {
                if byte == 0x9c || (*kind == TerminalControlStringKind::Osc && byte == 0x07) {
                    let event = terminal_synchronized_output_event_from_control_string(
                        *kind, payload, *truncated,
                    );
                    self.state = TerminalSynchronizedOutputState::Ground;
                    return event;
                }
                if *saw_escape {
                    if byte == b'\\' {
                        let event = terminal_synchronized_output_event_from_control_string(
                            *kind, payload, *truncated,
                        );
                        self.state = TerminalSynchronizedOutputState::Ground;
                        return event;
                    }
                    self.state = if byte == 0x1b {
                        TerminalSynchronizedOutputState::Escape
                    } else {
                        TerminalSynchronizedOutputState::Ground
                    };
                    return None;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                } else if matches!(
                    *kind,
                    TerminalControlStringKind::Dcs | TerminalControlStringKind::Osc
                ) {
                    if payload.len() < Self::MAX_CONTROL_STRING_QUERY_BYTES {
                        payload.push(byte);
                    } else {
                        *truncated = true;
                    }
                }
                None
            }
        }
    }
}

fn terminal_synchronized_output_event_from_csi(
    payload: &[u8],
    final_byte: u8,
    active: bool,
) -> Option<TerminalSynchronizedOutputEvent> {
    match (payload, final_byte) {
        (b"?2026", b'h') => Some(TerminalSynchronizedOutputEvent::Begin),
        (b"?2026", b'l') => Some(TerminalSynchronizedOutputEvent::End),
        (b"" | b"0", b'c') if active => {
            Some(TerminalSynchronizedOutputEvent::ActiveDeviceAttributesQuery(
                TerminalDeviceAttributesQuery::Primary,
            ))
        }
        (b">" | b">0", b'c') if active => {
            Some(TerminalSynchronizedOutputEvent::ActiveDeviceAttributesQuery(
                TerminalDeviceAttributesQuery::Secondary,
            ))
        }
        (b">" | b">0", b'q') => Some(TerminalSynchronizedOutputEvent::XtermVersionQuery),
        (b"5", b'n') if active => Some(TerminalSynchronizedOutputEvent::ActiveDeviceStatusQuery),
        (b"6", b'n') if active => Some(TerminalSynchronizedOutputEvent::CursorPositionQuery(
            TerminalCursorPositionQuery::Public,
        )),
        (b"?6", b'n') => Some(TerminalSynchronizedOutputEvent::CursorPositionQuery(
            TerminalCursorPositionQuery::Private,
        )),
        (b"18", b't') if active => Some(TerminalSynchronizedOutputEvent::ActiveTextAreaSizeQuery),
        (b"15", b't') => Some(TerminalSynchronizedOutputEvent::ScreenPixelSizeQuery),
        (b"16", b't') => Some(TerminalSynchronizedOutputEvent::CharacterCellSizeQuery),
        (b"19", b't') => Some(TerminalSynchronizedOutputEvent::ScreenCharSizeQuery),
        (_, b'p') if active => terminal_mode_query_from_decrqm_payload(payload)
            .map(TerminalSynchronizedOutputEvent::ActiveModeQuery),
        _ => None,
    }
}

fn terminal_synchronized_output_event_from_control_string(
    kind: TerminalControlStringKind,
    payload: &[u8],
    truncated: bool,
) -> Option<TerminalSynchronizedOutputEvent> {
    if truncated {
        return None;
    }

    match payload {
        b"$qm" if kind == TerminalControlStringKind::Dcs => {
            Some(TerminalSynchronizedOutputEvent::StatusStringQuery(TerminalStatusStringQuery::Sgr))
        }
        b"1337;ReportCellSize" if kind == TerminalControlStringKind::Osc => {
            Some(TerminalSynchronizedOutputEvent::Iterm2CellSizeQuery)
        }
        b"1337;Capabilities" if kind == TerminalControlStringKind::Osc => {
            Some(TerminalSynchronizedOutputEvent::FeatureReportQuery)
        }
        _ if kind == TerminalControlStringKind::Dcs => {
            terminal_termcap_query_from_control_string(payload)
                .map(TerminalSynchronizedOutputEvent::TermcapQuery)
        }
        _ => None,
    }
}

fn terminal_termcap_query_from_control_string(payload: &[u8]) -> Option<TerminalTermcapQuery> {
    let raw_names = payload.strip_prefix(b"+q")?;
    let names = raw_names
        .split(|byte| *byte == b';')
        .map(decode_terminal_hex_bytes)
        .collect::<Option<Vec<_>>>()?;

    (!names.is_empty()).then_some(TerminalTermcapQuery { names })
}

fn decode_terminal_hex_bytes(payload: &[u8]) -> Option<Vec<u8>> {
    if payload.is_empty() || !payload.len().is_multiple_of(2) {
        return None;
    }

    payload
        .chunks_exact(2)
        .map(|chunk| {
            let high = terminal_hex_digit_value(chunk[0])?;
            let low = terminal_hex_digit_value(chunk[1])?;
            Some((high << 4) | low)
        })
        .collect()
}

fn terminal_hex_digit_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn terminal_mode_query_from_decrqm_payload(payload: &[u8]) -> Option<TerminalModeQuery> {
    let payload = payload.strip_suffix(b"$")?;
    if let Some(private_payload) = payload.strip_prefix(b"?") {
        return parse_terminal_mode_query_number(private_payload).map(TerminalModeQuery::Private);
    }
    parse_terminal_mode_query_number(payload).map(TerminalModeQuery::Public)
}

fn parse_terminal_mode_query_number(payload: &[u8]) -> Option<u16> {
    if payload.is_empty() {
        return None;
    }

    let mut value = 0u32;
    for byte in payload {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u32::from(byte - b'0'))?;
        if value > u32::from(u16::MAX) {
            return None;
        }
    }

    Some(value as u16)
}

fn push_synchronized_output_control_query_response(
    state: &EmulatorState,
    event: Option<&TerminalSynchronizedOutputEvent>,
) {
    match event {
        Some(TerminalSynchronizedOutputEvent::ActiveDeviceAttributesQuery(query)) => {
            let response = match query {
                TerminalDeviceAttributesQuery::Primary => {
                    terminal_primary_device_attributes_response_bytes()
                }
                TerminalDeviceAttributesQuery::Secondary => {
                    terminal_secondary_device_attributes_response_bytes()
                }
            };
            push_response_bytes(&state.response_bytes, response);
        }
        Some(TerminalSynchronizedOutputEvent::ActiveDeviceStatusQuery) => {
            push_response_bytes(&state.response_bytes, b"\x1b[0n".to_vec());
        }
        Some(TerminalSynchronizedOutputEvent::CursorPositionQuery(query)) => {
            if let Some((row, col)) = terminal_cursor_position(&state.term) {
                let response = match query {
                    TerminalCursorPositionQuery::Public => {
                        format!("\x1b[{};{}R", row + 1, col + 1)
                    }
                    TerminalCursorPositionQuery::Private => {
                        format!("\x1b[?{};{}R", row + 1, col + 1)
                    }
                };
                push_response_bytes(&state.response_bytes, response.into_bytes());
            }
        }
        Some(TerminalSynchronizedOutputEvent::ActiveTextAreaSizeQuery) => {
            if let Ok(window_size) = state.window_size.lock() {
                push_response_bytes(
                    &state.response_bytes,
                    terminal_char_size_response_bytes(8, *window_size),
                );
            }
        }
        Some(TerminalSynchronizedOutputEvent::ScreenPixelSizeQuery) => {
            if let Ok(window_size) = state.window_size.lock() {
                push_response_bytes(
                    &state.response_bytes,
                    terminal_pixel_size_response_bytes(5, *window_size),
                );
            }
        }
        Some(TerminalSynchronizedOutputEvent::CharacterCellSizeQuery) => {
            if let Ok(window_size) = state.window_size.lock() {
                push_response_bytes(
                    &state.response_bytes,
                    terminal_cell_size_response_bytes(*window_size),
                );
            }
        }
        Some(TerminalSynchronizedOutputEvent::Iterm2CellSizeQuery) => {
            if let Ok(window_size) = state.window_size.lock() {
                push_response_bytes(
                    &state.response_bytes,
                    terminal_iterm2_cell_size_response_bytes(*window_size),
                );
            }
        }
        Some(TerminalSynchronizedOutputEvent::ScreenCharSizeQuery) => {
            if let Ok(window_size) = state.window_size.lock() {
                push_response_bytes(
                    &state.response_bytes,
                    terminal_char_size_response_bytes(9, *window_size),
                );
            }
        }
        Some(TerminalSynchronizedOutputEvent::XtermVersionQuery) => {
            push_response_bytes(&state.response_bytes, terminal_xterm_version_response_bytes());
        }
        Some(TerminalSynchronizedOutputEvent::FeatureReportQuery) => {
            push_response_bytes(&state.response_bytes, terminal_feature_report_response_bytes());
        }
        Some(TerminalSynchronizedOutputEvent::StatusStringQuery(
            TerminalStatusStringQuery::Sgr,
        )) => {
            push_response_bytes(
                &state.response_bytes,
                terminal_sgr_status_string_response_bytes(&state.extra_styles.sgr_state),
            );
        }
        Some(TerminalSynchronizedOutputEvent::TermcapQuery(query)) => {
            push_response_bytes(&state.response_bytes, terminal_xtgettcap_response_bytes(query));
        }
        Some(TerminalSynchronizedOutputEvent::ActiveModeQuery(query)) => {
            let mode_state = terminal_mode_query_state(state, *query);
            push_response_bytes(
                &state.response_bytes,
                terminal_mode_query_response_bytes(*query, mode_state),
            );
        }
        Some(TerminalSynchronizedOutputEvent::Begin | TerminalSynchronizedOutputEvent::End)
        | None => {}
    }
}

fn terminal_mode_query_state(state: &EmulatorState, query: TerminalModeQuery) -> TerminalModeState {
    match query {
        TerminalModeQuery::Public(4) => state.term.mode().contains(TermMode::INSERT).into(),
        TerminalModeQuery::Public(20) => {
            state.term.mode().contains(TermMode::LINE_FEED_NEW_LINE).into()
        }
        TerminalModeQuery::Public(_) => TerminalModeState::NotSupported,
        TerminalModeQuery::Private(69) => state.horizontal_margin_tracker.mode.into(),
        TerminalModeQuery::Private(mode) => terminal_private_mode_state(&state.term, mode),
    }
}

fn terminal_private_mode_state(term: &Term<EmulatorEventListener>, mode: u16) -> TerminalModeState {
    match mode {
        1 => term.mode().contains(TermMode::APP_CURSOR).into(),
        6 => term.mode().contains(TermMode::ORIGIN).into(),
        7 => term.mode().contains(TermMode::LINE_WRAP).into(),
        12 => term.cursor_style().blinking.into(),
        25 => term.mode().contains(TermMode::SHOW_CURSOR).into(),
        1000 => term.mode().contains(TermMode::MOUSE_REPORT_CLICK).into(),
        1002 => term.mode().contains(TermMode::MOUSE_DRAG).into(),
        1003 => term.mode().contains(TermMode::MOUSE_MOTION).into(),
        1004 => term.mode().contains(TermMode::FOCUS_IN_OUT).into(),
        1005 => term.mode().contains(TermMode::UTF8_MOUSE).into(),
        1006 => term.mode().contains(TermMode::SGR_MOUSE).into(),
        1007 => term.mode().contains(TermMode::ALTERNATE_SCROLL).into(),
        1042 => term.mode().contains(TermMode::URGENCY_HINTS).into(),
        47 | 1047 | 1049 => term.mode().contains(TermMode::ALT_SCREEN).into(),
        2004 => term.mode().contains(TermMode::BRACKETED_PASTE).into(),
        2026 => TerminalModeState::Set,
        _ => TerminalModeState::NotSupported,
    }
}

fn terminal_mode_query_response_bytes(
    query: TerminalModeQuery,
    mode_state: TerminalModeState,
) -> Vec<u8> {
    match query {
        TerminalModeQuery::Public(mode) => {
            format!("\x1b[{};{}$y", mode, mode_state as u8).into_bytes()
        }
        TerminalModeQuery::Private(mode) => {
            format!("\x1b[?{};{}$y", mode, mode_state as u8).into_bytes()
        }
    }
}

fn terminal_color_sequence_operations_from_csi(
    payload: &[u8],
    final_byte: u8,
) -> Option<Vec<TerminalColorSequenceOperation>> {
    parse_terminal_xterm_color_stack(payload, final_byte).map(|operations| {
        operations
            .into_iter()
            .map(|operation| match operation {
                TerminalXtermColorStackOperation::Push => {
                    TerminalColorSequenceOperation::PushColors
                }
                TerminalXtermColorStackOperation::Pop => TerminalColorSequenceOperation::PopColors,
                TerminalXtermColorStackOperation::Store(slot) => {
                    TerminalColorSequenceOperation::StoreColors(slot)
                }
                TerminalXtermColorStackOperation::Restore(slot) => {
                    TerminalColorSequenceOperation::RestoreColors(slot)
                }
                TerminalXtermColorStackOperation::Report => {
                    TerminalColorSequenceOperation::ReportColors
                }
            })
            .collect()
    })
}

fn synchronized_output_visual_parser_byte(
    event: Option<&TerminalSynchronizedOutputEvent>,
    byte: u8,
) -> u8 {
    match event {
        Some(
            TerminalSynchronizedOutputEvent::ActiveDeviceAttributesQuery(_)
            | TerminalSynchronizedOutputEvent::ActiveDeviceStatusQuery
            | TerminalSynchronizedOutputEvent::CursorPositionQuery(_)
            | TerminalSynchronizedOutputEvent::ActiveTextAreaSizeQuery
            | TerminalSynchronizedOutputEvent::ScreenPixelSizeQuery
            | TerminalSynchronizedOutputEvent::CharacterCellSizeQuery
            | TerminalSynchronizedOutputEvent::Iterm2CellSizeQuery
            | TerminalSynchronizedOutputEvent::ScreenCharSizeQuery
            | TerminalSynchronizedOutputEvent::XtermVersionQuery
            | TerminalSynchronizedOutputEvent::FeatureReportQuery
            | TerminalSynchronizedOutputEvent::ActiveModeQuery(_),
        ) => 0x18,
        _ => byte,
    }
}

fn terminal_color_sequence_operations_from_osc(
    payload: &[u8],
    truncated: bool,
    terminator: TerminalColorSequenceTerminator,
) -> Option<Vec<TerminalColorSequenceOperation>> {
    if truncated {
        return None;
    }
    if let Some(stack_operation) = parse_terminal_kitty_color_stack(payload) {
        return Some(vec![match stack_operation {
            TerminalKittyColorStackOperation::Push => TerminalColorSequenceOperation::PushColors,
            TerminalKittyColorStackOperation::Pop => TerminalColorSequenceOperation::PopColors,
        }]);
    }
    if let Some(kitty_operations) = parse_terminal_kitty_color_control(payload) {
        let operations = kitty_operations
            .into_iter()
            .filter_map(|operation| match operation {
                TerminalKittyColorControlOperation::Update(
                    target,
                    ScreenColor::Rgb { r, g, b },
                ) => {
                    let index = terminal_palette_target_native_index(target)?;
                    Some(TerminalColorSequenceOperation::Update(TerminalColorUpdate {
                        index,
                        color: rgb(r, g, b),
                    }))
                }
                TerminalKittyColorControlOperation::Update(_, _) => None,
                TerminalKittyColorControlOperation::Reset(target) => {
                    terminal_palette_target_native_index(target)
                        .map(TerminalColorSequenceOperation::Reset)
                }
                TerminalKittyColorControlOperation::QueryKnown { key, target } => {
                    Some(TerminalColorSequenceOperation::KittyQuery(TerminalKittyColorQuery {
                        key,
                        index: terminal_palette_target_native_index(target),
                        terminator,
                    }))
                }
                TerminalKittyColorControlOperation::QueryUnknown { key } => {
                    Some(TerminalColorSequenceOperation::KittyQuery(TerminalKittyColorQuery {
                        key,
                        index: None,
                        terminator,
                    }))
                }
            })
            .collect::<Vec<_>>();
        return (!operations.is_empty()).then_some(operations);
    }
    if let Some((target, ScreenColor::Rgb { r, g, b })) = parse_iterm2_set_colors_update(payload)
        && let Some(index) = terminal_palette_target_native_index(target)
    {
        return Some(vec![TerminalColorSequenceOperation::Update(TerminalColorUpdate {
            index,
            color: rgb(r, g, b),
        })]);
    }
    if let Some((index, ScreenColor::Rgb { r, g, b })) =
        parse_legacy_linux_console_palette_update(payload)
    {
        let color = rgb(r, g, b);
        return Some(
            legacy_linux_console_palette_indexes(index)
                .into_iter()
                .map(|index| {
                    TerminalColorSequenceOperation::Update(TerminalColorUpdate { index, color })
                })
                .collect(),
        );
    }
    if is_legacy_linux_console_palette_reset(payload) {
        return Some(
            (0..16)
                .flat_map(legacy_linux_console_palette_indexes)
                .map(TerminalColorSequenceOperation::Reset)
                .collect(),
        );
    }
    if let Some((target, ScreenColor::Rgb { r, g, b })) =
        parse_terminal_osc_p_palette_update(payload)
        && let Some(index) = terminal_osc_p_palette_target_index(target)
    {
        return Some(vec![TerminalColorSequenceOperation::Update(TerminalColorUpdate {
            index,
            color: rgb(r, g, b),
        })]);
    }

    let fields = payload.split(|byte| *byte == b';').collect::<Vec<_>>();
    let command = *fields.first()?;
    let mut operations = Vec::new();

    match command {
        b"4" => {
            if fields.len() <= 1 {
                return None;
            }
            for pair in fields[1..].chunks(2) {
                if pair.len() != 2 {
                    break;
                }
                let Some(index) = parse_terminal_osc4_color_index(pair[0]) else {
                    continue;
                };
                if index.lookup >= ALACRITTY_COLOR_COUNT {
                    continue;
                }
                if pair[1] == b"?" {
                    if index.lookup > 255 {
                        operations.push(TerminalColorSequenceOperation::Query(
                            TerminalColorQuery {
                                index: index.lookup,
                                response_index: index.response,
                                terminator,
                            },
                        ));
                    }
                } else if let Some(color) = parse_terminal_extra_color_spec(pair[1]) {
                    operations.push(TerminalColorSequenceOperation::Update(TerminalColorUpdate {
                        index: index.lookup,
                        color,
                    }));
                }
            }
        }
        b"104" => {
            if fields.len() == 1 {
                for index in 256..ALACRITTY_COLOR_COUNT {
                    operations.push(TerminalColorSequenceOperation::Reset(index));
                }
            }
            for field in &fields[1..] {
                let Some(index) = parse_terminal_osc4_color_index(field) else {
                    continue;
                };
                if index.lookup > 255 && index.lookup < ALACRITTY_COLOR_COUNT {
                    operations.push(TerminalColorSequenceOperation::Reset(index.lookup));
                }
            }
        }
        b"10" | b"11" | b"12" => {
            let mut target = terminal_default_palette_target_from_osc_code(command)?;
            let mut index = 1;
            while index < fields.len() {
                if index + 1 < fields.len()
                    && let Some(explicit_target) =
                        terminal_default_palette_target_from_osc_code(fields[index])
                {
                    target = explicit_target;
                    index += 1;
                }

                if let Some(color_index) = terminal_osc_p_palette_target_index(target)
                    && let Some(color) = parse_terminal_extra_color_spec(fields[index])
                {
                    operations.push(TerminalColorSequenceOperation::Update(TerminalColorUpdate {
                        index: color_index,
                        color,
                    }));
                }
                let Some(next_target) = next_terminal_default_palette_target(target) else {
                    break;
                };
                target = next_target;
                index += 1;
            }
        }
        _ => return None,
    }

    (!operations.is_empty()).then_some(operations)
}

fn legacy_linux_console_palette_indexes(index: u8) -> [usize; 1] {
    [usize::from(index)]
}

fn terminal_osc_p_palette_target_index(target: TerminalPaletteTarget) -> Option<usize> {
    match target {
        TerminalPaletteTarget::Foreground => Some(NamedColor::Foreground as usize),
        TerminalPaletteTarget::Background => Some(NamedColor::Background as usize),
        TerminalPaletteTarget::Cursor => Some(NamedColor::Cursor as usize),
        TerminalPaletteTarget::Ansi(_) => None,
    }
}

fn terminal_palette_target_native_index(target: TerminalPaletteTarget) -> Option<usize> {
    match target {
        TerminalPaletteTarget::Ansi(index) => Some(usize::from(index)),
        _ => terminal_osc_p_palette_target_index(target),
    }
}

fn terminal_color_query_response_bytes(query: TerminalColorQuery, color: Rgb) -> Vec<u8> {
    let terminator = match query.terminator {
        TerminalColorSequenceTerminator::Bel => "\x07",
        TerminalColorSequenceTerminator::St => "\x1b\\",
    };
    let response_index = match query.response_index {
        TerminalColorResponseIndex::Numeric(index) => index.to_string(),
        TerminalColorResponseIndex::ItermForeground => "-1".to_string(),
        TerminalColorResponseIndex::ItermBackground => "-2".to_string(),
    };
    format!(
        "\x1b]4;{};rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}{terminator}",
        response_index, color.r, color.r, color.g, color.g, color.b, color.b
    )
    .into_bytes()
}

fn terminal_kitty_color_query_response_bytes(
    query: TerminalKittyColorQuery,
    colors: &Colors,
) -> Vec<u8> {
    let terminator = match query.terminator {
        TerminalColorSequenceTerminator::Bel => "\x07",
        TerminalColorSequenceTerminator::St => "\x1b\\",
    };
    let Some(index) = query.index else {
        return format!("\x1b]21;{}=?{terminator}", query.key).into_bytes();
    };
    let Some(color) = terminal_response_rgb_for_index(index, colors) else {
        return format!("\x1b]21;{}={terminator}", query.key).into_bytes();
    };
    format!(
        "\x1b]21;{}=rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}{terminator}",
        query.key, color.r, color.r, color.g, color.g, color.b, color.b
    )
    .into_bytes()
}

fn parse_terminal_color_number(input: &[u8]) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let mut number = 0usize;
    for byte in input {
        if !byte.is_ascii_digit() {
            return None;
        }
        number = number.checked_mul(10)?.checked_add(usize::from(byte - b'0'))?;
    }
    Some(number)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalOsc4ColorIndex {
    lookup: usize,
    response: TerminalColorResponseIndex,
}

fn parse_terminal_osc4_color_index(input: &[u8]) -> Option<TerminalOsc4ColorIndex> {
    match input {
        b"-1" => Some(TerminalOsc4ColorIndex {
            lookup: NamedColor::Foreground as usize,
            response: TerminalColorResponseIndex::ItermForeground,
        }),
        b"-2" => Some(TerminalOsc4ColorIndex {
            lookup: NamedColor::Background as usize,
            response: TerminalColorResponseIndex::ItermBackground,
        }),
        _ => {
            let index = parse_terminal_color_number(input)?;
            Some(TerminalOsc4ColorIndex {
                lookup: index,
                response: TerminalColorResponseIndex::Numeric(index),
            })
        }
    }
}

fn parse_terminal_extra_color_spec(spec: &[u8]) -> Option<Rgb> {
    let raw = std::str::from_utf8(spec).ok()?.trim();
    match parse_terminal_color_spec(raw)? {
        ScreenColor::Rgb { r, g, b } => Some(rgb(r, g, b)),
        ScreenColor::Named { name } => parse_terminal_named_color_spec(&name),
        ScreenColor::Indexed { .. } => None,
    }
}

fn parse_terminal_named_color_spec(raw: &str) -> Option<Rgb> {
    let name = normalize_terminal_color_name(raw);
    if let Some(color) = parse_terminal_gray_name(&name) {
        return Some(color);
    }
    TERMINAL_NAMED_COLORS
        .iter()
        .find_map(|(candidate, r, g, b)| (*candidate == name).then_some(rgb(*r, *g, *b)))
}

fn normalize_terminal_color_name(name: &str) -> String {
    name.chars().filter(|ch| ch.is_ascii_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn parse_terminal_gray_name(name: &str) -> Option<Rgb> {
    let suffix = name.strip_prefix("gray").or_else(|| name.strip_prefix("grey"))?;
    if suffix.is_empty() {
        return Some(rgb(0xbe, 0xbe, 0xbe));
    }
    let percent = suffix.parse::<u16>().ok()?;
    if percent > 100 {
        return None;
    }
    let channel = ((u32::from(percent) * 255 + 50) / 100) as u8;
    Some(rgb(channel, channel, channel))
}

const TERMINAL_NAMED_COLORS: &[(&str, u8, u8, u8)] = &[
    ("aliceblue", 0xf0, 0xf8, 0xff),
    ("antiquewhite", 0xfa, 0xeb, 0xd7),
    ("aqua", 0x00, 0xff, 0xff),
    ("aquamarine", 0x7f, 0xff, 0xd4),
    ("azure", 0xf0, 0xff, 0xff),
    ("beige", 0xf5, 0xf5, 0xdc),
    ("bisque", 0xff, 0xe4, 0xc4),
    ("black", 0x00, 0x00, 0x00),
    ("blanchedalmond", 0xff, 0xeb, 0xcd),
    ("blue", 0x00, 0x00, 0xff),
    ("blueviolet", 0x8a, 0x2b, 0xe2),
    ("brown", 0xa5, 0x2a, 0x2a),
    ("burlywood", 0xde, 0xb8, 0x87),
    ("cadetblue", 0x5f, 0x9e, 0xa0),
    ("chartreuse", 0x7f, 0xff, 0x00),
    ("chocolate", 0xd2, 0x69, 0x1e),
    ("coral", 0xff, 0x7f, 0x50),
    ("cornflowerblue", 0x64, 0x95, 0xed),
    ("cornsilk", 0xff, 0xf8, 0xdc),
    ("crimson", 0xdc, 0x14, 0x3c),
    ("cyan", 0x00, 0xff, 0xff),
    ("darkblue", 0x00, 0x00, 0x8b),
    ("darkcyan", 0x00, 0x8b, 0x8b),
    ("darkgoldenrod", 0xb8, 0x86, 0x0b),
    ("darkgray", 0xa9, 0xa9, 0xa9),
    ("darkgreen", 0x00, 0x64, 0x00),
    ("darkgrey", 0xa9, 0xa9, 0xa9),
    ("darkkhaki", 0xbd, 0xb7, 0x6b),
    ("darkmagenta", 0x8b, 0x00, 0x8b),
    ("darkolivegreen", 0x55, 0x6b, 0x2f),
    ("darkorange", 0xff, 0x8c, 0x00),
    ("darkorchid", 0x99, 0x32, 0xcc),
    ("darkred", 0x8b, 0x00, 0x00),
    ("darksalmon", 0xe9, 0x96, 0x7a),
    ("darkseagreen", 0x8f, 0xbc, 0x8f),
    ("darkslateblue", 0x48, 0x3d, 0x8b),
    ("darkslategray", 0x2f, 0x4f, 0x4f),
    ("darkslategrey", 0x2f, 0x4f, 0x4f),
    ("darkturquoise", 0x00, 0xce, 0xd1),
    ("darkviolet", 0x94, 0x00, 0xd3),
    ("deeppink", 0xff, 0x14, 0x93),
    ("deepskyblue", 0x00, 0xbf, 0xff),
    ("dimgray", 0x69, 0x69, 0x69),
    ("dimgrey", 0x69, 0x69, 0x69),
    ("dodgerblue", 0x1e, 0x90, 0xff),
    ("firebrick", 0xb2, 0x22, 0x22),
    ("floralwhite", 0xff, 0xfa, 0xf0),
    ("forestgreen", 0x22, 0x8b, 0x22),
    ("fuchsia", 0xff, 0x00, 0xff),
    ("gainsboro", 0xdc, 0xdc, 0xdc),
    ("ghostwhite", 0xf8, 0xf8, 0xff),
    ("gold", 0xff, 0xd7, 0x00),
    ("goldenrod", 0xda, 0xa5, 0x20),
    ("gray", 0x80, 0x80, 0x80),
    ("green", 0x00, 0xff, 0x00),
    ("greenyellow", 0xad, 0xff, 0x2f),
    ("grey", 0x80, 0x80, 0x80),
    ("honeydew", 0xf0, 0xff, 0xf0),
    ("hotpink", 0xff, 0x69, 0xb4),
    ("indianred", 0xcd, 0x5c, 0x5c),
    ("indigo", 0x4b, 0x00, 0x82),
    ("ivory", 0xff, 0xff, 0xf0),
    ("khaki", 0xf0, 0xe6, 0x8c),
    ("lavender", 0xe6, 0xe6, 0xfa),
    ("lavenderblush", 0xff, 0xf0, 0xf5),
    ("lawngreen", 0x7c, 0xfc, 0x00),
    ("lemonchiffon", 0xff, 0xfa, 0xcd),
    ("lightblue", 0xad, 0xd8, 0xe6),
    ("lightcoral", 0xf0, 0x80, 0x80),
    ("lightcyan", 0xe0, 0xff, 0xff),
    ("lightgoldenrod", 0xee, 0xdd, 0x82),
    ("lightgoldenrodyellow", 0xfa, 0xfa, 0xd2),
    ("lightgray", 0xd3, 0xd3, 0xd3),
    ("lightgreen", 0x90, 0xee, 0x90),
    ("lightgrey", 0xd3, 0xd3, 0xd3),
    ("lightpink", 0xff, 0xb6, 0xc1),
    ("lightsalmon", 0xff, 0xa0, 0x7a),
    ("lightseagreen", 0x20, 0xb2, 0xaa),
    ("lightskyblue", 0x87, 0xce, 0xfa),
    ("lightslategray", 0x77, 0x88, 0x99),
    ("lightslategrey", 0x77, 0x88, 0x99),
    ("lightsteelblue", 0xb0, 0xc4, 0xde),
    ("lightyellow", 0xff, 0xff, 0xe0),
    ("lime", 0x00, 0xff, 0x00),
    ("limegreen", 0x32, 0xcd, 0x32),
    ("linen", 0xfa, 0xf0, 0xe6),
    ("magenta", 0xff, 0x00, 0xff),
    ("maroon", 0xb0, 0x30, 0x60),
    ("mediumaquamarine", 0x66, 0xcd, 0xaa),
    ("mediumblue", 0x00, 0x00, 0xcd),
    ("mediumorchid", 0xba, 0x55, 0xd3),
    ("mediumpurple", 0x93, 0x70, 0xdb),
    ("mediumseagreen", 0x3c, 0xb3, 0x71),
    ("mediumslateblue", 0x7b, 0x68, 0xee),
    ("mediumspringgreen", 0x00, 0xfa, 0x9a),
    ("mediumturquoise", 0x48, 0xd1, 0xcc),
    ("mediumvioletred", 0xc7, 0x15, 0x85),
    ("midnightblue", 0x19, 0x19, 0x70),
    ("mintcream", 0xf5, 0xff, 0xfa),
    ("mistyrose", 0xff, 0xe4, 0xe1),
    ("moccasin", 0xff, 0xe4, 0xb5),
    ("navajowhite", 0xff, 0xde, 0xad),
    ("navy", 0x00, 0x00, 0x80),
    ("oldlace", 0xfd, 0xf5, 0xe6),
    ("olive", 0x80, 0x80, 0x00),
    ("olivedrab", 0x6b, 0x8e, 0x23),
    ("orange", 0xff, 0xa5, 0x00),
    ("orangered", 0xff, 0x45, 0x00),
    ("orchid", 0xda, 0x70, 0xd6),
    ("palegoldenrod", 0xee, 0xe8, 0xaa),
    ("palegreen", 0x98, 0xfb, 0x98),
    ("paleturquoise", 0xaf, 0xee, 0xee),
    ("palevioletred", 0xdb, 0x70, 0x93),
    ("papayawhip", 0xff, 0xef, 0xd5),
    ("peachpuff", 0xff, 0xda, 0xb9),
    ("peru", 0xcd, 0x85, 0x3f),
    ("pink", 0xff, 0xc0, 0xcb),
    ("plum", 0xdd, 0xa0, 0xdd),
    ("powderblue", 0xb0, 0xe0, 0xe6),
    ("purple", 0xa0, 0x20, 0xf0),
    ("rebeccapurple", 0x66, 0x33, 0x99),
    ("red", 0xff, 0x00, 0x00),
    ("rosybrown", 0xbc, 0x8f, 0x8f),
    ("royalblue", 0x41, 0x69, 0xe1),
    ("saddlebrown", 0x8b, 0x45, 0x13),
    ("salmon", 0xfa, 0x80, 0x72),
    ("sandybrown", 0xf4, 0xa4, 0x60),
    ("seagreen", 0x2e, 0x8b, 0x57),
    ("seashell", 0xff, 0xf5, 0xee),
    ("sienna", 0xa0, 0x52, 0x2d),
    ("silver", 0xc0, 0xc0, 0xc0),
    ("skyblue", 0x87, 0xce, 0xeb),
    ("slateblue", 0x6a, 0x5a, 0xcd),
    ("slategray", 0x70, 0x80, 0x90),
    ("slategrey", 0x70, 0x80, 0x90),
    ("snow", 0xff, 0xfa, 0xfa),
    ("springgreen", 0x00, 0xff, 0x7f),
    ("steelblue", 0x46, 0x82, 0xb4),
    ("tan", 0xd2, 0xb4, 0x8c),
    ("teal", 0x00, 0x80, 0x80),
    ("thistle", 0xd8, 0xbf, 0xd8),
    ("tomato", 0xff, 0x63, 0x47),
    ("turquoise", 0x40, 0xe0, 0xd0),
    ("violet", 0xee, 0x82, 0xee),
    ("wheat", 0xf5, 0xde, 0xb3),
    ("white", 0xff, 0xff, 0xff),
    ("whitesmoke", 0xf5, 0xf5, 0xf5),
    ("yellow", 0xff, 0xff, 0x00),
    ("yellowgreen", 0x9a, 0xcd, 0x32),
];

#[derive(Debug, Default)]
struct TerminalTmuxPassthroughVisualTracker {
    state: TerminalTmuxPassthroughVisualState,
}

#[derive(Debug, Default)]
enum TerminalTmuxPassthroughVisualState {
    #[default]
    Ground,
    Escape,
    Dcs {
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
}

impl TerminalTmuxPassthroughVisualTracker {
    const MAX_VISUAL_PASSTHROUGH_BYTES: usize = 16 * 1024;

    fn advance(&mut self, byte: u8) -> Option<Vec<u8>> {
        match &mut self.state {
            TerminalTmuxPassthroughVisualState::Ground => {
                self.state = match byte {
                    0x1b => TerminalTmuxPassthroughVisualState::Escape,
                    0x90 => TerminalTmuxPassthroughVisualState::Dcs {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    _ => TerminalTmuxPassthroughVisualState::Ground,
                };
                None
            }
            TerminalTmuxPassthroughVisualState::Escape => {
                self.state = match byte {
                    b'P' => TerminalTmuxPassthroughVisualState::Dcs {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x1b => TerminalTmuxPassthroughVisualState::Escape,
                    _ => TerminalTmuxPassthroughVisualState::Ground,
                };
                None
            }
            TerminalTmuxPassthroughVisualState::Dcs { payload, saw_escape, truncated } => {
                if byte == 0x9c {
                    let decoded = terminal_tmux_passthrough_payload(payload, *truncated);
                    self.state = TerminalTmuxPassthroughVisualState::Ground;
                    return decoded;
                }
                if *saw_escape {
                    if byte == b'\\' {
                        let decoded = terminal_tmux_passthrough_payload(payload, *truncated);
                        self.state = TerminalTmuxPassthroughVisualState::Ground;
                        return decoded;
                    }
                    if byte == 0x1b {
                        if payload.len() < Self::MAX_VISUAL_PASSTHROUGH_BYTES {
                            payload.push(0x1b);
                        } else {
                            *truncated = true;
                        }
                    } else if payload.len() + 2 <= Self::MAX_VISUAL_PASSTHROUGH_BYTES {
                        payload.push(0x1b);
                        payload.push(byte);
                    } else {
                        *truncated = true;
                    }
                    *saw_escape = false;
                    return None;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                } else if payload.len() < Self::MAX_VISUAL_PASSTHROUGH_BYTES {
                    payload.push(byte);
                } else {
                    *truncated = true;
                }
                None
            }
        }
    }
}

fn terminal_metadata_update_from_osc(
    payload: &[u8],
    truncated: bool,
) -> Option<TerminalMetadataUpdate> {
    if truncated {
        return None;
    }

    if let Some(title) = terminal_title_from_osc_payload(payload) {
        return Some(TerminalMetadataUpdate::Title(title));
    }

    if let Some(working_directory) = payload.strip_prefix(b"7;") {
        let uri = terminal_working_directory_uri(working_directory)?;
        return Some(TerminalMetadataUpdate::WorkingDirectoryUri(uri));
    }

    if let Some(working_directory) = payload.strip_prefix(b"9;9;") {
        let uri = terminal_working_directory_uri(working_directory)?;
        return Some(TerminalMetadataUpdate::WorkingDirectoryUri(uri));
    }

    if let Some(working_directory) = terminal_vscode_working_directory(payload) {
        let uri = terminal_working_directory_uri(working_directory)?;
        return Some(TerminalMetadataUpdate::WorkingDirectoryUri(uri));
    }

    if let Some(working_directory) = payload.strip_prefix(b"1337;CurrentDir=") {
        let uri = terminal_working_directory_uri(working_directory)?;
        return Some(TerminalMetadataUpdate::WorkingDirectoryUri(uri));
    }

    if let Some((key, value)) = terminal_user_variable_from_osc(payload) {
        return Some(TerminalMetadataUpdate::UserVariable { key, value });
    }

    if let Some(shell_integration_mark) = terminal_shell_integration_mark(payload) {
        return Some(TerminalMetadataUpdate::SemanticMark(shell_integration_mark));
    }

    if let Some(progress) = terminal_progress_from_osc(payload) {
        return Some(TerminalMetadataUpdate::Progress(progress));
    }

    if let Some(shape) = terminal_cursor_shape_from_osc(payload) {
        return Some(TerminalMetadataUpdate::CursorShape(shape));
    }

    if let Some(side_effect) = terminal_notification_side_effect_from_osc(payload) {
        return Some(TerminalMetadataUpdate::SideEffect(side_effect));
    }

    None
}

fn terminal_title_from_osc_payload(payload: &[u8]) -> Option<String> {
    let title = payload
        .strip_prefix(b"0;")
        .or_else(|| payload.strip_prefix(b"1;"))
        .or_else(|| payload.strip_prefix(b"2;"))?;
    terminal_metadata_string(title, 4096)
}

fn terminal_cursor_shape_from_osc(payload: &[u8]) -> Option<AlacrittyCursorShape> {
    match payload.strip_prefix(b"1337;CursorShape=")? {
        b"0" => Some(AlacrittyCursorShape::Block),
        b"1" => Some(AlacrittyCursorShape::Beam),
        b"2" => Some(AlacrittyCursorShape::Underline),
        _ => None,
    }
}

fn terminal_metadata_update_from_tmux_dcs(
    payload: &[u8],
    truncated: bool,
) -> Option<TerminalMetadataUpdate> {
    let passthrough = terminal_tmux_passthrough_payload(payload, truncated)?;
    let mut tracker = TerminalMetadataSequenceTracker::default();
    passthrough.into_iter().find_map(|byte| tracker.advance(byte))
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

fn terminal_user_variable_from_osc(payload: &[u8]) -> Option<(String, String)> {
    let body = payload.strip_prefix(b"1337;SetUserVar=")?;
    let (key, encoded_value) = split_once_byte(body, b'=')?;
    let key = terminal_user_variable_key(key)?;
    if estimated_base64_decoded_len(encoded_value) > 4096 {
        return None;
    }
    let decoded = BASE64_STANDARD.decode(encoded_value).ok()?;
    let value = std::str::from_utf8(&decoded).ok()?;
    if value.chars().any(|ch| ch.is_control()) {
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

fn terminal_vscode_working_directory(payload: &[u8]) -> Option<&[u8]> {
    let body = payload.strip_prefix(b"633;P;")?;
    body.split(|byte| *byte == b';').find_map(|property| property.strip_prefix(b"Cwd="))
}

fn terminal_metadata_text(value: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(value).ok()?.trim();
    let text = strip_matching_quotes(text);
    (!text.chars().any(|ch| ch.is_control())).then_some(text)
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

fn terminal_working_directory_path_to_uri(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//") {
        let (host, path) = rest.split_once('/')?;
        if host.is_empty() || path.is_empty() || host.chars().any(|ch| ch.is_control()) {
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

fn truncate_terminal_metadata_message(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{truncated}...") } else { truncated }
}

#[derive(Debug, Default)]
struct TerminalMediaSequenceTracker {
    state: TerminalMediaSequenceState,
    kitty_chunk: Option<TerminalKittyGraphicsChunk>,
    iterm2_multipart_file: Option<TerminalIterm2MultipartFile>,
}

#[derive(Debug)]
struct TerminalMediaSequenceEvent {
    media: Option<ScreenLineMedia>,
    response: Option<Vec<u8>>,
}

impl TerminalMediaSequenceEvent {
    fn media(media: ScreenLineMedia) -> Self {
        Self { media: Some(media), response: None }
    }

    fn response(response: Vec<u8>) -> Self {
        Self { media: None, response: Some(response) }
    }
}

#[derive(Debug, Default)]
enum TerminalMediaSequenceState {
    #[default]
    Ground,
    Escape,
    Osc {
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
    Apc {
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
    Dcs {
        header_done: bool,
        kind: Option<ScreenLineMediaKind>,
        payload: Vec<u8>,
        saw_escape: bool,
        truncated: bool,
    },
}

impl TerminalMediaSequenceTracker {
    const MAX_OSC_MEDIA_BYTES: usize = 512 * 1024;
    const MAX_INLINE_IMAGE_BYTES: usize = 384 * 1024;

    fn advance(&mut self, byte: u8) -> Option<TerminalMediaSequenceEvent> {
        match &mut self.state {
            TerminalMediaSequenceState::Ground => {
                self.state = match byte {
                    0x1b => TerminalMediaSequenceState::Escape,
                    0x90 => TerminalMediaSequenceState::Dcs {
                        header_done: false,
                        kind: None,
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x9d => TerminalMediaSequenceState::Osc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x9f => TerminalMediaSequenceState::Apc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    _ => TerminalMediaSequenceState::Ground,
                };
                None
            }
            TerminalMediaSequenceState::Escape => {
                self.state = match byte {
                    b']' => TerminalMediaSequenceState::Osc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    b'_' => TerminalMediaSequenceState::Apc {
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    b'P' => TerminalMediaSequenceState::Dcs {
                        header_done: false,
                        kind: None,
                        payload: Vec::new(),
                        saw_escape: false,
                        truncated: false,
                    },
                    0x1b => TerminalMediaSequenceState::Escape,
                    _ => TerminalMediaSequenceState::Ground,
                };
                None
            }
            TerminalMediaSequenceState::Osc { payload, saw_escape, truncated } => {
                if byte == 0x9c {
                    let truncated = *truncated;
                    let media = finish_osc_media_payload(
                        &mut self.iterm2_multipart_file,
                        payload,
                        truncated,
                    );
                    self.state = TerminalMediaSequenceState::Ground;
                    return media.map(TerminalMediaSequenceEvent::media);
                }
                if *saw_escape {
                    let media = if byte == b'\\' {
                        let truncated = *truncated;
                        finish_osc_media_payload(
                            &mut self.iterm2_multipart_file,
                            payload,
                            truncated,
                        )
                    } else {
                        None
                    };
                    self.state = TerminalMediaSequenceState::Ground;
                    return media.map(TerminalMediaSequenceEvent::media);
                }
                if byte == 0x07 {
                    let truncated = *truncated;
                    let media = finish_osc_media_payload(
                        &mut self.iterm2_multipart_file,
                        payload,
                        truncated,
                    );
                    self.state = TerminalMediaSequenceState::Ground;
                    return media.map(TerminalMediaSequenceEvent::media);
                }
                if byte == 0x1b {
                    *saw_escape = true;
                } else if payload.len() < Self::MAX_OSC_MEDIA_BYTES {
                    payload.push(byte);
                } else {
                    *truncated = true;
                }
                None
            }
            TerminalMediaSequenceState::Apc { payload, saw_escape, truncated } => {
                if byte == 0x9c {
                    let payload = std::mem::take(payload);
                    let event = terminal_media_event_from_apc_payload(
                        &mut self.kitty_chunk,
                        &payload,
                        *truncated,
                    );
                    self.state = TerminalMediaSequenceState::Ground;
                    return event;
                }
                if *saw_escape {
                    let event = if byte == b'\\' {
                        let payload = std::mem::take(payload);
                        terminal_media_event_from_apc_payload(
                            &mut self.kitty_chunk,
                            &payload,
                            *truncated,
                        )
                    } else {
                        None
                    };
                    self.state = TerminalMediaSequenceState::Ground;
                    return event;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                } else if payload.len() < Self::MAX_OSC_MEDIA_BYTES {
                    payload.push(byte);
                } else {
                    *truncated = true;
                }
                None
            }
            TerminalMediaSequenceState::Dcs {
                header_done,
                kind,
                payload,
                saw_escape,
                truncated,
            } => {
                if byte == 0x9c {
                    let completed_media_event = terminal_media_event_from_tmux_dcs_payload(
                        payload, *truncated,
                    )
                    .or_else(|| {
                        (*kind).map(ScreenLineMedia::marker).map(TerminalMediaSequenceEvent::media)
                    });
                    self.state = TerminalMediaSequenceState::Ground;
                    return completed_media_event;
                }
                if *saw_escape {
                    if byte == b'\\' {
                        let completed_media_event =
                            terminal_media_event_from_tmux_dcs_payload(payload, *truncated)
                                .or_else(|| {
                                    (*kind)
                                        .map(ScreenLineMedia::marker)
                                        .map(TerminalMediaSequenceEvent::media)
                                });
                        self.state = TerminalMediaSequenceState::Ground;
                        return completed_media_event;
                    }
                    if byte == 0x1b {
                        if payload.len() < Self::MAX_OSC_MEDIA_BYTES {
                            payload.push(0x1b);
                        } else {
                            *truncated = true;
                        }
                    } else {
                        if payload.len() + 2 <= Self::MAX_OSC_MEDIA_BYTES {
                            payload.push(0x1b);
                            payload.push(byte);
                        } else {
                            *truncated = true;
                        }
                    }
                    *saw_escape = false;
                    return None;
                }
                if byte == 0x1b {
                    *saw_escape = true;
                    return None;
                }
                if payload.len() < Self::MAX_OSC_MEDIA_BYTES {
                    payload.push(byte);
                } else {
                    *truncated = true;
                }
                if !*header_done && (0x40..=0x7e).contains(&byte) {
                    *header_done = true;
                    if byte == b'q' {
                        *kind = Some(ScreenLineMediaKind::Sixel);
                    }
                }
                None
            }
        }
    }
}

fn terminal_media_event_from_apc_payload(
    kitty_chunk: &mut Option<TerminalKittyGraphicsChunk>,
    payload: &[u8],
    truncated: bool,
) -> Option<TerminalMediaSequenceEvent> {
    let kitty_payload = payload.strip_prefix(b"G")?;
    if let Some(query) = terminal_kitty_graphics_query_from_payload(kitty_payload, truncated) {
        *kitty_chunk = None;
        return Some(TerminalMediaSequenceEvent::response(
            terminal_kitty_graphics_query_response_bytes(query),
        ));
    }
    terminal_kitty_graphics_media(kitty_chunk, kitty_payload, truncated)
        .map(TerminalMediaSequenceEvent::media)
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

#[derive(Debug, Clone, Copy)]
struct TerminalKittyGraphicsQuery {
    id: u32,
    supported: bool,
}

fn terminal_kitty_graphics_query_from_payload(
    payload: &[u8],
    truncated: bool,
) -> Option<TerminalKittyGraphicsQuery> {
    if truncated {
        return None;
    }

    let (arguments, data) = split_once_byte(payload, b';').unwrap_or((payload, &[]));
    if terminal_kitty_graphics_control_value(arguments, b"a") != Some(b"q".as_slice()) {
        return None;
    }

    let id = terminal_kitty_graphics_control_value(arguments, b"i")
        .and_then(terminal_u32_parameter)
        .filter(|id| *id > 0)?;
    let transmission = terminal_kitty_graphics_control_value(arguments, b"t")
        .and_then(|value| value.first().copied())
        .unwrap_or(b'd');
    let format = terminal_kitty_graphics_control_value(arguments, b"f").unwrap_or(b"32");
    let compressed = terminal_kitty_graphics_control_value(arguments, b"o")
        .is_some_and(|value| !value.is_empty());
    let supported = transmission == b'd'
        && !compressed
        && terminal_kitty_graphics_query_payload_is_supported(arguments, data, format);

    Some(TerminalKittyGraphicsQuery { id, supported })
}

fn terminal_kitty_graphics_query_payload_is_supported(
    arguments: &[u8],
    data: &[u8],
    format: &[u8],
) -> bool {
    if data.is_empty() {
        return false;
    }
    let Ok(decoded) = BASE64_STANDARD.decode(data) else {
        return false;
    };
    match format {
        b"24" => terminal_kitty_graphics_rgb_payload_len(arguments, 3)
            .is_some_and(|expected| expected == decoded.len()),
        b"32" => terminal_kitty_graphics_rgb_payload_len(arguments, 4)
            .is_some_and(|expected| expected == decoded.len()),
        b"100" => terminal_image_mime_type(&decoded) == Some("image/png"),
        _ => false,
    }
}

fn terminal_kitty_graphics_rgb_payload_len(
    arguments: &[u8],
    bytes_per_pixel: u32,
) -> Option<usize> {
    let width =
        terminal_kitty_graphics_control_value(arguments, b"s").and_then(terminal_u32_parameter)?;
    let height =
        terminal_kitty_graphics_control_value(arguments, b"v").and_then(terminal_u32_parameter)?;
    let bytes =
        u64::from(width).checked_mul(u64::from(height))?.checked_mul(u64::from(bytes_per_pixel))?;
    usize::try_from(bytes).ok()
}

fn terminal_kitty_graphics_query_response_bytes(query: TerminalKittyGraphicsQuery) -> Vec<u8> {
    let status = if query.supported { "OK" } else { "ENOTSUP" };
    format!("\x1b_Gi={};{}\x1b\\", query.id, status).into_bytes()
}

fn terminal_kitty_graphics_control_value<'a>(arguments: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    arguments
        .split(|byte| *byte == b',')
        .filter_map(|argument| split_once_byte(argument, b'='))
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

fn terminal_media_event_from_tmux_dcs_payload(
    payload: &[u8],
    truncated: bool,
) -> Option<TerminalMediaSequenceEvent> {
    let passthrough = terminal_tmux_passthrough_payload(payload, truncated)?;
    let mut tracker = TerminalMediaSequenceTracker::default();
    passthrough.into_iter().find_map(|byte| tracker.advance(byte))
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

fn finish_osc_media_payload(
    multipart: &mut Option<TerminalIterm2MultipartFile>,
    payload: &mut Vec<u8>,
    truncated: bool,
) -> Option<ScreenLineMedia> {
    let payload = std::mem::take(payload);
    terminal_iterm2_multipart_file_media(multipart, &payload, truncated)
        .or_else(|| terminal_media_from_osc_payload(&payload, truncated))
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
        && estimated_base64_decoded_len(data)
            <= TerminalMediaSequenceTracker::MAX_INLINE_IMAGE_BYTES
        && let Some((mime_type, data_base64)) = terminal_inline_image_payload(data)
    {
        media.mime_type = Some(mime_type.to_string());
        media.data_base64 = Some(data_base64);
    }

    media
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
        if estimated_base64_decoded_len_for_bytes(next_len)
            > TerminalMediaSequenceTracker::MAX_INLINE_IMAGE_BYTES
        {
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
            && estimated_base64_decoded_len(&self.data)
                <= TerminalMediaSequenceTracker::MAX_INLINE_IMAGE_BYTES
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
        if estimated_base64_decoded_len_for_bytes(next_len)
            > TerminalMediaSequenceTracker::MAX_INLINE_IMAGE_BYTES
        {
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
    if value.is_empty() || value.chars().any(|ch| ch.is_control()) {
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExtraTextStyle {
    foreground: Option<Option<ScreenColor>>,
    background: Option<Option<ScreenColor>>,
    underline: Option<Option<ScreenUnderlineStyle>>,
    underline_color: Option<Option<ScreenColor>>,
    bold: Option<bool>,
    dim: Option<bool>,
    italic: Option<bool>,
    blink: bool,
    overline: bool,
    inverse: Option<bool>,
    hidden: Option<bool>,
    strikethrough: Option<bool>,
    border: Option<ScreenTextBorderStyle>,
    baseline: Option<ScreenTextBaseline>,
}

impl ExtraTextStyle {
    fn is_plain(&self) -> bool {
        self == &Self::default()
    }

    fn sgr_stack_snapshot(&self) -> Self {
        let mut style = self.clone();
        style.foreground.get_or_insert(None);
        style.background.get_or_insert(None);
        style.underline.get_or_insert(None);
        style.underline_color.get_or_insert(None);
        style.bold.get_or_insert(false);
        style.dim.get_or_insert(false);
        style.italic.get_or_insert(false);
        style.inverse.get_or_insert(false);
        style.hidden.get_or_insert(false);
        style.strikethrough.get_or_insert(false);
        style
    }

    fn restore_sgr_stack_attributes(
        &mut self,
        saved: &ExtraTextStyle,
        attributes: AnsiSgrStackAttributes,
    ) {
        if attributes.foreground {
            self.foreground = saved.foreground.clone();
        }
        if attributes.background {
            self.background = saved.background.clone();
        }
        if attributes.bold {
            self.bold = saved.bold;
        }
        if attributes.dim {
            self.dim = saved.dim;
        }
        if attributes.italic {
            self.italic = saved.italic;
        }
        if attributes.underline {
            self.underline = saved.underline;
            self.underline_color = saved.underline_color.clone();
        }
        if attributes.blink {
            self.blink = saved.blink;
        }
        if attributes.inverse {
            self.inverse = saved.inverse;
        }
        if attributes.hidden {
            self.hidden = saved.hidden;
        }
        if attributes.strikethrough {
            self.strikethrough = saved.strikethrough;
        }
        if attributes.overline {
            self.overline = saved.overline;
        }
        if attributes.border {
            self.border = saved.border;
        }
        if attributes.baseline {
            self.baseline = saved.baseline;
        }
    }

    fn apply_rectangular_screen_style(&mut self, style: &ScreenTextStyle) {
        self.bold = Some(style.bold);
        self.dim = Some(style.dim);
        self.underline = Some(style.underline);
        self.underline_color = Some(style.underline_color.clone());
        self.blink = style.blink;
        self.inverse = Some(style.inverse);
        self.hidden = Some(style.hidden);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExtraSgrStackEntry {
    style: ExtraTextStyle,
    attributes: AnsiSgrStackAttributes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ExtraTextStyleCellKey {
    row: usize,
    col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExtraTextStyleCell {
    ch: char,
    style: ExtraTextStyle,
}

struct TerminalRectangularAttributeControl {
    request: terminal_projection::ansi_sgr::TerminalRectangularAttributeRequest,
    mode: TerminalRectangularAttributeMode,
}

#[derive(Debug, Default)]
struct TerminalRectangularAttributeTracker {
    state: TerminalRectangularAttributeState,
}

#[derive(Debug, Default)]
enum TerminalRectangularAttributeState {
    #[default]
    Ground,
    Escape,
    Csi {
        payload: Vec<u8>,
    },
}

impl TerminalRectangularAttributeTracker {
    fn advance(&mut self, byte: u8) -> Option<TerminalRectangularAttributeControl> {
        match &mut self.state {
            TerminalRectangularAttributeState::Ground => {
                self.state = match byte {
                    0x1b => TerminalRectangularAttributeState::Escape,
                    0x9b => TerminalRectangularAttributeState::Csi { payload: Vec::new() },
                    _ => TerminalRectangularAttributeState::Ground,
                };
                None
            }
            TerminalRectangularAttributeState::Escape => {
                self.state = match byte {
                    b'[' => TerminalRectangularAttributeState::Csi { payload: Vec::new() },
                    0x1b => TerminalRectangularAttributeState::Escape,
                    _ => TerminalRectangularAttributeState::Ground,
                };
                None
            }
            TerminalRectangularAttributeState::Csi { payload } => {
                if byte == 0x1b {
                    self.state = TerminalRectangularAttributeState::Escape;
                    return None;
                }
                if byte < 0x40 {
                    if payload.len() < 128 {
                        payload.push(byte);
                    }
                    return None;
                }

                let mode = match byte {
                    b'r' => Some(TerminalRectangularAttributeMode::Change),
                    b't' => Some(TerminalRectangularAttributeMode::Reverse),
                    _ => None,
                };
                let control = mode.and_then(|mode| {
                    parse_terminal_rectangular_attribute_request(payload)
                        .map(|request| TerminalRectangularAttributeControl { request, mode })
                });
                self.state = TerminalRectangularAttributeState::Ground;
                control
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalScrollRegion {
    top: usize,
    bottom: usize,
}

#[derive(Debug, Default)]
struct TerminalScrollRegionTracker {
    state: TerminalScrollRegionState,
    region: Option<TerminalScrollRegion>,
}

#[derive(Debug, Default)]
enum TerminalScrollRegionState {
    #[default]
    Ground,
    Escape,
    Csi {
        payload: Vec<u8>,
    },
}

impl TerminalScrollRegionTracker {
    fn advance(&mut self, byte: u8, rows: usize) {
        match &mut self.state {
            TerminalScrollRegionState::Ground => {
                self.state = match byte {
                    0x1b => TerminalScrollRegionState::Escape,
                    0x9b => TerminalScrollRegionState::Csi { payload: Vec::new() },
                    _ => TerminalScrollRegionState::Ground,
                };
            }
            TerminalScrollRegionState::Escape => {
                self.state = match byte {
                    b'[' => TerminalScrollRegionState::Csi { payload: Vec::new() },
                    b'c' => {
                        self.region = None;
                        TerminalScrollRegionState::Ground
                    }
                    0x1b => TerminalScrollRegionState::Escape,
                    _ => TerminalScrollRegionState::Ground,
                };
            }
            TerminalScrollRegionState::Csi { payload } => {
                if byte == 0x1b {
                    self.state = TerminalScrollRegionState::Escape;
                    return;
                }
                if byte < 0x40 {
                    if payload.len() < 128 {
                        payload.push(byte);
                    }
                    return;
                }

                let payload_snapshot = payload.clone();
                match byte {
                    b'r' if terminal_csi_payload_has_only_numeric_parameters(&payload_snapshot) => {
                        self.apply_scroll_region_payload(&payload_snapshot, rows);
                    }
                    b'p' if payload_snapshot.contains(&b'!') => {
                        self.region = None;
                    }
                    _ => {}
                }
                self.state = TerminalScrollRegionState::Ground;
            }
        }
    }

    fn active_region(&self, rows: usize) -> Option<TerminalScrollRegion> {
        if rows == 0 {
            return None;
        }
        let full = TerminalScrollRegion { top: 0, bottom: rows - 1 };
        let Some(region) = self.region else {
            return Some(full);
        };
        let top = region.top.min(rows - 1);
        let bottom = region.bottom.min(rows - 1);
        (top < bottom).then_some(TerminalScrollRegion { top, bottom }).or(Some(full))
    }

    fn apply_scroll_region_payload(&mut self, payload: &[u8], rows: usize) {
        if rows == 0 {
            self.region = None;
            return;
        }
        if payload.is_empty() {
            self.region = None;
            return;
        }

        let (top, bottom) = first_two_terminal_csi_parameters(payload);
        let top = usize::from(top.unwrap_or(1).max(1).saturating_sub(1)).min(rows - 1);
        let bottom = usize::from(
            bottom
                .unwrap_or_else(|| u16::try_from(rows).unwrap_or(u16::MAX))
                .max(1)
                .saturating_sub(1),
        )
        .min(rows - 1);
        if top < bottom {
            self.region = Some(TerminalScrollRegion { top, bottom });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalHorizontalRegion {
    left: usize,
    right: usize,
}

#[derive(Debug, Default)]
struct TerminalHorizontalMarginTracker {
    state: TerminalHorizontalMarginState,
    mode: bool,
    region: Option<TerminalHorizontalRegion>,
}

#[derive(Debug, Default)]
enum TerminalHorizontalMarginState {
    #[default]
    Ground,
    Escape,
    Csi {
        payload: Vec<u8>,
    },
}

impl TerminalHorizontalMarginTracker {
    fn advance(&mut self, byte: u8, columns: usize) {
        match &mut self.state {
            TerminalHorizontalMarginState::Ground => {
                self.state = match byte {
                    0x1b => TerminalHorizontalMarginState::Escape,
                    0x9b => TerminalHorizontalMarginState::Csi { payload: Vec::new() },
                    _ => TerminalHorizontalMarginState::Ground,
                };
            }
            TerminalHorizontalMarginState::Escape => {
                self.state = match byte {
                    b'[' => TerminalHorizontalMarginState::Csi { payload: Vec::new() },
                    b'c' => {
                        self.reset();
                        TerminalHorizontalMarginState::Ground
                    }
                    0x1b => TerminalHorizontalMarginState::Escape,
                    _ => TerminalHorizontalMarginState::Ground,
                };
            }
            TerminalHorizontalMarginState::Csi { payload } => {
                if byte == 0x1b {
                    self.state = TerminalHorizontalMarginState::Escape;
                    return;
                }
                if byte < 0x40 {
                    if payload.len() < 128 {
                        payload.push(byte);
                    }
                    return;
                }

                let payload_snapshot = payload.clone();
                match byte {
                    b'h' if terminal_private_csi_payload_contains_mode(&payload_snapshot, 69) => {
                        self.mode = true;
                    }
                    b'l' if terminal_private_csi_payload_contains_mode(&payload_snapshot, 69) => {
                        self.mode = false;
                        self.region = None;
                    }
                    b's' if self.mode
                        && terminal_csi_payload_has_only_numeric_parameters(&payload_snapshot) =>
                    {
                        self.apply_margin_payload(&payload_snapshot, columns);
                    }
                    b'p' if payload_snapshot.contains(&b'!') => self.reset(),
                    _ => {}
                }
                self.state = TerminalHorizontalMarginState::Ground;
            }
        }
    }

    fn reset(&mut self) {
        self.mode = false;
        self.region = None;
    }

    fn active_region(&self, columns: usize) -> Option<TerminalHorizontalRegion> {
        if columns == 0 {
            return None;
        }
        let full = TerminalHorizontalRegion { left: 0, right: columns - 1 };
        if !self.mode {
            return Some(full);
        }
        let Some(region) = self.region else {
            return Some(full);
        };
        let left = region.left.min(columns - 1);
        let right = region.right.min(columns - 1);
        (left < right).then_some(TerminalHorizontalRegion { left, right }).or(Some(full))
    }

    fn apply_margin_payload(&mut self, payload: &[u8], columns: usize) {
        if columns == 0 {
            self.region = None;
            return;
        }
        let (left, right) = first_two_terminal_csi_parameters(payload);
        let left = usize::from(left.unwrap_or(1));
        let right =
            usize::from(right.unwrap_or_else(|| u16::try_from(columns).unwrap_or(u16::MAX)));
        let left = if left == 0 { 0 } else { left - 1 }.min(columns - 1);
        let right = if right == 0 { columns - 1 } else { right.saturating_sub(1).min(columns - 1) };
        if left < right {
            self.region = Some(TerminalHorizontalRegion { left, right });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalColumnControl {
    Insert { count: usize },
    Delete { count: usize },
    ScrollLeft { count: usize },
    ScrollRight { count: usize },
    BackIndex,
    ForwardIndex,
}

#[derive(Debug, Default)]
struct TerminalColumnControlTracker {
    state: TerminalColumnControlState,
}

#[derive(Debug, Default)]
enum TerminalColumnControlState {
    #[default]
    Ground,
    Escape,
    Csi {
        payload: Vec<u8>,
    },
}

impl TerminalColumnControlTracker {
    fn advance(&mut self, byte: u8) -> Option<TerminalColumnControl> {
        match &mut self.state {
            TerminalColumnControlState::Ground => {
                self.state = match byte {
                    0x1b => TerminalColumnControlState::Escape,
                    0x9b => TerminalColumnControlState::Csi { payload: Vec::new() },
                    _ => TerminalColumnControlState::Ground,
                };
                None
            }
            TerminalColumnControlState::Escape => match byte {
                b'6' => {
                    self.state = TerminalColumnControlState::Ground;
                    Some(TerminalColumnControl::BackIndex)
                }
                b'9' => {
                    self.state = TerminalColumnControlState::Ground;
                    Some(TerminalColumnControl::ForwardIndex)
                }
                b'[' => {
                    self.state = TerminalColumnControlState::Csi { payload: Vec::new() };
                    None
                }
                0x1b => {
                    self.state = TerminalColumnControlState::Escape;
                    None
                }
                _ => {
                    self.state = TerminalColumnControlState::Ground;
                    None
                }
            },
            TerminalColumnControlState::Csi { payload } => {
                if byte == 0x1b {
                    self.state = TerminalColumnControlState::Escape;
                    return None;
                }
                if byte < 0x40 {
                    if payload.len() < 128 {
                        payload.push(byte);
                    }
                    return None;
                }

                let count = terminal_first_csi_parameter(payload).unwrap_or(1).max(1) as usize;
                let control = match byte {
                    b'@' if payload.contains(&b' ') => {
                        Some(TerminalColumnControl::ScrollLeft { count })
                    }
                    b'A' if payload.contains(&b' ') => {
                        Some(TerminalColumnControl::ScrollRight { count })
                    }
                    b'}' if payload.contains(&b'\'') => {
                        Some(TerminalColumnControl::Insert { count })
                    }
                    b'~' if payload.contains(&b'\'') => {
                        Some(TerminalColumnControl::Delete { count })
                    }
                    _ => None,
                };
                self.state = TerminalColumnControlState::Ground;
                control
            }
        }
    }
}

#[derive(Clone)]
struct TerminalColumnCopyCell {
    cell: Cell,
    extra_style: ExtraTextStyle,
    protected: bool,
}

#[derive(Clone, Copy)]
struct TerminalColumnControlRowScope {
    row: usize,
    start_col: usize,
    end_col: usize,
    count: usize,
}

fn apply_terminal_column_control(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    region: Option<TerminalScrollRegion>,
    horizontal_region: Option<TerminalHorizontalRegion>,
    control: TerminalColumnControl,
) {
    let Some(region) = region else {
        return;
    };
    let Some(horizontal_region) = horizontal_region else {
        return;
    };
    let columns = term.columns();
    if columns == 0 {
        return;
    }
    let left_col = horizontal_region.left.min(columns - 1);
    let right_col = horizontal_region.right.min(columns - 1);
    if left_col >= right_col {
        return;
    }
    let end_col = right_col + 1;

    match control {
        TerminalColumnControl::BackIndex | TerminalColumnControl::ForwardIndex => {
            apply_terminal_horizontal_index_control(
                term,
                extra_styles,
                protected_cells,
                region,
                TerminalHorizontalRegion { left: left_col, right: right_col },
                control,
            );
            return;
        }
        TerminalColumnControl::Insert { .. }
        | TerminalColumnControl::Delete { .. }
        | TerminalColumnControl::ScrollLeft { .. }
        | TerminalColumnControl::ScrollRight { .. } => {}
    }

    let (start_col, count) = match control {
        TerminalColumnControl::Insert { count } | TerminalColumnControl::Delete { count } => {
            let Some((cursor_row, cursor_col)) = terminal_cursor_position(term) else {
                return;
            };
            if cursor_col < left_col
                || cursor_col > right_col
                || cursor_row < region.top
                || cursor_row > region.bottom
            {
                return;
            }
            (cursor_col, count.min(end_col - cursor_col))
        }
        TerminalColumnControl::ScrollLeft { count }
        | TerminalColumnControl::ScrollRight { count } => (left_col, count.min(end_col - left_col)),
        TerminalColumnControl::BackIndex | TerminalColumnControl::ForwardIndex => unreachable!(),
    };
    if count == 0 {
        return;
    }

    for row in region.top..=region.bottom {
        apply_terminal_column_control_to_row(
            term,
            extra_styles,
            protected_cells,
            TerminalColumnControlRowScope { row, start_col, end_col, count },
            control,
        );
    }
}

fn apply_terminal_column_control_to_row(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    scope: TerminalColumnControlRowScope,
    control: TerminalColumnControl,
) {
    let TerminalColumnControlRowScope { row, start_col, end_col, count } = scope;
    let columns = term.columns();
    let end_col = end_col.min(columns);
    if row >= term.screen_lines() || start_col >= end_col {
        return;
    }

    let blank_cell = terminal_blank_column_copy_cell(term);
    clear_terminal_column_boundary(
        term,
        extra_styles,
        protected_cells,
        row,
        start_col,
        &blank_cell,
    );
    match control {
        TerminalColumnControl::Insert { .. } => {
            clear_terminal_column_boundary(
                term,
                extra_styles,
                protected_cells,
                row,
                end_col.saturating_sub(count),
                &blank_cell,
            );
        }
        TerminalColumnControl::Delete { .. } => {
            clear_terminal_column_boundary(
                term,
                extra_styles,
                protected_cells,
                row,
                start_col.saturating_add(count),
                &blank_cell,
            );
        }
        TerminalColumnControl::ScrollLeft { .. } => {
            clear_terminal_column_boundary(
                term,
                extra_styles,
                protected_cells,
                row,
                count,
                &blank_cell,
            );
        }
        TerminalColumnControl::ScrollRight { .. } => {
            clear_terminal_column_boundary(
                term,
                extra_styles,
                protected_cells,
                row,
                columns.saturating_sub(count).saturating_sub(1),
                &blank_cell,
            );
        }
        TerminalColumnControl::BackIndex | TerminalColumnControl::ForwardIndex => unreachable!(),
    }

    let snapshot = terminal_column_copy_snapshot(term, extra_styles, protected_cells, row);
    clear_terminal_column_range(
        term,
        extra_styles,
        protected_cells,
        row,
        start_col,
        end_col,
        &blank_cell,
    );

    for target_col in start_col..end_col {
        let copied = match control {
            TerminalColumnControl::Insert { .. } if target_col < start_col + count => None,
            TerminalColumnControl::Insert { .. } => terminal_column_copy_cell_for_write(
                &snapshot,
                target_col - count,
                start_col,
                target_col,
                end_col,
                &blank_cell,
            ),
            TerminalColumnControl::Delete { .. } if target_col + count < end_col => {
                terminal_column_copy_cell_for_write(
                    &snapshot,
                    target_col + count,
                    start_col + count,
                    target_col,
                    end_col,
                    &blank_cell,
                )
            }
            TerminalColumnControl::Delete { .. } => None,
            TerminalColumnControl::ScrollLeft { .. } if target_col + count < end_col => {
                terminal_column_copy_cell_for_write(
                    &snapshot,
                    target_col + count,
                    start_col + count,
                    target_col,
                    end_col,
                    &blank_cell,
                )
            }
            TerminalColumnControl::ScrollLeft { .. } => None,
            TerminalColumnControl::ScrollRight { .. } if target_col < start_col + count => None,
            TerminalColumnControl::ScrollRight { .. } => terminal_column_copy_cell_for_write(
                &snapshot,
                target_col - count,
                start_col,
                target_col,
                end_col,
                &blank_cell,
            ),
            TerminalColumnControl::BackIndex | TerminalColumnControl::ForwardIndex => {
                unreachable!()
            }
        };
        write_terminal_column_cell(
            term,
            extra_styles,
            protected_cells,
            row,
            target_col,
            copied,
            &blank_cell,
        );
    }
}

fn apply_terminal_horizontal_index_control(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    region: TerminalScrollRegion,
    horizontal_region: TerminalHorizontalRegion,
    control: TerminalColumnControl,
) {
    let Some((cursor_row, cursor_col)) = terminal_cursor_position(term) else {
        return;
    };
    let columns = term.columns();
    if columns == 0 {
        return;
    }
    let left_col = horizontal_region.left.min(columns - 1);
    let right_col = horizontal_region.right.min(columns - 1);
    if left_col >= right_col {
        return;
    }

    match control {
        TerminalColumnControl::BackIndex if cursor_col > left_col => {
            ansi::Handler::goto(term, cursor_row as i32, cursor_col - 1);
        }
        TerminalColumnControl::BackIndex => {
            apply_terminal_column_control(
                term,
                extra_styles,
                protected_cells,
                Some(region),
                Some(TerminalHorizontalRegion { left: left_col, right: right_col }),
                TerminalColumnControl::ScrollRight { count: 1 },
            );
        }
        TerminalColumnControl::ForwardIndex if cursor_col < right_col => {
            ansi::Handler::goto(term, cursor_row as i32, cursor_col + 1);
        }
        TerminalColumnControl::ForwardIndex => {
            apply_terminal_column_control(
                term,
                extra_styles,
                protected_cells,
                Some(region),
                Some(TerminalHorizontalRegion { left: left_col, right: right_col }),
                TerminalColumnControl::ScrollLeft { count: 1 },
            );
        }
        TerminalColumnControl::Insert { .. }
        | TerminalColumnControl::Delete { .. }
        | TerminalColumnControl::ScrollLeft { .. }
        | TerminalColumnControl::ScrollRight { .. } => unreachable!(),
    }
}

fn terminal_column_copy_snapshot(
    term: &Term<EmulatorEventListener>,
    extra_styles: &ExtraTextStyleOverlay,
    protected_cells: &TerminalProtectedCellOverlay,
    row: usize,
) -> Vec<TerminalColumnCopyCell> {
    (0..term.columns())
        .map(|col| {
            let cell = terminal_cell_at(term, row, col).unwrap_or_default();
            TerminalColumnCopyCell {
                extra_style: extra_styles.style_for_cell(row, col, cell.c),
                protected: protected_cells.is_cell_protected(row, col),
                cell,
            }
        })
        .collect()
}

fn terminal_column_copy_cell_for_write(
    row: &[TerminalColumnCopyCell],
    source_col: usize,
    source_start_col: usize,
    target_col: usize,
    columns: usize,
    blank_cell: &TerminalColumnCopyCell,
) -> Option<TerminalColumnCopyCell> {
    let copied = row.get(source_col)?;
    if copied.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
        let previous_wide_cell_is_copied = source_col > source_start_col
            && source_col
                .checked_sub(1)
                .and_then(|previous_col| row.get(previous_col))
                .is_some_and(|previous| previous.cell.flags.contains(Flags::WIDE_CHAR));
        return (!previous_wide_cell_is_copied).then(|| blank_cell.clone());
    }
    if copied.cell.flags.contains(Flags::WIDE_CHAR) && target_col.saturating_add(2) > columns {
        return Some(blank_cell.clone());
    }
    Some(copied.clone())
}

fn write_terminal_column_cell(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    row: usize,
    col: usize,
    copied: Option<TerminalColumnCopyCell>,
    blank_cell: &TerminalColumnCopyCell,
) {
    let copied = copied.unwrap_or_else(|| blank_cell.clone());
    term.grid_mut()[Line::from(row)][Column(col)] = copied.cell.clone();
    extra_styles.write_cell_style(row, col, copied.cell.c, copied.extra_style.clone());
    protected_cells.write_protection_snapshot(
        row,
        col,
        copied.cell.c,
        copied.cell,
        copied.extra_style,
        copied.protected,
    );
}

fn clear_terminal_column_boundary(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    row: usize,
    col: usize,
    blank_cell: &TerminalColumnCopyCell,
) {
    if col >= term.columns() {
        return;
    }
    let (start, end) = terminal_wide_cluster_bounds(term, row, col);
    if end.saturating_sub(start) <= 1 {
        return;
    }
    clear_terminal_column_range(term, extra_styles, protected_cells, row, start, end, blank_cell);
}

fn clear_terminal_column_range(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    row: usize,
    start_col: usize,
    end_col: usize,
    blank_cell: &TerminalColumnCopyCell,
) {
    if row >= term.screen_lines() || start_col >= end_col {
        return;
    }
    let columns = term.columns();
    let end_col = end_col.min(columns);
    for col in start_col..end_col {
        term.grid_mut()[Line::from(row)][Column(col)] = blank_cell.cell.clone();
    }
    extra_styles.clear_row_range(row, start_col, end_col);
    protected_cells.clear_row_range(row, start_col, end_col);
}

fn terminal_blank_column_copy_cell(term: &Term<EmulatorEventListener>) -> TerminalColumnCopyCell {
    TerminalColumnCopyCell {
        cell: terminal_blank_cell(term),
        extra_style: ExtraTextStyle::default(),
        protected: false,
    }
}

fn terminal_blank_cell(term: &Term<EmulatorEventListener>) -> Cell {
    Cell::from(term.grid().cursor.template.bg)
}

fn terminal_csi_payload_has_only_numeric_parameters(payload: &[u8]) -> bool {
    payload.iter().all(|byte| matches!(byte, b'0'..=b'9' | b';' | b':'))
}

fn terminal_private_csi_payload_contains_mode(payload: &[u8], mode: u16) -> bool {
    let Some(private_payload) = payload.strip_prefix(b"?") else {
        return false;
    };
    private_payload
        .split(|byte| matches!(byte, b';' | b':'))
        .filter_map(|part| terminal_csi_parameter_part(Some(part)))
        .any(|part| part == mode)
}

fn first_two_terminal_csi_parameters(payload: &[u8]) -> (Option<u16>, Option<u16>) {
    let mut parts = payload.split(|byte| matches!(byte, b';' | b':'));
    (terminal_csi_parameter_part(parts.next()), terminal_csi_parameter_part(parts.next()))
}

fn terminal_first_csi_parameter(payload: &[u8]) -> Option<u16> {
    terminal_csi_parameter_part(payload.split(|byte| matches!(byte, b';' | b':')).next())
}

fn terminal_csi_parameter_part(part: Option<&[u8]>) -> Option<u16> {
    let digits = part?.iter().copied().filter(u8::is_ascii_digit).collect::<Vec<_>>();
    if digits.is_empty() {
        return None;
    }
    std::str::from_utf8(&digits).ok()?.parse::<u16>().ok().map(|value| value.min(i16::MAX as u16))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalRectangularAreaControl {
    Fill(TerminalRectangularFillRequest),
    Erase(TerminalRectangularArea),
    SelectiveErase(TerminalRectangularArea),
    Copy(TerminalRectangularCopyRequest),
}

#[derive(Debug, Default)]
struct TerminalRectangularAreaTracker {
    state: TerminalRectangularAreaState,
}

#[derive(Debug, Default)]
enum TerminalRectangularAreaState {
    #[default]
    Ground,
    Escape,
    Csi {
        payload: Vec<u8>,
    },
}

impl TerminalRectangularAreaTracker {
    fn advance(&mut self, byte: u8) -> Option<TerminalRectangularAreaControl> {
        match &mut self.state {
            TerminalRectangularAreaState::Ground => {
                self.state = match byte {
                    0x1b => TerminalRectangularAreaState::Escape,
                    0x9b => TerminalRectangularAreaState::Csi { payload: Vec::new() },
                    _ => TerminalRectangularAreaState::Ground,
                };
                None
            }
            TerminalRectangularAreaState::Escape => {
                self.state = match byte {
                    b'[' => TerminalRectangularAreaState::Csi { payload: Vec::new() },
                    0x1b => TerminalRectangularAreaState::Escape,
                    _ => TerminalRectangularAreaState::Ground,
                };
                None
            }
            TerminalRectangularAreaState::Csi { payload } => {
                if byte == 0x1b {
                    self.state = TerminalRectangularAreaState::Escape;
                    return None;
                }
                if byte < 0x40 {
                    if payload.len() < 128 {
                        payload.push(byte);
                    }
                    return None;
                }

                let control = match byte {
                    b'x' => parse_terminal_rectangular_fill_request(payload)
                        .map(TerminalRectangularAreaControl::Fill),
                    b'z' => parse_terminal_rectangular_area(payload)
                        .map(TerminalRectangularAreaControl::Erase),
                    b'{' => parse_terminal_rectangular_area(payload)
                        .map(TerminalRectangularAreaControl::SelectiveErase),
                    b'v' => parse_terminal_rectangular_copy_request(payload)
                        .map(TerminalRectangularAreaControl::Copy),
                    _ => None,
                };
                self.state = TerminalRectangularAreaState::Ground;
                control
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedTerminalRectangle {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

fn apply_terminal_rectangular_area_control(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    control: TerminalRectangularAreaControl,
) {
    match control {
        TerminalRectangularAreaControl::Fill(request) => {
            fill_terminal_rectangular_area(term, extra_styles, protected_cells, request);
        }
        TerminalRectangularAreaControl::Erase(area) => {
            erase_terminal_rectangular_area(term, extra_styles, protected_cells, area);
        }
        TerminalRectangularAreaControl::SelectiveErase(area) => {
            protected_cells.apply_selective_rectangular_erase(term, extra_styles, area);
        }
        TerminalRectangularAreaControl::Copy(request) => {
            copy_terminal_rectangular_area(term, extra_styles, protected_cells, request);
        }
    }
}

fn fill_terminal_rectangular_area(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    request: TerminalRectangularFillRequest,
) {
    let Some(ch) = char::from_u32(request.codepoint) else {
        return;
    };
    if ch.is_control() || ch.width().unwrap_or(0) != 1 {
        return;
    }
    let Some(rect) = resolve_terminal_rectangle(term, request.area) else {
        return;
    };

    let original_cursor = term.grid().cursor.clone();
    let insert_mode = term.mode().contains(TermMode::INSERT);
    let origin_mode = term.mode().contains(TermMode::ORIGIN);
    if insert_mode {
        ansi::Handler::unset_mode(term, ansi::NamedMode::Insert.into());
    }
    if origin_mode {
        ansi::Handler::unset_private_mode(term, ansi::NamedPrivateMode::Origin.into());
    }

    let current_extra_style = extra_styles.current.clone();
    for row in rect.top..=rect.bottom {
        for col in rect.left..=rect.right {
            ansi::Handler::goto(term, row as i32, col);
            ansi::Handler::input(term, ch);
            extra_styles.write_cell_style(row, col, ch, current_extra_style.clone());
            protected_cells.write_cell(term, extra_styles, row, col, ch, 1);
        }
    }

    if origin_mode {
        ansi::Handler::set_private_mode(term, ansi::NamedPrivateMode::Origin.into());
    }
    if insert_mode {
        ansi::Handler::set_mode(term, ansi::NamedMode::Insert.into());
    }
    term.grid_mut().cursor = original_cursor;
}

fn erase_terminal_rectangular_area(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    area: TerminalRectangularArea,
) {
    let Some(rect) = resolve_terminal_rectangle(term, area) else {
        return;
    };

    let blank_cell = terminal_blank_cell(term);
    for row in rect.top..=rect.bottom {
        let mut col = rect.left;
        while col <= rect.right {
            let (start, end) = terminal_wide_cluster_bounds(term, row, col);
            for target_col in start..end {
                term.grid_mut()[Line::from(row)][Column(target_col)] = blank_cell.clone();
            }
            extra_styles.clear_row_range(row, start, end);
            protected_cells.clear_row_range(row, start, end);
            col = col.saturating_add(1);
        }
    }
}

#[derive(Clone)]
struct TerminalRectangularCopyCell {
    cell: Cell,
    extra_style: ExtraTextStyle,
    protected: bool,
}

fn copy_terminal_rectangular_area(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    request: TerminalRectangularCopyRequest,
) {
    let Some(source) = resolve_terminal_rectangle(term, request.source) else {
        return;
    };
    let rows = term.screen_lines();
    let columns = term.columns();
    if rows == 0 || columns == 0 {
        return;
    }

    let destination_top = usize::from(request.destination_top.saturating_sub(1));
    let destination_left = usize::from(request.destination_left.saturating_sub(1));
    if destination_top >= rows || destination_left >= columns {
        return;
    }

    let snapshot = terminal_rectangular_copy_snapshot(term, extra_styles, protected_cells, source);
    let copy_height = snapshot.len().min(rows.saturating_sub(destination_top));
    let copy_width = snapshot.first().map(Vec::len).unwrap_or(0).min(columns - destination_left);
    if copy_height == 0 || copy_width == 0 {
        return;
    }

    for row_offset in 0..copy_height {
        let row = destination_top + row_offset;
        clear_terminal_rectangular_copy_destination_row(
            term,
            extra_styles,
            protected_cells,
            row,
            destination_left,
            destination_left + copy_width,
        );
    }

    let blank_cell = terminal_blank_cell(term);
    for (row_offset, copied_row) in snapshot.iter().take(copy_height).enumerate() {
        let row = destination_top + row_offset;
        for col_offset in 0..copy_width {
            let col = destination_left + col_offset;
            let Some(copied) =
                terminal_rectangular_copy_cell_for_write(copied_row, col_offset, &blank_cell)
            else {
                continue;
            };
            term.grid_mut()[Line::from(row)][Column(col)] = copied.cell.clone();
            extra_styles.write_cell_style(row, col, copied.cell.c, copied.extra_style.clone());
            protected_cells.write_protection_snapshot(
                row,
                col,
                copied.cell.c,
                copied.cell.clone(),
                copied.extra_style.clone(),
                copied.protected,
            );
        }
    }
}

fn terminal_rectangular_copy_snapshot(
    term: &Term<EmulatorEventListener>,
    extra_styles: &ExtraTextStyleOverlay,
    protected_cells: &TerminalProtectedCellOverlay,
    source: ResolvedTerminalRectangle,
) -> Vec<Vec<TerminalRectangularCopyCell>> {
    let width = source.right.saturating_sub(source.left).saturating_add(1);
    (source.top..=source.bottom)
        .map(|row| {
            (0..width)
                .map(|offset| {
                    let col = source.left + offset;
                    let cell = terminal_cell_at(term, row, col).unwrap_or_default();
                    TerminalRectangularCopyCell {
                        extra_style: extra_styles.style_for_cell(row, col, cell.c),
                        protected: protected_cells.is_cell_protected(row, col),
                        cell,
                    }
                })
                .collect()
        })
        .collect()
}

fn clear_terminal_rectangular_copy_destination_row(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    protected_cells: &mut TerminalProtectedCellOverlay,
    row: usize,
    start_col: usize,
    end_col: usize,
) {
    let columns = term.columns();
    let mut clear_ranges = Vec::new();
    for col in start_col..end_col.min(columns) {
        clear_ranges.push(terminal_wide_cluster_bounds(term, row, col));
    }

    for (start, end) in clear_ranges {
        let blank_cell = terminal_blank_cell(term);
        for col in start..end {
            term.grid_mut()[Line::from(row)][Column(col)] = blank_cell.clone();
        }
        extra_styles.clear_row_range(row, start, end);
        protected_cells.clear_row_range(row, start, end);
    }
}

fn terminal_rectangular_copy_cell_for_write(
    row: &[TerminalRectangularCopyCell],
    offset: usize,
    blank_cell: &Cell,
) -> Option<TerminalRectangularCopyCell> {
    let copied = row.get(offset)?;
    if copied.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
        let previous_wide_cell_is_copied = offset
            .checked_sub(1)
            .and_then(|previous_offset| row.get(previous_offset))
            .is_some_and(|previous| previous.cell.flags.contains(Flags::WIDE_CHAR));
        return (!previous_wide_cell_is_copied).then_some(TerminalRectangularCopyCell {
            cell: blank_cell.clone(),
            extra_style: ExtraTextStyle::default(),
            protected: copied.protected,
        });
    }
    if copied.cell.flags.contains(Flags::WIDE_CHAR) && offset.saturating_add(2) > row.len() {
        return Some(TerminalRectangularCopyCell {
            cell: blank_cell.clone(),
            extra_style: ExtraTextStyle::default(),
            protected: copied.protected,
        });
    }
    Some(copied.clone())
}

fn resolve_terminal_rectangle(
    term: &Term<EmulatorEventListener>,
    area: TerminalRectangularArea,
) -> Option<ResolvedTerminalRectangle> {
    let rows = term.screen_lines();
    let columns = term.columns();
    if rows == 0 || columns == 0 {
        return None;
    }

    let top = usize::from(area.top.saturating_sub(1)).min(rows.saturating_sub(1));
    let bottom = usize::from(
        area.bottom.unwrap_or_else(|| u16::try_from(rows).unwrap_or(u16::MAX)).saturating_sub(1),
    )
    .min(rows.saturating_sub(1));
    let left = usize::from(area.left.saturating_sub(1)).min(columns.saturating_sub(1));
    let right = usize::from(
        area.right.unwrap_or_else(|| u16::try_from(columns).unwrap_or(u16::MAX)).saturating_sub(1),
    )
    .min(columns.saturating_sub(1));

    (top <= bottom && left <= right).then_some(ResolvedTerminalRectangle {
        top,
        bottom,
        left,
        right,
    })
}

fn terminal_wide_cluster_bounds(
    term: &Term<EmulatorEventListener>,
    row: usize,
    col: usize,
) -> (usize, usize) {
    let columns = term.columns();
    if row >= term.screen_lines() || col >= columns {
        return (col, col);
    }

    let cell = &term.grid()[Line::from(row)][Column(col)];
    if cell.flags.contains(Flags::WIDE_CHAR_SPACER) && col > 0 {
        let start = col - 1;
        return (start, start.saturating_add(2).min(columns));
    }
    if cell.flags.contains(Flags::WIDE_CHAR) {
        return (col, col.saturating_add(2).min(columns));
    }
    (col, col.saturating_add(1).min(columns))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalProtectionControl {
    ProtectedMode(bool),
    SelectiveLineErase(u16),
    SelectiveDisplayErase(u16),
}

impl TerminalProtectionControl {
    fn is_selective_erase(self) -> bool {
        matches!(
            self,
            TerminalProtectionControl::SelectiveLineErase(_)
                | TerminalProtectionControl::SelectiveDisplayErase(_)
        )
    }
}

#[derive(Debug, Default)]
struct TerminalProtectionTracker {
    state: TerminalProtectionTrackerState,
}

#[derive(Debug, Default)]
enum TerminalProtectionTrackerState {
    #[default]
    Ground,
    Escape,
    Csi {
        payload: Vec<u8>,
    },
}

impl TerminalProtectionTracker {
    fn advance(&mut self, byte: u8) -> Option<TerminalProtectionControl> {
        match &mut self.state {
            TerminalProtectionTrackerState::Ground => {
                if byte == 0x1b {
                    self.state = TerminalProtectionTrackerState::Escape;
                }
                None
            }
            TerminalProtectionTrackerState::Escape => match byte {
                b'[' => {
                    self.state = TerminalProtectionTrackerState::Csi { payload: Vec::new() };
                    None
                }
                b'V' => {
                    self.state = TerminalProtectionTrackerState::Ground;
                    Some(TerminalProtectionControl::ProtectedMode(true))
                }
                b'W' => {
                    self.state = TerminalProtectionTrackerState::Ground;
                    Some(TerminalProtectionControl::ProtectedMode(false))
                }
                0x1b => None,
                _ => {
                    self.state = TerminalProtectionTrackerState::Ground;
                    None
                }
            },
            TerminalProtectionTrackerState::Csi { payload } => {
                if byte == 0x1b {
                    self.state = TerminalProtectionTrackerState::Escape;
                    return None;
                }
                if byte < 0x40 {
                    if payload.len() < 128 {
                        payload.push(byte);
                    }
                    return None;
                }

                let control = terminal_protection_control_from_csi(payload, byte);
                self.state = TerminalProtectionTrackerState::Ground;
                control
            }
        }
    }
}

fn terminal_protection_control_from_csi(
    payload: &[u8],
    final_byte: u8,
) -> Option<TerminalProtectionControl> {
    match final_byte {
        b'q' if payload.contains(&b'"') => Some(TerminalProtectionControl::ProtectedMode(
            first_csi_parameter_u16(payload).unwrap_or(0) == 1,
        )),
        b'K' => payload.strip_prefix(b"?").map(|payload| {
            TerminalProtectionControl::SelectiveLineErase(
                first_csi_parameter_u16(payload).unwrap_or(0),
            )
        }),
        b'J' => payload.strip_prefix(b"?").map(|payload| {
            TerminalProtectionControl::SelectiveDisplayErase(
                first_csi_parameter_u16(payload).unwrap_or(0),
            )
        }),
        _ => None,
    }
}

fn first_csi_parameter_u16(payload: &[u8]) -> Option<u16> {
    let first = payload.split(|byte| *byte == b';' || *byte == b':').next()?;
    let digits = first
        .iter()
        .copied()
        .skip_while(|byte| !byte.is_ascii_digit())
        .take_while(u8::is_ascii_digit)
        .collect::<Vec<_>>();
    if digits.is_empty() {
        return None;
    }
    parse_terminal_mode_query_number(&digits)
}

#[derive(Debug, Clone)]
struct TerminalProtectedCell {
    ch: char,
    cell: Cell,
    extra_style: ExtraTextStyle,
}

#[derive(Debug, Clone, Default)]
struct TerminalProtectedCellOverlay {
    protected_mode: bool,
    cells: HashMap<ExtraTextStyleCellKey, TerminalProtectedCell>,
}

impl TerminalProtectedCellOverlay {
    fn clear(&mut self) {
        self.protected_mode = false;
        self.cells.clear();
    }

    fn clear_cells(&mut self) {
        self.cells.clear();
    }

    fn set_protected_mode(&mut self, protected: bool) {
        self.protected_mode = protected;
    }

    fn is_cell_protected(&self, row: usize, col: usize) -> bool {
        self.cells.contains_key(&ExtraTextStyleCellKey { row, col })
    }

    fn write_protection_snapshot(
        &mut self,
        row: usize,
        col: usize,
        ch: char,
        cell: Cell,
        extra_style: ExtraTextStyle,
        protected: bool,
    ) {
        let key = ExtraTextStyleCellKey { row, col };
        if protected {
            self.cells.insert(key, TerminalProtectedCell { ch, cell, extra_style });
        } else {
            self.cells.remove(&key);
        }
    }

    fn write_cell(
        &mut self,
        term: &Term<EmulatorEventListener>,
        extra_styles: &ExtraTextStyleOverlay,
        row: usize,
        col: usize,
        ch: char,
        width: usize,
    ) {
        let columns = term.columns();
        let end_col = col.saturating_add(width.max(1)).min(columns);
        if col >= end_col || row >= term.screen_lines() {
            return;
        }

        for target_col in col..end_col {
            let key = ExtraTextStyleCellKey { row, col: target_col };
            if self.protected_mode {
                if let Some(cell) = terminal_cell_at(term, row, target_col) {
                    let cell_ch = cell.c;
                    let extra_style = extra_styles.style_for_cell(row, target_col, cell_ch);
                    self.cells.insert(
                        key,
                        TerminalProtectedCell {
                            ch: if target_col == col { ch } else { cell_ch },
                            cell,
                            extra_style,
                        },
                    );
                }
            } else {
                self.cells.remove(&key);
            }
        }
    }

    fn clear_row_range(&mut self, row: usize, start_col: usize, end_col: usize) {
        self.cells.retain(|key, _| key.row != row || key.col < start_col || key.col >= end_col);
    }

    fn clear_row(&mut self, row: usize) {
        self.cells.retain(|key, _| key.row != row);
    }

    fn clear_rows_above(&mut self, row: usize) {
        self.cells.retain(|key, _| key.row >= row);
    }

    fn clear_rows_below(&mut self, row: usize) {
        self.cells.retain(|key, _| key.row <= row);
    }

    fn clear_screen_for_mode(
        &mut self,
        row: usize,
        col: usize,
        rows: usize,
        columns: usize,
        mode: &ansi::ClearMode,
    ) {
        match mode {
            ansi::ClearMode::Below => {
                self.clear_rows_below(row);
                self.clear_row_range(row, col, columns);
            }
            ansi::ClearMode::Above => {
                self.clear_rows_above(row);
                self.clear_row_range(row, 0, col.saturating_add(1).min(columns));
            }
            ansi::ClearMode::All => self.clear_cells(),
            ansi::ClearMode::Saved => {
                let _ = rows;
            }
        }
    }

    fn apply_selective_line_erase(
        &mut self,
        term: &mut Term<EmulatorEventListener>,
        extra_styles: &mut ExtraTextStyleOverlay,
        mode: u16,
    ) {
        let Some((row, col)) = terminal_cursor_position(term) else {
            return;
        };
        let columns = term.columns();
        let (start_col, end_col) = line_selective_erase_bounds(mode, col, columns);
        self.clear_unprotected_row_range(term, extra_styles, row, start_col, end_col);
        self.restore_row_range(term, extra_styles, row, start_col, end_col);
    }

    fn apply_selective_display_erase(
        &mut self,
        term: &mut Term<EmulatorEventListener>,
        extra_styles: &mut ExtraTextStyleOverlay,
        mode: u16,
    ) {
        let Some((row, col)) = terminal_cursor_position(term) else {
            return;
        };
        let rows = term.screen_lines();
        let columns = term.columns();
        match mode {
            1 => {
                for target_row in 0..row {
                    self.clear_unprotected_row_range(term, extra_styles, target_row, 0, columns);
                    self.restore_row_range(term, extra_styles, target_row, 0, columns);
                }
                self.clear_unprotected_row_range(
                    term,
                    extra_styles,
                    row,
                    0,
                    col.saturating_add(1).min(columns),
                );
                self.restore_row_range(
                    term,
                    extra_styles,
                    row,
                    0,
                    col.saturating_add(1).min(columns),
                );
            }
            2 | 3 => {
                for target_row in 0..rows {
                    self.clear_unprotected_row_range(term, extra_styles, target_row, 0, columns);
                    self.restore_row_range(term, extra_styles, target_row, 0, columns);
                }
            }
            _ => {
                self.clear_unprotected_row_range(term, extra_styles, row, col, columns);
                self.restore_row_range(term, extra_styles, row, col, columns);
                for target_row in row.saturating_add(1)..rows {
                    self.clear_unprotected_row_range(term, extra_styles, target_row, 0, columns);
                    self.restore_row_range(term, extra_styles, target_row, 0, columns);
                }
            }
        }
    }

    fn apply_selective_rectangular_erase(
        &mut self,
        term: &mut Term<EmulatorEventListener>,
        extra_styles: &mut ExtraTextStyleOverlay,
        area: TerminalRectangularArea,
    ) {
        let Some(rect) = resolve_terminal_rectangle(term, area) else {
            return;
        };

        for row in rect.top..=rect.bottom {
            let mut col = rect.left;
            while col <= rect.right {
                let (start, end) = terminal_wide_cluster_bounds(term, row, col);
                let blank_cell = terminal_blank_cell(term);
                let protected = (start..end).any(|target_col| {
                    self.cells.contains_key(&ExtraTextStyleCellKey { row, col: target_col })
                });
                if !protected {
                    for target_col in start..end {
                        term.grid_mut()[Line::from(row)][Column(target_col)] = blank_cell.clone();
                    }
                    extra_styles.clear_row_range(row, start, end);
                }
                col = col.saturating_add(1);
            }
            self.restore_row_range(term, extra_styles, row, rect.left, rect.right + 1);
        }
    }

    fn clear_unprotected_row_range(
        &self,
        term: &mut Term<EmulatorEventListener>,
        extra_styles: &mut ExtraTextStyleOverlay,
        row: usize,
        start_col: usize,
        end_col: usize,
    ) {
        let columns = term.columns();
        let end_col = end_col.min(columns);
        if row >= term.screen_lines() || start_col >= end_col {
            return;
        }

        let blank_cell = terminal_blank_cell(term);
        for col in start_col..end_col {
            let key = ExtraTextStyleCellKey { row, col };
            if self.cells.contains_key(&key) {
                continue;
            }
            term.grid_mut()[Line::from(row)][Column(col)] = blank_cell.clone();
            extra_styles.clear_row_range(row, col, col.saturating_add(1));
        }
    }

    fn restore_row_range(
        &self,
        term: &mut Term<EmulatorEventListener>,
        extra_styles: &mut ExtraTextStyleOverlay,
        row: usize,
        start_col: usize,
        end_col: usize,
    ) {
        let cells = self
            .cells
            .iter()
            .filter(|(key, _)| key.row == row && key.col >= start_col && key.col < end_col)
            .map(|(key, protected)| (*key, protected.clone()))
            .collect::<Vec<_>>();

        for (key, protected) in cells {
            restore_terminal_protected_cell(term, extra_styles, key, protected);
        }
    }
}

fn terminal_cell_at(term: &Term<EmulatorEventListener>, row: usize, col: usize) -> Option<Cell> {
    if row >= term.screen_lines() || col >= term.columns() {
        return None;
    }
    Some(term.grid()[Line::from(row)][Column(col)].clone())
}

fn restore_terminal_protected_cell(
    term: &mut Term<EmulatorEventListener>,
    extra_styles: &mut ExtraTextStyleOverlay,
    key: ExtraTextStyleCellKey,
    protected: TerminalProtectedCell,
) {
    if key.row >= term.screen_lines() || key.col >= term.columns() {
        return;
    }
    term.grid_mut()[Line::from(key.row)][Column(key.col)] = protected.cell;
    extra_styles.write_cell_style(key.row, key.col, protected.ch, protected.extra_style);
}

fn line_selective_erase_bounds(mode: u16, col: usize, columns: usize) -> (usize, usize) {
    match mode {
        1 => (0, col.saturating_add(1).min(columns)),
        2 => (0, columns),
        _ => (col, columns),
    }
}

#[derive(Debug, Default)]
struct ExtraTextStyleOverlay {
    current: ExtraTextStyle,
    sgr_state: ExtraTextStyle,
    stack: Vec<ExtraSgrStackEntry>,
    cells: HashMap<ExtraTextStyleCellKey, ExtraTextStyleCell>,
}

impl ExtraTextStyleOverlay {
    fn clear(&mut self) {
        self.current = ExtraTextStyle::default();
        self.sgr_state = ExtraTextStyle::default();
        self.stack.clear();
        self.cells.clear();
    }

    fn clear_cells(&mut self) {
        self.cells.clear();
    }

    fn clear_row(&mut self, row: usize) {
        self.cells.retain(|key, _| key.row != row);
    }

    fn clear_row_range(&mut self, row: usize, start_col: usize, end_col: usize) {
        self.cells.retain(|key, _| key.row != row || key.col < start_col || key.col >= end_col);
    }

    fn clear_rows_above(&mut self, row: usize) {
        self.cells.retain(|key, _| key.row >= row);
    }

    fn clear_rows_below(&mut self, row: usize) {
        self.cells.retain(|key, _| key.row <= row);
    }

    fn set_current_blink(&mut self, blink: bool) {
        self.current.blink = blink;
        self.sgr_state.blink = blink;
    }

    fn push_sgr_state(&mut self, attributes: AnsiSgrStackAttributes) {
        if self.stack.len() >= MAX_EXTRA_SGR_STACK_DEPTH {
            self.stack.remove(0);
        }
        self.stack
            .push(ExtraSgrStackEntry { style: self.sgr_state.sgr_stack_snapshot(), attributes });
    }

    fn pop_sgr_state(&mut self) {
        if let Some(entry) = self.stack.pop() {
            self.sgr_state.restore_sgr_stack_attributes(&entry.style, entry.attributes);
            self.current.restore_sgr_stack_attributes(&entry.style, entry.attributes);
        }
    }

    fn apply_sgr_update(&mut self, update: ExtraSgrUpdate) {
        Self::apply_sgr_update_to_tracked_state(&mut self.sgr_state, &update);
        Self::apply_sgr_update_to_render_overlay(&mut self.current, &update);
    }

    fn apply_sgr_update_to_tracked_state(style: &mut ExtraTextStyle, update: &ExtraSgrUpdate) {
        if update.reset {
            *style = ExtraTextStyle::default();
        }
        if let Some(foreground) = update.foreground.clone() {
            style.foreground = Some(foreground);
        }
        if let Some(background) = update.background.clone() {
            style.background = Some(background);
        }
        if let Some(bold) = update.bold {
            style.bold = Some(bold);
        }
        if let Some(dim) = update.dim {
            style.dim = Some(dim);
        }
        if let Some(italic) = update.italic {
            style.italic = Some(italic);
        }
        if let Some(underline) = update.underline {
            style.underline = Some(underline);
        }
        if let Some(underline_color) = update.underline_color.clone() {
            style.underline_color = Some(underline_color);
        }
        if let Some(overline) = update.overline {
            style.overline = overline;
        }
        if let Some(inverse) = update.inverse {
            style.inverse = Some(inverse);
        }
        if let Some(hidden) = update.hidden {
            style.hidden = Some(hidden);
        }
        if let Some(strikethrough) = update.strikethrough {
            style.strikethrough = Some(strikethrough);
        }
        if update.reset_border {
            style.border = None;
        }
        if let Some(border) = update.border {
            style.border = Some(border);
        }
        if let Some(baseline) = update.baseline {
            style.baseline = baseline;
        }
    }

    fn apply_sgr_update_to_render_overlay(style: &mut ExtraTextStyle, update: &ExtraSgrUpdate) {
        if update.reset {
            *style = ExtraTextStyle::default();
        }
        if let Some(foreground) = update.foreground.clone() {
            style.foreground = render_overlay_color(foreground);
        }
        if let Some(foreground) = update.render_foreground.clone() {
            style.foreground = Some(Some(foreground));
        }
        if let Some(background) = update.background.clone() {
            style.background = render_overlay_color(background);
        }
        if let Some(background) = update.render_background.clone() {
            style.background = Some(Some(background));
        }
        if let Some(underline) = update.underline {
            style.underline = underline.map(Some);
        }
        if let Some(underline_color) = update.underline_color.clone() {
            style.underline_color = underline_color.map(Some);
        }
        if let Some(overline) = update.overline {
            style.overline = overline;
        }
        if update.reset_border {
            style.border = None;
        }
        if let Some(border) = update.border {
            style.border = Some(border);
        }
        if let Some(baseline) = update.baseline {
            style.baseline = baseline;
        }
    }

    fn style_for_cell(&self, row: usize, col: usize, ch: char) -> ExtraTextStyle {
        let key = ExtraTextStyleCellKey { row, col };
        self.cells
            .get(&key)
            .filter(|cell| cell.ch == ch)
            .map(|cell| cell.style.clone())
            .unwrap_or_default()
    }

    fn apply_rectangular_attributes(
        &mut self,
        term: &Term<EmulatorEventListener>,
        control: TerminalRectangularAttributeControl,
    ) {
        let rows = term.screen_lines();
        let cols = term.columns();
        let top = usize::from(control.request.top.saturating_sub(1));
        let bottom = usize::from(
            control
                .request
                .bottom
                .unwrap_or_else(|| u16::try_from(rows).unwrap_or(u16::MAX))
                .saturating_sub(1),
        )
        .min(rows.saturating_sub(1));
        let left = usize::from(control.request.left.saturating_sub(1));
        let right = usize::from(
            control
                .request
                .right
                .unwrap_or_else(|| u16::try_from(cols).unwrap_or(u16::MAX))
                .saturating_sub(1),
        )
        .min(cols.saturating_sub(1));
        if top > bottom || left > right {
            return;
        }

        let content = term.renderable_content();
        let colors = term.colors();
        for indexed in content.display_iter {
            let Ok(row) = usize::try_from(indexed.point.line.0) else {
                continue;
            };
            let col = indexed.point.column.0;
            if row < top || row > bottom || col < left || col > right {
                continue;
            }
            if indexed
                .cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }

            let mut extra_style = self.style_for_cell(row, col, indexed.cell.c);
            let mut style = screen_text_style_from_cell(indexed.cell, extra_style.clone(), colors);
            apply_terminal_rectangular_attribute_actions(
                &mut style,
                &control.request.actions,
                control.mode,
            );
            extra_style.apply_rectangular_screen_style(&style);
            self.write_cell_style(row, col, indexed.cell.c, extra_style);
        }
    }

    fn write_cell(&mut self, row: usize, col: usize, ch: char) {
        self.write_cell_style(row, col, ch, self.current.clone());
    }

    fn write_cell_style(&mut self, row: usize, col: usize, ch: char, style: ExtraTextStyle) {
        let key = ExtraTextStyleCellKey { row, col };
        if style.is_plain() {
            self.cells.remove(&key);
        } else {
            self.cells.insert(key, ExtraTextStyleCell { ch, style });
        }
    }
}

fn render_overlay_color(color: Option<ScreenColor>) -> Option<Option<ScreenColor>> {
    match color {
        Some(ScreenColor::Rgb { .. }) => Some(color),
        Some(ScreenColor::Named { .. } | ScreenColor::Indexed { .. }) | None => None,
    }
}

#[derive(Debug, Default)]
struct ExtraSgrTracker {
    state: ExtraSgrTrackerState,
}

#[derive(Debug, Default)]
enum ExtraSgrTrackerState {
    #[default]
    Ground,
    Escape,
    Csi {
        params: Vec<u8>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExtraSgrUpdate {
    reset: bool,
    foreground: Option<Option<ScreenColor>>,
    background: Option<Option<ScreenColor>>,
    render_foreground: Option<ScreenColor>,
    render_background: Option<ScreenColor>,
    bold: Option<bool>,
    dim: Option<bool>,
    italic: Option<bool>,
    underline: Option<Option<ScreenUnderlineStyle>>,
    underline_color: Option<Option<ScreenColor>>,
    overline: Option<bool>,
    inverse: Option<bool>,
    hidden: Option<bool>,
    strikethrough: Option<bool>,
    reset_border: bool,
    border: Option<ScreenTextBorderStyle>,
    baseline: Option<Option<ScreenTextBaseline>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExtraSgrControl {
    Update(ExtraSgrUpdate),
    Push(AnsiSgrStackAttributes),
    Pop,
}

impl ExtraSgrUpdate {
    fn is_empty(&self) -> bool {
        !self.reset
            && self.foreground.is_none()
            && self.background.is_none()
            && self.render_foreground.is_none()
            && self.render_background.is_none()
            && self.bold.is_none()
            && self.dim.is_none()
            && self.italic.is_none()
            && self.underline.is_none()
            && self.underline_color.is_none()
            && self.overline.is_none()
            && self.inverse.is_none()
            && self.hidden.is_none()
            && self.strikethrough.is_none()
            && !self.reset_border
            && self.border.is_none()
            && self.baseline.is_none()
    }
}

impl ExtraSgrTracker {
    fn advance(&mut self, byte: u8) -> Option<ExtraSgrControl> {
        match &mut self.state {
            ExtraSgrTrackerState::Ground => {
                self.state = match byte {
                    0x1b => ExtraSgrTrackerState::Escape,
                    0x9b => ExtraSgrTrackerState::Csi { params: Vec::new() },
                    _ => ExtraSgrTrackerState::Ground,
                };
                None
            }
            ExtraSgrTrackerState::Escape => {
                self.state = if byte == b'[' {
                    ExtraSgrTrackerState::Csi { params: Vec::new() }
                } else if byte == 0x1b {
                    ExtraSgrTrackerState::Escape
                } else {
                    ExtraSgrTrackerState::Ground
                };
                None
            }
            ExtraSgrTrackerState::Csi { params } => {
                if byte == b'm' {
                    let update = parse_extra_sgr_update(params);
                    self.state = ExtraSgrTrackerState::Ground;
                    return (!update.is_empty()).then_some(ExtraSgrControl::Update(update));
                }

                if let Some(attributes) = parse_xterm_sgr_stack_attributes(params) {
                    let control = match byte {
                        b'{' | b'p' => Some(ExtraSgrControl::Push(attributes)),
                        b'}' | b'q' if params == b"#" => Some(ExtraSgrControl::Pop),
                        _ => None,
                    };
                    if control.is_some() {
                        self.state = ExtraSgrTrackerState::Ground;
                        return control;
                    }
                }

                if byte == 0x1b {
                    self.state = ExtraSgrTrackerState::Escape;
                    return None;
                }

                if matches!(byte, b'0'..=b'9' | b';' | b':' | b'#') && params.len() < 128 {
                    params.push(byte);
                } else {
                    self.state = ExtraSgrTrackerState::Ground;
                }
                None
            }
        }
    }
}

fn parse_extra_sgr_update(params: &[u8]) -> ExtraSgrUpdate {
    if params.is_empty() {
        return extra_sgr_reset_update();
    }

    let text = String::from_utf8_lossy(params);
    let mut update = ExtraSgrUpdate::default();
    let parts = text.split(';').collect::<Vec<_>>();
    let mut index = 0;
    while index < parts.len() {
        let part = parts[index];
        if let Some((target, color)) = parse_colon_sgr_color_part(part) {
            update.apply_color_if_needed(target, color.clone());
            update.apply_render_color_if_needed(target, color);
            index += 1;
            continue;
        }
        if let Some(underline) = parse_colon_sgr_underline_style(part) {
            update.underline = Some(underline);
            index += 1;
            continue;
        }

        let code = part.split(':').next().unwrap_or_default();
        let parsed_code = if code.is_empty() { Ok(0) } else { code.parse::<u16>() };
        match parsed_code {
            Ok(0) => {
                update = extra_sgr_reset_update();
            }
            Ok(1) => update.bold = Some(true),
            Ok(2) => update.dim = Some(true),
            Ok(3) => update.italic = Some(true),
            Ok(38) | Ok(48) | Ok(58) => {
                let target = match code {
                    "38" => SgrColorTarget::Foreground,
                    "48" => SgrColorTarget::Background,
                    "58" => SgrColorTarget::Underline,
                    _ => unreachable!(),
                };
                if let Some((color, consumed)) =
                    parse_semicolon_sgr_color_fields(&parts[index + 1..])
                {
                    update.apply_color_if_needed(target, color);
                    index += consumed;
                }
            }
            Ok(22) => {
                update.bold = Some(false);
                update.dim = Some(false);
            }
            Ok(23) => update.italic = Some(false),
            Ok(39) => update.foreground = Some(None),
            Ok(49) => update.background = Some(None),
            Ok(value @ 30..=37) => update.foreground = native_named_sgr_color(value - 30),
            Ok(value @ 40..=47) => update.background = native_named_sgr_color(value - 40),
            Ok(4) => update.underline = Some(Some(ScreenUnderlineStyle::Single)),
            Ok(21) => update.underline = Some(Some(ScreenUnderlineStyle::Double)),
            Ok(24) => update.underline = Some(None),
            Ok(27) => update.inverse = Some(false),
            Ok(28) => update.hidden = Some(false),
            Ok(29) => update.strikethrough = Some(false),
            Ok(51) => update.border = Some(ScreenTextBorderStyle::Framed),
            Ok(52) => update.border = Some(ScreenTextBorderStyle::Encircled),
            Ok(53) => update.overline = Some(true),
            Ok(54) => {
                update.reset_border = true;
                update.border = None;
            }
            Ok(55) => update.overline = Some(false),
            Ok(59) => update.underline_color = Some(None),
            Ok(73) => update.baseline = Some(Some(ScreenTextBaseline::Superscript)),
            Ok(74) => update.baseline = Some(Some(ScreenTextBaseline::Subscript)),
            Ok(75) => update.baseline = Some(None),
            Ok(7) => update.inverse = Some(true),
            Ok(8) => update.hidden = Some(true),
            Ok(9) => update.strikethrough = Some(true),
            Ok(value @ 90..=97) => update.foreground = native_named_sgr_color(value - 90 + 8),
            Ok(value @ 100..=107) => update.background = native_named_sgr_color(value - 100 + 8),
            _ => {}
        }
        index += 1;
    }
    update
}

fn extra_sgr_reset_update() -> ExtraSgrUpdate {
    ExtraSgrUpdate {
        reset: true,
        overline: Some(false),
        reset_border: true,
        ..ExtraSgrUpdate::default()
    }
}

impl ExtraSgrUpdate {
    fn apply_color(&mut self, target: SgrColorTarget, color: Option<ScreenColor>) {
        match target {
            SgrColorTarget::Foreground => self.foreground = Some(color),
            SgrColorTarget::Background => self.background = Some(color),
            SgrColorTarget::Underline => self.underline_color = Some(color),
        }
    }

    fn apply_color_if_needed(&mut self, target: SgrColorTarget, color: ScreenColor) {
        self.apply_color(target, Some(color));
    }

    fn apply_render_color_if_needed(&mut self, target: SgrColorTarget, color: ScreenColor) {
        match target {
            SgrColorTarget::Foreground => self.render_foreground = Some(color),
            SgrColorTarget::Background => self.render_background = Some(color),
            SgrColorTarget::Underline => {}
        }
    }
}

fn native_named_sgr_color(index: u16) -> Option<Option<ScreenColor>> {
    Some(Some(ScreenColor::Named {
        name: NATIVE_SGR_COLOR_NAMES.get(usize::from(u8::try_from(index).ok()?))?.to_string(),
    }))
}

fn native_named_sgr_index(name: &str) -> Option<u8> {
    NATIVE_SGR_COLOR_NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .and_then(|index| u8::try_from(index).ok())
}

struct ExtraTextStyleHandler<'a> {
    term: &'a mut Term<EmulatorEventListener>,
    response_bytes: &'a Arc<Mutex<Vec<Vec<u8>>>>,
    window_size: &'a Arc<Mutex<WindowSize>>,
    horizontal_margin_mode: bool,
    extra_styles: &'a mut ExtraTextStyleOverlay,
    protected_cells: &'a mut TerminalProtectedCellOverlay,
    media_overlay: &'a mut TerminalMediaOverlay,
    side_effect_overlay: &'a mut TerminalSideEffectOverlay,
    semantic_mark_overlay: &'a mut TerminalSemanticMarkOverlay,
}

impl ExtraTextStyleHandler<'_> {
    fn cursor_position(&self) -> Option<(usize, usize)> {
        let content = self.term.renderable_content();
        let row = usize::try_from(content.cursor.point.line.0).ok()?;
        Some((row, content.cursor.point.column.0))
    }

    fn clear_current_line_effect_metadata(&mut self) {
        if let Some((row, _)) = self.cursor_position() {
            self.media_overlay.clear_row(row);
            self.side_effect_overlay.clear_row(row);
        } else {
            self.media_overlay.clear();
            self.side_effect_overlay.clear();
        }
    }

    fn clear_current_extra_style_range(&mut self, start_col: usize, end_col: usize) {
        let Some((row, _)) = self.cursor_position() else {
            self.extra_styles.clear_cells();
            self.protected_cells.clear_cells();
            return;
        };
        if start_col >= end_col {
            return;
        }
        self.extra_styles.clear_row_range(row, start_col, end_col);
        self.protected_cells.clear_row_range(row, start_col, end_col);
    }

    fn clear_current_semantic_mark_range(&mut self, start_col: usize, end_col: usize) {
        let Some((row, _)) = self.cursor_position() else {
            self.semantic_mark_overlay.clear();
            return;
        };
        self.semantic_mark_overlay.clear_mark_range(row, start_col, end_col);
    }

    fn clear_current_extra_style_tail(&mut self) {
        let Some((_, col)) = self.cursor_position() else {
            self.extra_styles.clear_cells();
            return;
        };
        self.clear_current_extra_style_range(col, self.term.columns());
    }

    fn clear_current_extra_style_chars(&mut self, count: usize) {
        let Some((_, col)) = self.cursor_position() else {
            self.extra_styles.clear_cells();
            return;
        };
        self.clear_current_extra_style_range(
            col,
            col.saturating_add(count).min(self.term.columns()),
        );
    }

    fn clear_current_semantic_mark_tail(&mut self) {
        let Some((_, col)) = self.cursor_position() else {
            self.semantic_mark_overlay.clear();
            return;
        };
        self.clear_current_semantic_mark_range(col, self.term.columns());
    }

    fn clear_current_semantic_mark_chars(&mut self, count: usize) {
        let Some((_, col)) = self.cursor_position() else {
            self.semantic_mark_overlay.clear();
            return;
        };
        self.clear_current_semantic_mark_range(
            col,
            col.saturating_add(count).min(self.term.columns()),
        );
    }

    fn clear_current_line_extra_styles(&mut self, mode: &ansi::LineClearMode) {
        let Some((_, col)) = self.cursor_position() else {
            self.extra_styles.clear_cells();
            self.protected_cells.clear_cells();
            return;
        };
        let columns = self.term.columns();
        match *mode {
            ansi::LineClearMode::Right => {
                self.clear_current_extra_style_range(col, columns);
            }
            ansi::LineClearMode::Left => {
                self.clear_current_extra_style_range(0, col.saturating_add(1).min(columns));
            }
            ansi::LineClearMode::All => {
                if let Some((row, _)) = self.cursor_position() {
                    self.extra_styles.clear_row(row);
                    self.protected_cells.clear_row(row);
                } else {
                    self.extra_styles.clear_cells();
                    self.protected_cells.clear_cells();
                }
            }
        }
    }

    fn clear_current_line_semantic_marks(&mut self, mode: &ansi::LineClearMode) {
        let Some((_, col)) = self.cursor_position() else {
            self.semantic_mark_overlay.clear();
            return;
        };
        let columns = self.term.columns();
        match *mode {
            ansi::LineClearMode::Right => {
                self.clear_current_semantic_mark_range(col, columns);
            }
            ansi::LineClearMode::Left => {
                self.clear_current_semantic_mark_range(0, col.saturating_add(1).min(columns));
            }
            ansi::LineClearMode::All => {
                if let Some((row, _)) = self.cursor_position() {
                    self.semantic_mark_overlay.clear_row(row);
                } else {
                    self.semantic_mark_overlay.clear();
                }
            }
        }
    }

    fn clear_screen_overlays(&mut self) {
        self.extra_styles.clear_cells();
        self.protected_cells.clear_cells();
        self.media_overlay.clear();
        self.side_effect_overlay.clear();
        self.semantic_mark_overlay.clear();
    }

    fn clear_screen_overlays_for_mode(&mut self, mode: &ansi::ClearMode) {
        let Some((row, col)) = self.cursor_position() else {
            if !matches!(*mode, ansi::ClearMode::Saved) {
                self.clear_screen_overlays();
            }
            return;
        };

        match *mode {
            ansi::ClearMode::Below => {
                self.extra_styles.clear_rows_below(row);
                self.clear_current_extra_style_range(col, self.term.columns());
                self.protected_cells.clear_screen_for_mode(
                    row,
                    col,
                    self.term.screen_lines(),
                    self.term.columns(),
                    mode,
                );
                self.media_overlay.clear_rows_below(row);
                self.side_effect_overlay.clear_rows_below(row);
                self.semantic_mark_overlay.clear_rows_below(row);
                self.clear_current_semantic_mark_range(col, self.term.columns());
                self.clear_current_line_effect_metadata();
            }
            ansi::ClearMode::Above => {
                self.extra_styles.clear_rows_above(row);
                self.clear_current_extra_style_range(
                    0,
                    col.saturating_add(1).min(self.term.columns()),
                );
                self.protected_cells.clear_screen_for_mode(
                    row,
                    col,
                    self.term.screen_lines(),
                    self.term.columns(),
                    mode,
                );
                self.media_overlay.clear_rows_above(row);
                self.side_effect_overlay.clear_rows_above(row);
                self.semantic_mark_overlay.clear_rows_above(row);
                self.clear_current_semantic_mark_range(
                    0,
                    col.saturating_add(1).min(self.term.columns()),
                );
                self.clear_current_line_effect_metadata();
            }
            ansi::ClearMode::All => {
                self.clear_screen_overlays();
            }
            ansi::ClearMode::Saved => {}
        }
    }

    fn clear_all_overlays(&mut self) {
        self.extra_styles.clear();
        self.protected_cells.clear();
        self.media_overlay.clear();
        self.side_effect_overlay.clear();
        self.semantic_mark_overlay.clear();
    }

    fn mark_side_effect(
        &mut self,
        kind: ScreenLineSideEffectKind,
        target: Option<ScreenLineSideEffectTarget>,
    ) {
        let row = self.cursor_position().map(|(row, _)| row).unwrap_or_default();
        self.side_effect_overlay.push(
            row,
            ScreenLineSideEffect {
                kind,
                disposition: ScreenLineSideEffectDisposition::Blocked,
                target,
                message: None,
            },
        );
    }
}

macro_rules! delegate_handler_method {
    ($name:ident($($arg:ident: $ty:ty),* $(,)?)) => {
        fn $name(&mut self, $($arg: $ty),*) {
            ansi::Handler::$name(self.term, $($arg),*);
        }
    };
}

impl ansi::Handler for ExtraTextStyleHandler<'_> {
    delegate_handler_method!(set_title(title: Option<String>));
    delegate_handler_method!(set_cursor_style(style: Option<ansi::CursorStyle>));
    delegate_handler_method!(set_cursor_shape(shape: AlacrittyCursorShape));
    delegate_handler_method!(goto(line: i32, col: usize));
    delegate_handler_method!(goto_line(line: i32));
    delegate_handler_method!(goto_col(col: usize));
    delegate_handler_method!(move_up(lines: usize));
    delegate_handler_method!(move_down(lines: usize));
    delegate_handler_method!(identify_terminal(intermediate: Option<char>));
    delegate_handler_method!(device_status(status: usize));
    delegate_handler_method!(move_forward(cols: usize));
    delegate_handler_method!(move_backward(cols: usize));
    delegate_handler_method!(move_down_and_cr(rows: usize));
    delegate_handler_method!(move_up_and_cr(rows: usize));
    delegate_handler_method!(put_tab(count: u16));
    delegate_handler_method!(backspace());
    delegate_handler_method!(carriage_return());
    delegate_handler_method!(linefeed());
    delegate_handler_method!(bell());
    delegate_handler_method!(newline());
    delegate_handler_method!(set_horizontal_tabstop());
    delegate_handler_method!(clear_tabs(mode: ansi::TabulationClearMode));
    delegate_handler_method!(set_tabs(interval: u16));
    delegate_handler_method!(reverse_index());
    delegate_handler_method!(set_mode(mode: ansi::Mode));
    delegate_handler_method!(unset_mode(mode: ansi::Mode));
    delegate_handler_method!(report_mode(mode: ansi::Mode));
    fn report_private_mode(&mut self, mode: ansi::PrivateMode) {
        if let Some(query) = native_compatible_private_mode_query(mode) {
            let mode_state = match query {
                69 => self.horizontal_margin_mode.into(),
                _ => terminal_private_mode_state(self.term, query),
            };
            push_response_bytes(
                self.response_bytes,
                terminal_mode_query_response_bytes(TerminalModeQuery::Private(query), mode_state),
            );
            return;
        }

        ansi::Handler::report_private_mode(self.term, mode);
    }
    delegate_handler_method!(set_scrolling_region(top: usize, bottom: Option<usize>));
    delegate_handler_method!(set_keypad_application_mode());
    delegate_handler_method!(unset_keypad_application_mode());
    delegate_handler_method!(set_active_charset(index: ansi::CharsetIndex));
    delegate_handler_method!(configure_charset(
        index: ansi::CharsetIndex,
        charset: ansi::StandardCharset,
    ));
    delegate_handler_method!(set_color(index: usize, color: ansi::Rgb));
    delegate_handler_method!(dynamic_color_sequence(
        prefix: String,
        index: usize,
        terminator: &str,
    ));
    delegate_handler_method!(reset_color(index: usize));
    fn clipboard_store(&mut self, clipboard: u8, _data: &[u8]) {
        self.mark_side_effect(
            ScreenLineSideEffectKind::ClipboardWrite,
            screen_line_clipboard_target(clipboard),
        );
    }

    fn clipboard_load(&mut self, clipboard: u8, _terminator: &str) {
        self.mark_side_effect(
            ScreenLineSideEffectKind::ClipboardRead,
            screen_line_clipboard_target(clipboard),
        );
    }
    delegate_handler_method!(push_title());
    delegate_handler_method!(pop_title());
    fn text_area_size_pixels(&mut self) {
        if let Ok(window_size) = self.window_size.lock() {
            push_response_bytes(
                self.response_bytes,
                terminal_pixel_size_response_bytes(4, *window_size),
            );
        }
    }
    delegate_handler_method!(text_area_size_chars());
    delegate_handler_method!(set_hyperlink(hyperlink: Option<ansi::Hyperlink>));
    delegate_handler_method!(report_keyboard_mode());
    delegate_handler_method!(push_keyboard_mode(mode: ansi::KeyboardModes));
    delegate_handler_method!(pop_keyboard_modes(to_pop: u16));
    delegate_handler_method!(set_keyboard_mode(
        mode: ansi::KeyboardModes,
        behavior: ansi::KeyboardModesApplyBehavior,
    ));
    delegate_handler_method!(set_modify_other_keys(mode: ansi::ModifyOtherKeys));
    delegate_handler_method!(report_modify_other_keys());
    delegate_handler_method!(set_scp(
        char_path: ansi::ScpCharPath,
        update_mode: ansi::ScpUpdateMode,
    ));

    fn input(&mut self, c: char) {
        let position = self.cursor_position();
        ansi::Handler::input(self.term, c);
        if let Some((row, col)) = position {
            self.extra_styles.write_cell(row, col, c);
            self.protected_cells.write_cell(
                self.term,
                self.extra_styles,
                row,
                col,
                c,
                c.width().unwrap_or(1),
            );
        }
    }

    fn substitute(&mut self) {
        let position = self.cursor_position();
        ansi::Handler::substitute(self.term);
        if let Some((row, col)) = position {
            self.extra_styles.write_cell(row, col, '\u{1a}');
            self.protected_cells.write_cell(self.term, self.extra_styles, row, col, '\u{1a}', 1);
        }
    }

    fn insert_blank(&mut self, count: usize) {
        self.clear_current_extra_style_tail();
        self.clear_current_semantic_mark_tail();
        self.clear_current_line_effect_metadata();
        ansi::Handler::insert_blank(self.term, count);
    }

    fn scroll_up(&mut self, lines: usize) {
        self.clear_screen_overlays();
        ansi::Handler::scroll_up(self.term, lines);
    }

    fn scroll_down(&mut self, lines: usize) {
        self.clear_screen_overlays();
        ansi::Handler::scroll_down(self.term, lines);
    }

    fn insert_blank_lines(&mut self, lines: usize) {
        self.clear_screen_overlays();
        ansi::Handler::insert_blank_lines(self.term, lines);
    }

    fn delete_lines(&mut self, lines: usize) {
        self.clear_screen_overlays();
        ansi::Handler::delete_lines(self.term, lines);
    }

    fn erase_chars(&mut self, count: usize) {
        self.clear_current_extra_style_chars(count);
        self.clear_current_semantic_mark_chars(count);
        self.clear_current_line_effect_metadata();
        ansi::Handler::erase_chars(self.term, count);
    }

    fn delete_chars(&mut self, count: usize) {
        self.clear_current_extra_style_tail();
        self.clear_current_semantic_mark_tail();
        self.clear_current_line_effect_metadata();
        ansi::Handler::delete_chars(self.term, count);
    }

    fn move_backward_tabs(&mut self, count: u16) {
        ansi::Handler::move_backward_tabs(self.term, count);
    }

    fn move_forward_tabs(&mut self, count: u16) {
        ansi::Handler::move_forward_tabs(self.term, count);
    }

    fn save_cursor_position(&mut self) {
        ansi::Handler::save_cursor_position(self.term);
    }

    fn restore_cursor_position(&mut self) {
        ansi::Handler::restore_cursor_position(self.term);
    }

    fn clear_line(&mut self, mode: ansi::LineClearMode) {
        self.clear_current_line_extra_styles(&mode);
        self.clear_current_line_semantic_marks(&mode);
        self.clear_current_line_effect_metadata();
        ansi::Handler::clear_line(self.term, mode);
    }

    fn clear_screen(&mut self, mode: ansi::ClearMode) {
        self.clear_screen_overlays_for_mode(&mode);
        ansi::Handler::clear_screen(self.term, mode);
    }

    fn reset_state(&mut self) {
        self.clear_all_overlays();
        ansi::Handler::reset_state(self.term);
    }

    fn terminal_attribute(&mut self, attr: ansi::Attr) {
        match attr {
            ansi::Attr::Reset => self.extra_styles.set_current_blink(false),
            ansi::Attr::BlinkSlow | ansi::Attr::BlinkFast => {
                self.extra_styles.set_current_blink(true);
            }
            ansi::Attr::CancelBlink => self.extra_styles.set_current_blink(false),
            _ => {}
        }
        ansi::Handler::terminal_attribute(self.term, attr);
    }

    fn set_private_mode(&mut self, mode: ansi::PrivateMode) {
        match native_compatible_set_private_mode_action(mode) {
            NativePrivateModeAction::Forward(mode) => {
                self.clear_screen_overlays();
                ansi::Handler::set_private_mode(self.term, mode);
            }
            NativePrivateModeAction::SaveCursorPosition => {
                ansi::Handler::save_cursor_position(self.term);
            }
            NativePrivateModeAction::RestoreCursorPosition => {
                ansi::Handler::restore_cursor_position(self.term);
            }
        }
    }

    fn unset_private_mode(&mut self, mode: ansi::PrivateMode) {
        match native_compatible_unset_private_mode_action(mode) {
            NativePrivateModeAction::Forward(mode) => {
                self.clear_screen_overlays();
                ansi::Handler::unset_private_mode(self.term, mode);
            }
            NativePrivateModeAction::SaveCursorPosition => {
                ansi::Handler::save_cursor_position(self.term);
            }
            NativePrivateModeAction::RestoreCursorPosition => {
                ansi::Handler::restore_cursor_position(self.term);
            }
        }
    }

    fn decaln(&mut self) {
        self.clear_screen_overlays();
        ansi::Handler::decaln(self.term);
    }
}

enum NativePrivateModeAction {
    Forward(ansi::PrivateMode),
    SaveCursorPosition,
    RestoreCursorPosition,
}

fn native_compatible_set_private_mode_action(mode: ansi::PrivateMode) -> NativePrivateModeAction {
    match mode {
        ansi::PrivateMode::Unknown(47 | 1047) => NativePrivateModeAction::Forward(
            ansi::NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        ),
        ansi::PrivateMode::Unknown(1048) => NativePrivateModeAction::SaveCursorPosition,
        _ => NativePrivateModeAction::Forward(mode),
    }
}

fn native_compatible_unset_private_mode_action(mode: ansi::PrivateMode) -> NativePrivateModeAction {
    match mode {
        ansi::PrivateMode::Unknown(47 | 1047) => NativePrivateModeAction::Forward(
            ansi::NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        ),
        ansi::PrivateMode::Unknown(1048) => NativePrivateModeAction::RestoreCursorPosition,
        _ => NativePrivateModeAction::Forward(mode),
    }
}

fn native_compatible_private_mode_query(mode: ansi::PrivateMode) -> Option<u16> {
    match mode {
        ansi::PrivateMode::Unknown(mode @ (47 | 69 | 1047)) => Some(mode),
        ansi::PrivateMode::Named(ansi::NamedPrivateMode::SwapScreenAndSetRestoreCursor) => {
            Some(1049)
        }
        _ => None,
    }
}

fn screen_line_clipboard_target(clipboard: u8) -> Option<ScreenLineSideEffectTarget> {
    match clipboard {
        b'c' => Some(ScreenLineSideEffectTarget::Clipboard),
        b'p' | b's' => Some(ScreenLineSideEffectTarget::Selection),
        _ => Some(ScreenLineSideEffectTarget::Unknown),
    }
}

fn screen_text_style_from_cell(
    cell: &Cell,
    extra_style: ExtraTextStyle,
    colors: &Colors,
) -> ScreenTextStyle {
    let flags = cell.flags;
    let mut style = ScreenTextStyle {
        foreground: screen_color_from_alacritty(cell.fg, colors),
        background: screen_color_from_alacritty(cell.bg, colors),
        underline_color: cell
            .underline_color()
            .and_then(|color| screen_color_from_alacritty(color, colors)),
        bold: flags.contains(Flags::BOLD),
        dim: flags.contains(Flags::DIM),
        italic: flags.contains(Flags::ITALIC),
        blink: extra_style.blink,
        underline: screen_underline_style_from_flags(flags),
        overline: extra_style.overline,
        border: extra_style.border,
        baseline: extra_style.baseline,
        inverse: flags.contains(Flags::INVERSE),
        hidden: flags.contains(Flags::HIDDEN),
        strikethrough: flags.contains(Flags::STRIKEOUT),
        hyperlink: cell.hyperlink().map(|link| link.uri().to_string()),
    };
    if let Some(foreground) = extra_style.foreground {
        style.foreground = foreground.map(|color| resolve_extra_screen_color(color, colors));
    }
    if let Some(background) = extra_style.background {
        style.background = background.map(|color| resolve_extra_screen_color(color, colors));
    }
    if let Some(bold) = extra_style.bold {
        style.bold = bold;
    }
    if let Some(dim) = extra_style.dim {
        style.dim = dim;
    }
    if let Some(italic) = extra_style.italic {
        style.italic = italic;
    }
    if let Some(underline) = extra_style.underline {
        style.underline = underline;
    }
    if let Some(underline_color) = extra_style.underline_color {
        style.underline_color =
            underline_color.map(|color| resolve_extra_screen_color(color, colors));
    }
    if let Some(inverse) = extra_style.inverse {
        style.inverse = inverse;
    }
    if let Some(hidden) = extra_style.hidden {
        style.hidden = hidden;
    }
    if let Some(strikethrough) = extra_style.strikethrough {
        style.strikethrough = strikethrough;
    }
    style
}

fn screen_color_from_alacritty(color: Color, colors: &Colors) -> Option<ScreenColor> {
    match color {
        Color::Named(NamedColor::Foreground) | Color::Named(NamedColor::Background) => None,
        Color::Named(named) => colors[named]
            .map(screen_color_from_rgb)
            .or_else(|| Some(ScreenColor::Named { name: named_color_name(named).to_string() })),
        Color::Indexed(index) => colors[usize::from(index)]
            .map(screen_color_from_rgb)
            .or(Some(ScreenColor::Indexed { index })),
        Color::Spec(rgb) => Some(screen_color_from_rgb(rgb)),
    }
}

fn screen_color_from_rgb(rgb: Rgb) -> ScreenColor {
    ScreenColor::Rgb { r: rgb.r, g: rgb.g, b: rgb.b }
}

fn resolve_extra_screen_color(color: ScreenColor, colors: &Colors) -> ScreenColor {
    match color {
        ScreenColor::Indexed { index } => colors[usize::from(index)]
            .map(screen_color_from_rgb)
            .unwrap_or(ScreenColor::Indexed { index }),
        ScreenColor::Named { name } => native_named_sgr_index(&name)
            .and_then(|index| colors[usize::from(index)].map(screen_color_from_rgb))
            .unwrap_or(ScreenColor::Named { name }),
        color @ ScreenColor::Rgb { .. } => color,
    }
}

fn screen_surface_palette_from_colors(colors: &Colors) -> ScreenSurfacePalette {
    ScreenSurfacePalette {
        foreground: colors[NamedColor::Foreground].map(screen_color_from_rgb),
        background: colors[NamedColor::Background].map(screen_color_from_rgb),
        cursor: colors[NamedColor::Cursor].map(screen_color_from_rgb),
    }
}

fn screen_underline_style_from_flags(flags: Flags) -> Option<ScreenUnderlineStyle> {
    if flags.contains(Flags::DOUBLE_UNDERLINE) {
        return Some(ScreenUnderlineStyle::Double);
    }
    if flags.contains(Flags::UNDERCURL) {
        return Some(ScreenUnderlineStyle::Curly);
    }
    if flags.contains(Flags::DOTTED_UNDERLINE) {
        return Some(ScreenUnderlineStyle::Dotted);
    }
    if flags.contains(Flags::DASHED_UNDERLINE) {
        return Some(ScreenUnderlineStyle::Dashed);
    }
    if flags.contains(Flags::UNDERLINE) {
        return Some(ScreenUnderlineStyle::Single);
    }
    None
}

fn screen_cursor_shape_from_alacritty(shape: AlacrittyCursorShape) -> ScreenCursorShape {
    match shape {
        AlacrittyCursorShape::Block => ScreenCursorShape::Block,
        AlacrittyCursorShape::Underline => ScreenCursorShape::Underline,
        AlacrittyCursorShape::Beam => ScreenCursorShape::Beam,
        AlacrittyCursorShape::HollowBlock => ScreenCursorShape::HollowBlock,
        AlacrittyCursorShape::Hidden => ScreenCursorShape::Hidden,
    }
}

fn named_color_name(color: NamedColor) -> &'static str {
    match color {
        NamedColor::Black => "black",
        NamedColor::Red => "red",
        NamedColor::Green => "green",
        NamedColor::Yellow => "yellow",
        NamedColor::Blue => "blue",
        NamedColor::Magenta => "magenta",
        NamedColor::Cyan => "cyan",
        NamedColor::White => "white",
        NamedColor::BrightBlack => "bright_black",
        NamedColor::BrightRed => "bright_red",
        NamedColor::BrightGreen => "bright_green",
        NamedColor::BrightYellow => "bright_yellow",
        NamedColor::BrightBlue => "bright_blue",
        NamedColor::BrightMagenta => "bright_magenta",
        NamedColor::BrightCyan => "bright_cyan",
        NamedColor::BrightWhite => "bright_white",
        NamedColor::Foreground => "foreground",
        NamedColor::Background => "background",
        NamedColor::Cursor => "cursor",
        NamedColor::DimBlack => "dim_black",
        NamedColor::DimRed => "dim_red",
        NamedColor::DimGreen => "dim_green",
        NamedColor::DimYellow => "dim_yellow",
        NamedColor::DimBlue => "dim_blue",
        NamedColor::DimMagenta => "dim_magenta",
        NamedColor::DimCyan => "dim_cyan",
        NamedColor::DimWhite => "dim_white",
        NamedColor::BrightForeground => "bright_foreground",
        NamedColor::DimForeground => "dim_foreground",
    }
}

fn trim_rich_line_end(text: &mut String, spans: &mut Vec<ScreenLineSpan>) {
    let trimmed_len = rich_line_trimmed_len(text, spans);
    if trimmed_len == text.len() {
        return;
    }

    text.truncate(trimmed_len);
    let mut remaining = trimmed_len;
    let mut next_spans = Vec::new();
    for span in spans.iter() {
        if remaining == 0 {
            break;
        }
        if span.text.len() <= remaining {
            next_spans.push(span.clone());
            remaining -= span.text.len();
            continue;
        }

        let truncated = take_utf8_prefix(&span.text, remaining);
        if !truncated.is_empty() {
            next_spans.push(ScreenLineSpan { text: truncated, style: span.style.clone() });
        }
        break;
    }
    *spans = next_spans;
}

fn rich_line_trimmed_len(text: &str, spans: &[ScreenLineSpan]) -> usize {
    if spans.is_empty() {
        return text.trim_end().len();
    }

    let mut end = text.len();
    for span in spans.iter().rev() {
        let span_start = end.saturating_sub(span.text.len());
        if !span.style.is_plain() {
            break;
        }

        let trimmed_span_len = span.text.trim_end().len();
        if trimmed_span_len == span.text.len() {
            break;
        }

        end = span_start + trimmed_span_len;
        if trimmed_span_len > 0 {
            break;
        }
    }
    end
}

fn take_utf8_prefix(value: &str, max_bytes: usize) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if output.len() + ch.len_utf8() > max_bytes {
            break;
        }
        output.push(ch);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_plain_padding_after_rich_text() {
        let mut text = "red   ".to_string();
        let mut spans = vec![
            ScreenLineSpan {
                text: "red".to_string(),
                style: ScreenTextStyle {
                    foreground: Some(ScreenColor::Named { name: "red".to_string() }),
                    ..ScreenTextStyle::default()
                },
            },
            ScreenLineSpan { text: "   ".to_string(), style: ScreenTextStyle::default() },
        ];

        trim_rich_line_end(&mut text, &mut spans);

        assert_eq!(text, "red");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "red");
    }

    #[test]
    fn preserves_styled_trailing_spaces() {
        let mut text = "   ".to_string();
        let mut spans = vec![ScreenLineSpan {
            text: "   ".to_string(),
            style: ScreenTextStyle {
                background: Some(ScreenColor::Named { name: "red".to_string() }),
                ..ScreenTextStyle::default()
            },
        }];

        trim_rich_line_end(&mut text, &mut spans);

        assert_eq!(text, "   ");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "   ");
    }

    #[test]
    fn skips_fullwidth_spacer_cells_without_losing_rich_spans() {
        let colors = Colors::default();
        let mut wide = Cell::default();
        wide.c = '表';
        wide.fg = Color::Named(NamedColor::Red);
        wide.flags.insert(Flags::WIDE_CHAR);

        let mut spacer = Cell::default();
        spacer.flags.insert(Flags::WIDE_CHAR_SPACER);

        let mut next = Cell::default();
        next.c = 'A';

        let mut builder = RichScreenLineBuilder::default();
        builder.push_cell_at_col(0, &wide, ExtraTextStyle::default(), &colors);
        builder.push_cell_at_col(1, &spacer, ExtraTextStyle::default(), &colors);
        builder.push_cell_at_col(2, &next, ExtraTextStyle::default(), &colors);

        let line = builder.finish();

        assert_eq!(line.text, "表A");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "表"
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            }),
            "wide rich cell should keep its style while spacer stays invisible: {:?}",
            line.spans
        );
        assert!(
            !line.spans.iter().any(|span| span.text.contains(" A")),
            "fullwidth spacer must not become visible text: {:?}",
            line.spans
        );
    }

    #[test]
    fn keeps_zero_width_marks_attached_to_their_styled_cell() {
        let colors = Colors::default();
        let mut base = Cell::default();
        base.c = 'e';
        base.fg = Color::Named(NamedColor::Green);

        let mut next = Cell::default();
        next.c = 'Z';

        let mut builder = RichScreenLineBuilder::default();
        builder.push_cell_at_col(0, &base, ExtraTextStyle::default(), &colors);
        builder.push_zerowidth('\u{0301}');
        builder.push_cell_at_col(1, &next, ExtraTextStyle::default(), &colors);

        let line = builder.finish();
        let accented = format!("e{}Z", '\u{0301}');
        let styled_accented = format!("e{}", '\u{0301}');

        assert_eq!(line.text, accented);
        assert!(
            line.spans.iter().any(|span| {
                span.text == styled_accented
                    && span.style.foreground
                        == Some(ScreenColor::Named { name: "green".to_string() })
            }),
            "zero-width mark should stay in the styled base-cell span: {:?}",
            line.spans
        );
    }

    #[test]
    fn parses_extra_sgr_overline_without_matching_nested_color_components() {
        assert_eq!(
            parse_extra_sgr_update(b"0;53"),
            ExtraSgrUpdate {
                reset: true,
                overline: Some(true),
                reset_border: true,
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b";"),
            ExtraSgrUpdate {
                reset: true,
                overline: Some(false),
                reset_border: true,
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"4:3;0"),
            ExtraSgrUpdate {
                reset: true,
                overline: Some(false),
                reset_border: true,
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"4:3;;53"),
            ExtraSgrUpdate {
                reset: true,
                overline: Some(true),
                reset_border: true,
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"51"),
            ExtraSgrUpdate {
                border: Some(ScreenTextBorderStyle::Framed),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"54;52"),
            ExtraSgrUpdate {
                reset_border: true,
                border: Some(ScreenTextBorderStyle::Encircled),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"51;54"),
            ExtraSgrUpdate { reset_border: true, ..ExtraSgrUpdate::default() }
        );
        assert_eq!(
            parse_extra_sgr_update(b"55"),
            ExtraSgrUpdate { overline: Some(false), ..ExtraSgrUpdate::default() }
        );
        assert_eq!(
            parse_extra_sgr_update(b"73"),
            ExtraSgrUpdate {
                baseline: Some(Some(ScreenTextBaseline::Superscript)),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"74"),
            ExtraSgrUpdate {
                baseline: Some(Some(ScreenTextBaseline::Subscript)),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"75"),
            ExtraSgrUpdate { baseline: Some(None), ..ExtraSgrUpdate::default() }
        );
        assert_eq!(
            parse_extra_sgr_update(b"38:2:53:55:0"),
            ExtraSgrUpdate {
                foreground: Some(Some(ScreenColor::Rgb { r: 53, g: 55, b: 0 })),
                render_foreground: Some(ScreenColor::Rgb { r: 53, g: 55, b: 0 }),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"38;2;1;2;3;1"),
            ExtraSgrUpdate {
                foreground: Some(Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })),
                bold: Some(true),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"58;2;1;2;3;24"),
            ExtraSgrUpdate {
                underline_color: Some(Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })),
                underline: Some(None),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"38;2;;12;34;56;1"),
            ExtraSgrUpdate {
                foreground: Some(Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })),
                bold: Some(true),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"58;6;;9;8;7;128;24"),
            ExtraSgrUpdate {
                underline_color: Some(Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })),
                underline: Some(None),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"4:3"),
            ExtraSgrUpdate {
                underline: Some(Some(ScreenUnderlineStyle::Curly)),
                ..ExtraSgrUpdate::default()
            }
        );
        assert_eq!(
            parse_extra_sgr_update(b"4:0"),
            ExtraSgrUpdate { underline: Some(None), ..ExtraSgrUpdate::default() }
        );
    }

    #[test]
    fn native_rendering_strips_iso_charset_designations_without_leaking_final_bytes() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(b"a\x1b#6b\x1b%Gd\x1b%@e\x1b$)Cf\x1b(Bg\x1b*0h\x1b Fz");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("abdefghz"))
            .expect("ISO/DEC designation controls should not leak final bytes");

        assert_eq!(line.text.trim_end(), "abdefghz");
    }

    #[test]
    fn native_rendering_preserves_index_column_and_next_line_column_reset() {
        let buffer = EmulatorBuffer::new(4, 24);
        buffer.advance(b"ab\x1bDxy\r\nab\x1bExy");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text.trim_end(), "ab");
        assert_eq!(lines[1].text.trim_end(), "  xy");
        assert_eq!(lines[2].text.trim_end(), "ab");
        assert_eq!(lines[3].text.trim_end(), "xy");
    }

    #[test]
    fn resets_colon_underline_sgr_without_leaking_to_following_text() {
        let buffer = EmulatorBuffer::new(4, 96);
        buffer.advance(b"\x1b[4:3mcurly\x1b[4:0mplain");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("curlyplain"))
            .expect("colon underline reset line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "curly" && span.style.underline == Some(ScreenUnderlineStyle::Curly)
            }),
            "SGR 4:3 should preserve curly underline: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "plain" && span.style.underline.is_none()),
            "SGR 4:0 should reset underline before following text: {:?}",
            line.spans
        );
    }

    #[test]
    fn applies_carriage_return_progress_rewrite_without_duplicate_output() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(b"progress 10%\rprogress 100%\x1b[K");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("progress"))
            .expect("progress line should render");

        assert_eq!(line.text, "progress 100%");
        assert!(
            !line.text.contains("10%"),
            "carriage-return rewrite must not keep stale progress text: {line:?}"
        );
    }

    #[test]
    fn applies_backspace_rewrite_without_duplicate_output() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(b"abc\x08\x08XY");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("aXY"))
            .expect("backspace rewrite line should render");

        assert_eq!(line.text, "aXY");
        assert!(
            !line.text.contains("abc"),
            "backspace rewrite must not keep overwritten cells: {line:?}"
        );
    }

    #[test]
    fn preserves_extended_sgr_styles_from_vt_output() {
        let buffer = EmulatorBuffer::new(6, 160);
        buffer.advance(
            b"\x1b[2mdim\x1b[0m \
              \x1b[3mitalic\x1b[0m \
              \x1b[4:2mdouble\x1b[0m \
              \x1b[4:3mcurly\x1b[0m \
              \x1b[4:4mdotted\x1b[0m \
              \x1b[4:5mdashed\x1b[0m \
              \x1b[4:99mfallback\x1b[0m \
              \x1b[58;2;1;2;3m\x1b[4mcolored\x1b[0m \
              \x1b[5mblink\x1b[25msteady\x1b[0m \
              \x1b[6mfast\x1b[0m \
              \x1b[53mover\x1b[55mflat\x1b[0m \
              \x1b[51mframe\x1b[54mplain\x1b[0m \
              \x1b[52mcircle\x1b[0m \
              \x1b[73msuper\x1b[75mregular\x1b[0m \
              \x1b[74msub\x1b[0m \
              \x1b[7minverse\x1b[0m \
              \x1b[8mhidden\x1b[0m \
              \x1b[9mstrike\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("dim"))
            .expect("extended SGR line should render");

        assert!(
            line.spans.iter().any(|span| span.text == "dim" && span.style.dim),
            "SGR dim should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "italic" && span.style.italic),
            "SGR italic should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "double" && span.style.underline == Some(ScreenUnderlineStyle::Double)
            }),
            "SGR double underline should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "curly" && span.style.underline == Some(ScreenUnderlineStyle::Curly)
            }),
            "SGR curly underline should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "dotted" && span.style.underline == Some(ScreenUnderlineStyle::Dotted)
            }),
            "SGR dotted underline should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "dashed" && span.style.underline == Some(ScreenUnderlineStyle::Dashed)
            }),
            "SGR dashed underline should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "fallback"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
            }),
            "unknown SGR 4:x underline style should degrade to single underline: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "colored"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
            }),
            "SGR underline color should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "blink" && span.style.blink),
            "SGR slow blink should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "fast" && span.style.blink),
            "SGR rapid blink should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text.starts_with("steady") && !span.style.blink),
            "SGR cancel blink should reset following text: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "over" && span.style.overline),
            "SGR overline should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text.starts_with("flat") && !span.style.overline),
            "SGR cancel overline should reset following text: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "frame"
                && span.style.border == Some(ScreenTextBorderStyle::Framed)),
            "SGR framed text should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.starts_with("plain") && span.style.border.is_none()),
            "SGR cancel framed/encircled should reset following text: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "circle"
                && span.style.border == Some(ScreenTextBorderStyle::Encircled)),
            "SGR encircled text should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "super"
                && span.style.baseline == Some(ScreenTextBaseline::Superscript)),
            "SGR superscript should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.starts_with("regular") && span.style.baseline.is_none()),
            "SGR baseline reset should reset following text: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "sub"
                && span.style.baseline == Some(ScreenTextBaseline::Subscript)),
            "SGR subscript should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "inverse" && span.style.inverse),
            "SGR inverse should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "hidden" && span.style.hidden),
            "SGR hidden should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "strike" && span.style.strikethrough),
            "SGR strikeout should be preserved: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_tmux_terminfo_double_colon_sgr_forms_from_vt_output() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b[4::3;58::2::9::8::7munder\x1b[0m \
              \x1b[38::2::1::2::3mfg\x1b[0m \
              \x1b[48::5::196mbg\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("under"))
            .expect("double-colon SGR line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "under"
                    && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            }),
            "tmux Smulx/Setulc undercurl should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
            }),
            "double-colon truecolor foreground should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg"
                    && span.style.background == Some(ScreenColor::Indexed { index: 196 })
            }),
            "double-colon indexed background should be preserved: {:?}",
            line.spans
        );
    }

    #[test]
    fn native_rendering_preserves_mixed_terminal_visual_sequence_matrix() {
        let buffer = EmulatorBuffer::new(6, 160);
        buffer.advance(
            b"\x1b]4;1;rgb:12/34/56;22;#0A0B0C\x07\
              \x1b]10;rgb:ee/ee/ee;11;rgb:00/00/00;12;rgb:01/02/03\x07\
              \x1b[31mred\x1b[0m \
              \x1b[38;5;22midx\x1b[0m \
              \x1b[38:2::1:2:3;48:2::4:5:6mtrue\x1b[0m \
              \x1b[7minverse\x1b[27mplain \
              \x1b]8;;https://example.test/log\x07\x1b[4:3;58:2::9:8:7mlink\x1b]8;;\x07\x1b[0m",
        );

        let surface = buffer.render(None).surface;
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
    fn native_rendering_preserves_xterm_sgr_stack_for_overlay_styles() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(b"\x1b[53;4:3mouter \x1b[#{\x1b[55;4:5minner\x1b[#} outer2\x1b[0m plain");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("outer"))
            .expect("xterm SGR stack line should render");

        let outer = line
            .spans
            .iter()
            .find(|span| span.text == "outer ")
            .expect("outer style span should exist");
        assert!(outer.style.overline);
        assert_eq!(outer.style.underline, Some(ScreenUnderlineStyle::Curly));

        let inner = line
            .spans
            .iter()
            .find(|span| span.text == "inner")
            .expect("inner override style span should exist");
        assert!(!inner.style.overline);
        assert_eq!(inner.style.underline, Some(ScreenUnderlineStyle::Dashed));

        let restored = line
            .spans
            .iter()
            .find(|span| span.text == " outer2")
            .expect("restored style span should exist");
        assert!(restored.style.overline);
        assert_eq!(restored.style.underline, Some(ScreenUnderlineStyle::Curly));
    }

    #[test]
    fn native_rendering_preserves_xterm_sgr_stack_for_standard_colors() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(b"\x1b[31;1mred \x1b[#{\x1b[32;22mgreen\x1b[#} red2\x1b[0m plain");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("xterm SGR standard color stack line should render");

        let restored = line
            .spans
            .iter()
            .find(|span| span.text == " red2")
            .expect("restored red span should exist");
        assert_eq!(restored.style.foreground, Some(ScreenColor::Named { name: "red".to_string() }));
        assert!(restored.style.bold);

        let plain = line
            .spans
            .iter()
            .find(|span| span.text == " plain")
            .expect("plain reset span should exist");
        assert!(plain.style.is_plain());
    }

    #[test]
    fn native_rendering_preserves_xterm_selective_sgr_stack_attributes() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(b"\x1b[31mred \x1b[30#{\x1b[32;1mgreen\x1b[#} red2");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("xterm selective SGR stack line should render");

        let restored = line
            .spans
            .iter()
            .find(|span| span.text == " red2")
            .expect("selectively restored red span should exist");
        assert_eq!(restored.style.foreground, Some(ScreenColor::Named { name: "red".to_string() }));
        assert!(
            restored.style.bold,
            "selective foreground restore must leave non-selected bold state untouched"
        );
    }

    #[test]
    fn native_rendering_preserves_xterm_sgr_stack_aliases_for_overlay_styles() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(b"\x1b[53mover\x1b[#p\x1b[55mflat\x1b[#qover2");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("over"))
            .expect("xterm SGR stack alias line should render");

        assert!(
            line.spans.iter().any(|span| span.text == "over" && span.style.overline),
            "initial overline span should render: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "flat" && !span.style.overline),
            "inner flat span should render: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "over2" && span.style.overline),
            "restored overline span should render: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_c1_csi_extended_sgr_styles_from_vt_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x9b53mover\x9b55mflat");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("over"))
            .expect("C1 CSI extended SGR line should render");

        assert!(
            line.spans.iter().any(|span| span.text == "over" && span.style.overline),
            "C1 CSI SGR 53 should preserve overline: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "flat" && !span.style.overline),
            "C1 CSI SGR 55 should reset overline: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_extended_sgr_styles_outside_partial_line_clear_ranges() {
        let buffer = EmulatorBuffer::new(2, 80);
        buffer.advance(b"\x1b[53mkeep\x1b[0mxxxx\r\x1b[4C\x1b[K");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("keep"))
            .expect("partially cleared line should still render retained text");

        assert_eq!(line.text, "keep");
        assert!(
            line.spans.iter().any(|span| span.text == "keep" && span.style.overline),
            "EL to the right of cursor must not erase extended style metadata to the left: {:?}",
            line.spans
        );

        let erase_buffer = EmulatorBuffer::new(2, 80);
        erase_buffer.advance(b"\x1b[53mkeep\x1b[0mxxxx\r\x1b[4C\x1b[4X");
        let erase_line = erase_buffer
            .render(None)
            .surface
            .lines
            .into_iter()
            .find(|line| line.text.contains("keep"))
            .expect("ECH line should still render retained text");
        assert_eq!(erase_line.text, "keep");
        assert!(
            erase_line.spans.iter().any(|span| span.text == "keep" && span.style.overline),
            "ECH must not erase extended style metadata to the left: {:?}",
            erase_line.spans
        );

        let delete_buffer = EmulatorBuffer::new(2, 80);
        delete_buffer.advance(b"\x1b[53mkeep\x1b[0mxxxx\r\x1b[4C\x1b[4P");
        let delete_line = delete_buffer
            .render(None)
            .surface
            .lines
            .into_iter()
            .find(|line| line.text.contains("keep"))
            .expect("DCH line should still render retained text");
        assert_eq!(delete_line.text, "keep");
        assert!(
            delete_line.spans.iter().any(|span| span.text == "keep" && span.style.overline),
            "DCH must not erase extended style metadata to the left: {:?}",
            delete_line.spans
        );
    }

    #[test]
    fn preserves_extended_sgr_styles_outside_partial_screen_clear_ranges() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[53mtop\x1b[0m\r\n\x1b[53mkeep\x1b[0mxxxx\r\x1b[4C\x1b[J");

        let surface = buffer.render(None).surface;
        let top_line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("top"))
            .expect("line above clear-below cursor should still render");
        assert!(
            top_line.spans.iter().any(|span| span.text == "top" && span.style.overline),
            "ED below must not erase extended style metadata above cursor: {:?}",
            top_line.spans
        );

        let keep_line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("keep"))
            .expect("current line text to left of clear-below cursor should still render");
        assert_eq!(keep_line.text, "keep");
        assert!(
            keep_line.spans.iter().any(|span| span.text == "keep" && span.style.overline),
            "ED below must not erase extended style metadata to the left of cursor: {:?}",
            keep_line.spans
        );

        let saved_buffer = EmulatorBuffer::new(2, 80);
        saved_buffer.advance(b"\x1b[53mkeep\x1b[0m\x1b[3J");
        let saved_line = saved_buffer
            .render(None)
            .surface
            .lines
            .into_iter()
            .find(|line| line.text.contains("keep"))
            .expect("visible line should survive saved scrollback clear");
        assert!(
            saved_line.spans.iter().any(|span| span.text == "keep" && span.style.overline),
            "ED saved-scrollback clear must not erase current screen style metadata: {:?}",
            saved_line.spans
        );
    }

    #[test]
    fn resets_common_sgr_attributes_without_leaking_to_following_text() {
        let buffer = EmulatorBuffer::new(6, 180);
        buffer.advance(
            b"\x1b[1;2mintense\x1b[22mnormal \
              \x1b[3mitalic\x1b[23mroman \
              \x1b[4munder\x1b[24mplain \
              \x1b[21mdouble\x1b[24mflat \
              \x1b[7minverse\x1b[27mforward \
              \x1b[8mhidden\x1b[28mshown \
              \x1b[9mstrike\x1b[29mclean \
              \x1b[53mover\x1b[mreset",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("intense"))
            .expect("targeted SGR reset line should render");

        assert!(
            line.spans
                .iter()
                .any(|span| span.text == "intense" && span.style.bold && span.style.dim),
            "SGR 1/2 should preserve bold and dim before reset: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.starts_with("normal") && !span.style.bold && !span.style.dim),
            "SGR 22 should reset bold and dim only: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "italic" && span.style.italic),
            "SGR 3 should preserve italic: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text.starts_with("roman") && !span.style.italic),
            "SGR 23 should reset italic: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "under" && span.style.underline.is_some()),
            "SGR 4 should preserve underline: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.starts_with("plain") && span.style.underline.is_none()),
            "SGR 24 should reset underline: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "double"
                && span.style.underline == Some(ScreenUnderlineStyle::Double)),
            "SGR 21 should preserve double underline: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.starts_with("flat") && span.style.underline.is_none()),
            "SGR 24 should also reset SGR 21 double underline: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "inverse" && span.style.inverse),
            "SGR 7 should preserve inverse: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text.starts_with("forward") && !span.style.inverse),
            "SGR 27 should reset inverse: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "hidden" && span.style.hidden),
            "SGR 8 should preserve hidden text: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text.starts_with("shown") && !span.style.hidden),
            "SGR 28 should reset hidden text: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "strike" && span.style.strikethrough),
            "SGR 9 should preserve strikethrough: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.starts_with("clean") && !span.style.strikethrough),
            "SGR 29 should reset strikethrough: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "over" && span.style.overline),
            "SGR 53 should preserve overline before bare reset: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == "reset"
                && !span.style.overline
                && span.style == ScreenTextStyle::default()),
            "bare CSI m should reset extra and standard SGR state: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_bright_ansi_sgr_colors_and_targeted_resets() {
        let buffer = EmulatorBuffer::new(4, 160);
        buffer.advance(
            b"\x1b[91mbright-fg\x1b[0m \
              \x1b[104mbright-bg\x1b[0m \
              \x1b[91;104mboth\x1b[39mfg-reset\x1b[49mbg-reset",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("bright-fg"))
            .expect("bright ANSI SGR line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "bright-fg"
                    && span.style.foreground
                        == Some(ScreenColor::Named { name: "bright_red".to_string() })
                    && span.style.background.is_none()
            }),
            "SGR 91 should preserve bright foreground color: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bright-bg"
                    && span.style.foreground.is_none()
                    && span.style.background
                        == Some(ScreenColor::Named { name: "bright_blue".to_string() })
            }),
            "SGR 104 should preserve bright background color: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "both"
                    && span.style.foreground
                        == Some(ScreenColor::Named { name: "bright_red".to_string() })
                    && span.style.background
                        == Some(ScreenColor::Named { name: "bright_blue".to_string() })
            }),
            "combined bright foreground/background should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg-reset"
                    && span.style.foreground.is_none()
                    && span.style.background
                        == Some(ScreenColor::Named { name: "bright_blue".to_string() })
            }),
            "SGR 39 should reset only foreground color: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg-reset"
                    && span.style.foreground.is_none()
                    && span.style.background.is_none()
            }),
            "SGR 49 should reset only background color: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_colon_truecolor_sgr_output() {
        let buffer = EmulatorBuffer::new(3, 80);
        buffer.advance(
            b"\x1b[38:2:12:34:56mfg\x1b[0m \
              \x1b[48:2:70:80:90mbg\x1b[0m \
              \x1b[38;5;196midx\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("fg"))
            .expect("truecolor line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
            }),
            "colon truecolor foreground should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg"
                    && span.style.background == Some(ScreenColor::Rgb { r: 70, g: 80, b: 90 })
            }),
            "colon truecolor background should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Indexed { index: 196 })
            }),
            "indexed foreground should be preserved: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_semicolon_sgr_color_followed_by_later_attributes() {
        let buffer = EmulatorBuffer::new(3, 96);
        buffer.advance(
            b"\x1b[38;3;0;128;255;1mcmy-bold\x1b[0m \
              \x1b[58;2;1;2;3;24munder-reset\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("cmy-bold"))
            .expect("combined SGR line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "cmy-bold"
                    && span.style.bold
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 127, b: 0 })
            }),
            "semicolon CMY foreground should not skip following bold: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "under-reset"
                    && span.style.underline.is_none()
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
            }),
            "semicolon underline color should not skip following underline reset: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_semicolon_sgr_colors_with_empty_color_space_slot() {
        let buffer = EmulatorBuffer::new(3, 120);
        buffer.advance(
            b"\x1b[38;2;;12;34;56mfg\x1b[0m \
              \x1b[48;2;;70;80;90mbg\x1b[0m \
              \x1b[4m\x1b[58;6;;9;8;7;128munder\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("fg"))
            .expect("semicolon empty color-space SGR line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
            }),
            "semicolon truecolor foreground with empty color-space slot should render: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg"
                    && span.style.background == Some(ScreenColor::Rgb { r: 70, g: 80, b: 90 })
            }),
            "semicolon truecolor background with empty color-space slot should render: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "under"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            }),
            "semicolon RGBA underline color with empty color-space slot should render: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_colon_truecolor_sgr_with_color_space_id_output() {
        let buffer = EmulatorBuffer::new(4, 160);
        buffer.advance(
            b"\x1b[38:2:1:12:34:56mfg-cs\x1b[0m \
              \x1b[48:2::70:80:90mbg-empty-cs\x1b[0m \
              \x1b[4m\x1b[58:2:1:9:8:7munder-cs\x1b[0m \
              \x1b[4m\x1b[58:2::3:2:1munder-empty-cs\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("fg-cs"))
            .expect("color-space truecolor line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg-cs"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
            }),
            "colon truecolor foreground with color-space id should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg-empty-cs"
                    && span.style.background == Some(ScreenColor::Rgb { r: 70, g: 80, b: 90 })
            }),
            "colon truecolor background with empty color-space id should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "under-cs"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            }),
            "colon underline truecolor with color-space id should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "under-empty-cs"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 3, g: 2, b: 1 })
            }),
            "colon underline truecolor with empty color-space id should be preserved: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_rgba_sgr_by_degrading_alpha_to_rgb_output() {
        let buffer = EmulatorBuffer::new(4, 160);
        buffer.advance(
            b"\x1b[38:6::12:34:56:128mfg-rgba\x1b[0m \
              \x1b[48:6:1:70:80:90:64mbg-rgba-cs\x1b[0m \
              \x1b[4m\x1b[58:6::9:8:7:255munder-rgba\x1b[0m \
              \x1b[38;6;3;2;1;128mfg-rgba-semi\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("fg-rgba"))
            .expect("RGBA SGR line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg-rgba"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
            }),
            "colon RGBA foreground should degrade to RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg-rgba-cs"
                    && span.style.background == Some(ScreenColor::Rgb { r: 70, g: 80, b: 90 })
            }),
            "colon RGBA background with color-space id should degrade to RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "under-rgba"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            }),
            "colon RGBA underline color should degrade to RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg-rgba-semi"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 3, g: 2, b: 1 })
            }),
            "semicolon RGBA foreground should degrade to RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_cmy_and_cmyk_sgr_colors_as_rgb_output() {
        let buffer = EmulatorBuffer::new(4, 160);
        buffer.advance(
            b"\x1b[38:3::0:128:255mfg-cmy\x1b[0m \
              \x1b[48:4::0:128:255:64mbg-cmyk\x1b[0m \
              \x1b[4m\x1b[58:3::255:0:128munder-cmy\x1b[0m \
              \x1b[38;4;0;0;0;128mfg-cmyk-semi\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("fg-cmy"))
            .expect("CMY/CMYK SGR line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg-cmy"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 127, b: 0 })
            }),
            "colon CMY foreground should convert to RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg-cmyk"
                    && span.style.background == Some(ScreenColor::Rgb { r: 191, g: 95, b: 0 })
            }),
            "colon CMYK background should convert to RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "under-cmy"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 0, g: 255, b: 127 })
            }),
            "colon CMY underline color should convert to RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg-cmyk-semi"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 127, g: 127, b: 127 })
            }),
            "semicolon CMYK foreground should convert to RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_colon_underline_color_sgr_and_reset_output() {
        let buffer = EmulatorBuffer::new(4, 160);
        buffer.advance(
            b"\x1b[4:3m\x1b[58:2:9:8:7mwavy-rgb\x1b[59m plain \
              \x1b[4m\x1b[58:5::196mindexed-under\x1b[0m \
              \x1b[4:3mcurly-before-reset\x1b[;mempty-reset",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("wavy-rgb"))
            .expect("colon underline color SGR line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "wavy-rgb"
                    && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            }),
            "colon truecolor underline color should be preserved: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text.contains("plain")
                    && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                    && span.style.underline_color.is_none()
            }),
            "SGR 59 should reset underline color without resetting underline style: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "indexed-under"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Indexed { index: 196 })
            }),
            "colon indexed underline color should preserve indexed palette contract: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "curly-before-reset"
                    && span.style.underline == Some(ScreenUnderlineStyle::Curly)
            }),
            "colon underline style should be applied before empty-parameter reset: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "empty-reset"
                    && span.style.underline.is_none()
                    && span.style.underline_color.is_none()
            }),
            "empty SGR parameter should reset extra underline state: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_for_explicit_ansi_and_indexed_colors() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;rgb:12/34/56;22;rgb:aa/bb/cc\x07\
              \x1b[31mred\x1b[0m \
              \x1b[38;5;22midx\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x12, g: 0x34, b: 0x56 })
            }),
            "OSC 4 ANSI color override should be rendered as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
            }),
            "OSC 4 indexed color override should be rendered as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_for_colon_indexed_foreground_and_background() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;22;rgb:aa/bb/cc;23;rgb:11/22/33\x07\
              \x1b[38:5::22mfg\x1b[0m \
              \x1b[48:5::23mbg\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("fg"))
            .expect("dynamic colon indexed palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
            }),
            "OSC 4 colon indexed foreground color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg"
                    && span.style.background == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
            }),
            "OSC 4 colon indexed background color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_for_extra_underline_colors() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;196;rgb:aa/bb/cc\x07\
              \x1b[4m\x1b[58:5::196munder\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("under"))
            .expect("dynamic underline palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "under"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color
                        == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
            }),
            "OSC 4 indexed underline color override should be rendered as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_iterm2_set_colors_palette_updates() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]1337;SetColors=fg=srgb:112233\x07\
              \x1b]1337;SetColors=bg=445566\x07\
              \x1b]1337;SetColors=curbg=p3:778899\x07\
              \x1b]1337;SetColors=red=00ff00\x07\
              \x1b[31mred\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("iTerm2 SetColors line should render");

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 17, g: 34, b: 51 }),
                background: Some(ScreenColor::Rgb { r: 68, g: 85, b: 102 }),
                cursor: Some(ScreenColor::Rgb { r: 119, g: 136, b: 153 }),
            }
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0, g: 255, b: 0 })
            }),
            "iTerm2 SetColors ANSI palette update should render through SGR: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_kitty_color_control_palette_updates() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]21;foreground=#112233;background=rgb:44/55/66;cursor=rgba(119, 136, 153, 0.5);1=#00ff00\x1b\\\
              \x1b[31mred\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("kitty color control line should render");

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 17, g: 34, b: 51 }),
                background: Some(ScreenColor::Rgb { r: 68, g: 85, b: 102 }),
                cursor: Some(ScreenColor::Rgb { r: 119, g: 136, b: 153 }),
            }
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0, g: 255, b: 0 })
            }),
            "kitty color control ANSI palette update should render through SGR: {:?}",
            line.spans
        );
    }

    #[test]
    fn kitty_color_stack_restores_native_palette_for_queries_and_future_output() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]21;foreground=#010203;1=#112233\x1b\\\
              \x1b]30001\x1b\\\
              \x1b]21;foreground=#040506;1=#445566\x1b\\\
              \x1b[31mtemporary\x1b[0m \
              \x1b]30101\x1b\\\
              \x1b]21;1=?\x1b\\\
              \x1b[31mrestored\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]21;1=rgb:1111/2222/3333\x1b\\"
        );
        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: None,
                cursor: None,
            }
        );
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("temporary"))
            .expect("kitty color stack line should render");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "restored"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
            }),
            "restored color should render after stack pop: {:?}",
            line.spans
        );
    }

    #[test]
    fn xterm_color_stack_restores_native_palette_for_queries_and_future_output() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;#010203;#111111;#222222\x07\
              \x1b]4;1;#112233\x07\
              \x1b[#P\
              \x1b]10;#040506;#333333;#444444\x07\
              \x1b]4;1;#445566\x07\
              \x1b[31mtemporary\x1b[0m \
              \x1b[#Q\
              \x1b]4;1;?\x07\
              \x1b[31mrestored\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;1;rgb:1111/2222/3333\x07"
        );
        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 17, g: 17, b: 17 }),
                cursor: Some(ScreenColor::Rgb { r: 34, g: 34, b: 34 }),
            }
        );
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("temporary"))
            .expect("xterm color stack line should render");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "restored"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
            }),
            "restored color should render after xterm stack pop: {:?}",
            line.spans
        );
    }

    #[test]
    fn xterm_color_stack_addressed_slots_restore_without_popping_native_palette() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;#112233\x07\
              \x1b[1#P\
              \x1b]4;1;#445566\x07\
              \x1b[2#P\
              \x1b[1#Q\x1b]4;1;?\x07\x1b[31mone\x1b[0m \
              \x1b[2#Q\x1b]4;1;?\x07\x1b[31mtwo\x1b[0m \
              \x1b[1#Q\x1b]4;1;?\x07\x1b[31mone-again\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;1;rgb:1111/2222/3333\x07\
              \x1b]4;1;rgb:4444/5555/6666\x07\
              \x1b]4;1;rgb:1111/2222/3333\x07"
        );
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("one"))
            .expect("xterm addressed color stack line should render");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "one"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
            }),
            "slot 1 color should render: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "one-again"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 })
            }),
            "slot 1 color should remain available after slot 2 restore: {:?}",
            line.spans
        );
    }

    #[test]
    fn xterm_color_stack_reports_current_entry_and_last_saved_slot() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b[#R\
              \x1b[#P\x1b[#R\
              \x1b[#P\x1b[#R\
              \x1b[1#P\x1b[#R\
              \x1b[2#Q\x1b[#R",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?0;0#Q\
              \x1b[?1;1#Q\
              \x1b[?2;2#Q\
              \x1b[?1;2#Q\
              \x1b[?1;2#Q"
        );
    }

    #[test]
    fn deccara_changes_attributes_in_native_surface() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"abcdef\x1b[1;2;1;4;1;4$r");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("abcdef"))
            .expect("DECCARA line should render");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bcd"
                    && span.style.bold
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
            }),
            "DECCARA should style the selected rectangle: {:?}",
            line.spans
        );
    }

    #[test]
    fn deccara_resets_and_decrara_reverses_native_surface_attributes() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x1b[31;1;4mabcdef\x1b[0m\x1b[1;2;1;4;22;24$r");
        buffer.advance(b"\x1b[2;1Huvwxyz\x1b[2;2;2;4;1;7$t");

        let surface = buffer.render(None).surface;
        let first = surface
            .lines
            .iter()
            .find(|line| line.text.contains("abcdef"))
            .expect("DECCARA reset line should render");
        assert!(
            first.spans.iter().any(|span| {
                span.text == "bcd"
                    && !span.style.bold
                    && span.style.underline.is_none()
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            }),
            "DECCARA reset should keep color but clear selected attributes: {:?}",
            first.spans
        );

        let second = surface
            .lines
            .iter()
            .find(|line| line.text.contains("uvwxyz"))
            .expect("DECRARA line should render");
        assert!(
            second
                .spans
                .iter()
                .any(|span| { span.text == "vwx" && span.style.bold && span.style.inverse }),
            "DECRARA should reverse selected attributes: {:?}",
            second.spans
        );
    }

    #[test]
    fn native_scroll_region_origin_mode_positions_rich_output_inside_margins() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"one\r\ntwo\r\nthree\r\nfour\
              \x1b[2;3r\x1b[?6h\x1b[1;1H\x1b[31mX\x1b[0m\
              \x1b[?6l\x1b[1;1HY",
        );

        let surface = buffer.render(None).surface;
        assert_eq!(surface.lines[0].text, "Yne");
        assert_eq!(surface.lines[1].text, "Xwo");
        assert_eq!(surface.lines[2].text, "three");
        assert_eq!(surface.lines[3].text, "four");
        assert!(
            surface.lines[1].spans.iter().any(|span| {
                span.text == "X"
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            }),
            "origin-mode write should keep rich foreground spans: {:?}",
            surface.lines[1].spans
        );
    }

    #[test]
    fn native_scroll_region_linefeed_scrolls_only_margins() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"one\r\ntwo\r\nthree\r\nfour\x1b[2;3r\x1b[3;1H\nX");

        let surface = buffer.render(None).surface;
        assert_eq!(surface.lines[0].text, "one");
        assert_eq!(surface.lines[1].text, "three");
        assert_eq!(surface.lines[2].text, "X");
        assert_eq!(surface.lines[3].text, "four");
    }

    #[test]
    fn resolves_dynamic_palette_overrides_from_modern_css_rgb_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;22;rgb(100% 50% 0%);23;rgba(12 34 56 / 40%);24;#11223344;25;color(srgb 1 0.5 0 / 40%)\x07\
              \x1b[38;5;22mfg\x1b[0m \
              \x1b[48;5;23mbg\x1b[0m \
              \x1b[38;5;24mhex-alpha\x1b[0m \
              \x1b[38;5;25mcolor-srgb\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("fg"))
            .expect("modern CSS color palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "fg"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
            }),
            "OSC 4 CSS rgb percentage color should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bg"
                    && span.style.background == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
            }),
            "OSC 4 CSS rgba slash-alpha color should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "hex-alpha"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 17, g: 34, b: 51 })
            }),
            "OSC 4 CSS hex-alpha color should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "color-srgb"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
            }),
            "OSC 4 CSS color(srgb) color should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_from_legacy_hex_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;#123456;22;#aabbcc\x07\
              \x1b[31mred\x1b[0m \
              \x1b[38;5;22midx\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("legacy hex dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x12, g: 0x34, b: 0x56 })
            }),
            "OSC 4 legacy hex ANSI color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
            }),
            "OSC 4 legacy hex indexed color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_from_rgbi_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;rgbi:1.0/0.5/0.0;22;rgbi:0.0/0.25/1.0\x07\
              \x1b[31mred\x1b[0m \
              \x1b[38;5;22midx\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("rgbi dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 })
            }),
            "OSC 4 rgbi ANSI color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0, g: 64, b: 255 })
            }),
            "OSC 4 rgbi indexed color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_from_rgba_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;rgba:1212/3434/5656/7878;22;rgba(10, 11, 12, 0.5)\x07\
              \x1b[31mred\x1b[0m \
              \x1b[38;5;22midx\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("rgba dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 })
            }),
            "OSC 4 rgba ANSI color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 10, g: 11, b: 12 })
            }),
            "OSC 4 rgba indexed color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_from_compact_hex_and_color_space_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;rgb:abc;2;srgb:102030;22;p3:405060\x07\
              \x1b[31mred\x1b[0m \
              \x1b[32mgreen\x1b[0m \
              \x1b[38;5;22midx\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("compact hex dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 170, g: 187, b: 204 })
            }),
            "OSC 4 compact rgb hex ANSI color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "green"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x10, g: 0x20, b: 0x30 })
            }),
            "OSC 4 srgb color-space ANSI color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x40, g: 0x50, b: 0x60 })
            }),
            "OSC 4 p3 color-space indexed color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_legacy_linux_console_palette_updates_for_ansi_colors() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]P1aabbcc\x07\x1b[31mred\x1b[0m \
              \x1b]PAddeeff\x07\x1b[92mbright-green\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("legacy Linux console palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
            }),
            "legacy Linux console palette should recolor ANSI red: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "bright-green"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xdd, g: 0xee, b: 0xff })
            }),
            "legacy Linux console palette should recolor ANSI bright green: {:?}",
            line.spans
        );

        let reset_buffer = EmulatorBuffer::new(4, 80);
        reset_buffer.advance(b"\x1b]P1aabbcc\x07\x1b]R\x07\x1b[31mreset-red\x1b[0m");
        let reset_surface = reset_buffer.render(None).surface;
        let reset_line = reset_surface
            .lines
            .iter()
            .find(|line| line.text.contains("reset-red"))
            .expect("legacy Linux console palette reset line should render");
        assert!(
            reset_line.spans.iter().any(|span| {
                span.text == "reset-red"
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            }),
            "legacy Linux console palette reset should restore ANSI red: {:?}",
            reset_line.spans
        );
    }

    #[test]
    fn applies_valid_dynamic_palette_pairs_around_invalid_pairs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;bad;#000000;22;#010203;300;#ffffff;23;#040506;orphan\x07\
              \x1b[38;5;22mfirst\x1b[0m \
              \x1b[38;5;23msecond\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("first"))
            .expect("partial dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "first"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
            }),
            "OSC 4 should apply valid pairs after invalid pairs: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "second"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 })
            }),
            "OSC 4 should ignore trailing orphan fields without dropping valid pairs: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_palette_overrides_from_named_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;red;22;gray50;23;Rebecca Purple;24;Light Sea Green\x07\
              \x1b[31mred\x1b[0m \
              \x1b[38;5;22mgray\x1b[0m \
              \x1b[38;5;23mpurple\x1b[0m \
              \x1b[38;5;24msea\x1b[0m",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("red"))
            .expect("named dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "red"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xff, g: 0, b: 0 })
            }),
            "OSC 4 named ANSI color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "gray"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x80, g: 0x80, b: 0x80 })
            }),
            "OSC 4 grayN indexed color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "purple"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x66, g: 0x33, b: 0x99 })
            }),
            "OSC 4 CSS/X11 named color override should render as RGB: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "sea"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0x20, g: 0xb2, b: 0xaa })
            }),
            "OSC 4 space separated named color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_tmux_wrapped_named_palette_overrides() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x1bPtmux;\x1b\x1b]4;22;Light Blue\x07\x1b\\\x1b[38;5;22midx\x1b[0m");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("idx"))
            .expect("tmux wrapped named dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xad, g: 0xd8, b: 0xe6 })
            }),
            "tmux wrapped OSC 4 named color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_c1_named_palette_overrides() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x9d4;22;red\x9c\x1b[38;5;22midx\x1b[0m");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("idx"))
            .expect("C1 named dynamic palette line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "idx"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xff, g: 0, b: 0 })
            }),
            "C1 OSC 4 named color override should render as RGB: {:?}",
            line.spans
        );
    }

    #[test]
    fn resets_dynamic_palette_overrides_back_to_indexed_contract() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;22;rgb:aa/bb/cc\x07\
              \x1b[38;5;22mcustom\x1b[0m",
        );

        let custom_surface = buffer.render(None).surface;
        let custom_line = custom_surface
            .lines
            .iter()
            .find(|line| line.text.contains("custom"))
            .expect("dynamic palette line should render before reset");

        assert!(
            custom_line.spans.iter().any(|span| {
                span.text == "custom"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 0xaa, g: 0xbb, b: 0xcc })
            }),
            "OSC 4 indexed color should render as RGB before reset: {:?}",
            custom_line.spans
        );

        buffer.advance(b" \x1b]104;22\x07\x1b[38;5;22mreset\x1b[0m");
        let reset_surface = buffer.render(None).surface;
        let reset_line = reset_surface
            .lines
            .iter()
            .find(|line| line.text.contains("reset"))
            .expect("dynamic palette reset line should render");

        assert!(
            reset_line.spans.iter().any(|span| {
                span.text == "reset"
                    && span.style.foreground == Some(ScreenColor::Indexed { index: 22 })
            }),
            "OSC 104 reset should restore indexed color contract: {:?}",
            reset_line.spans
        );
    }

    #[test]
    fn resets_all_named_dynamic_palette_overrides_back_to_indexed_contract() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;1;red;22;gray50\x07\
              \x1b[31mred\x1b[0m \
              \x1b[38;5;22mgray\x1b[0m",
        );
        buffer.advance(b" \x1b]104\x07\x1b[31mreset-red\x1b[0m \x1b[38;5;22mreset-gray\x1b[0m");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("reset-red"))
            .expect("dynamic palette reset-all line should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "reset-red"
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            }),
            "OSC 104 reset-all should restore ANSI named color contract: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "reset-gray"
                    && span.style.foreground == Some(ScreenColor::Indexed { index: 22 })
            }),
            "OSC 104 reset-all should restore indexed color contract: {:?}",
            line.spans
        );
    }

    #[test]
    fn resolves_dynamic_default_surface_palette_from_legacy_hex_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;#010203\x07\
              \x1b]11;#0a0b0c\x07\
              \x1b]12;#ddeeff\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 0x0a, g: 0x0b, b: 0x0c }),
                cursor: Some(ScreenColor::Rgb { r: 0xdd, g: 0xee, b: 0xff }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after legacy hex palette overrides: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resolves_dynamic_default_surface_palette_from_rgbi_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;rgbi:1.0/0.5/0.0\x07\
              \x1b]11;rgbi:0.0/0.25/1.0\x07\
              \x1b]12;rgbi:0.1/0.2/0.3\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 }),
                background: Some(ScreenColor::Rgb { r: 0, g: 64, b: 255 }),
                cursor: Some(ScreenColor::Rgb { r: 26, g: 51, b: 77 }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after rgbi palette overrides: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resolves_dynamic_default_surface_palette_from_rgba_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;rgba:1212/3434/5656/7878\x07\
              \x1b]11;rgba:0a0a/0b0b/0c0c/ffff\x07\
              \x1b]12;rgba(13, 14, 15, 50%)\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 18, g: 52, b: 86 }),
                background: Some(ScreenColor::Rgb { r: 10, g: 11, b: 12 }),
                cursor: Some(ScreenColor::Rgb { r: 13, g: 14, b: 15 }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after rgba palette overrides: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resolves_dynamic_default_surface_palette_from_modern_css_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;rgb(100% 50% 0%)\x07\
              \x1b]11;#11223344\x07\
              \x1b]12;color(srgb 1 0.5 0 / 40%)\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 }),
                background: Some(ScreenColor::Rgb { r: 17, g: 34, b: 51 }),
                cursor: Some(ScreenColor::Rgb { r: 255, g: 128, b: 0 }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after modern CSS palette overrides: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resets_named_dynamic_default_surface_palette() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;white\x07\
              \x1b]11;black\x07\
              \x1b]12;Light Blue\x07plain",
        );
        buffer.advance(b"\x1b]110\x07\x1b]111\x07\x1b]112\x07");

        let surface = buffer.render(None).surface;

        assert_eq!(surface.palette, ScreenSurfacePalette::default());
    }

    #[test]
    fn resolves_dynamic_default_surface_palette_from_named_color_specs() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;white\x07\
              \x1b]11;black\x07\
              \x1b]12;Light Blue\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 0xff, g: 0xff, b: 0xff }),
                background: Some(ScreenColor::Rgb { r: 0, g: 0, b: 0 }),
                cursor: Some(ScreenColor::Rgb { r: 0xad, g: 0xd8, b: 0xe6 }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after named palette overrides: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resolves_dynamic_default_surface_palette_from_osc_10_11_12() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]10;rgb:01/02/03\x07\
              \x1b]11;rgb:04/05/06\x07\
              \x1b]12;rgb:07/08/09\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: Some(ScreenColor::Rgb { r: 7, g: 8, b: 9 }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render with inherited palette: {:?}",
            surface.lines
        );

        buffer.advance(b"\x1b]110\x07\x1b]111\x07\x1b]112\x07");
        let reset_surface = buffer.render(None).surface;

        assert_eq!(reset_surface.palette, ScreenSurfacePalette::default());
    }

    #[test]
    fn resolves_surface_palette_from_osc4_extended_default_slots() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x1b]4;256;rgb:01/02/03;257;rgb:04/05/06;258;rgb:07/08/09\x07plain");

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: Some(ScreenColor::Rgb { r: 7, g: 8, b: 9 }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render with OSC 4 extended default slots: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resolves_surface_palette_from_iterm2_osc4_default_aliases() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x1b]4;-1;rgb:01/02/03;-2;rgb:04/05/06\x07plain");

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: None,
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render with iTerm2 OSC 4 default aliases: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resolves_surface_palette_from_iterm2_osc_p_default_slots() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x1b]Pg112233\x07\x1b]Ph445566\x07\x1b]Pl778899\x07plain");

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 }),
                background: Some(ScreenColor::Rgb { r: 0x44, g: 0x55, b: 0x66 }),
                cursor: Some(ScreenColor::Rgb { r: 0x77, g: 0x88, b: 0x99 }),
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render with iTerm2 OSC P default slots: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resets_surface_palette_from_osc104_extended_default_slots() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;256;rgb:01/02/03;257;rgb:04/05/06;258;rgb:07/08/09\x07\
              \x1b]104;256;258\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: None,
                background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
                cursor: None,
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after OSC 104 extended slot reset: {:?}",
            surface.lines
        );
    }

    #[test]
    fn resets_surface_palette_from_iterm2_osc4_default_aliases() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;-1;rgb:01/02/03;-2;rgb:04/05/06\x07\
              \x1b]104;-2\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: None,
                cursor: None,
            }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after iTerm2 OSC 104 alias reset: {:?}",
            surface.lines
        );
    }

    #[test]
    fn reset_all_surface_palette_clears_osc4_extended_default_slots() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]4;256;rgb:01/02/03;257;rgb:04/05/06;258;rgb:07/08/09\x07\
              \x1b]104\x07plain",
        );

        let surface = buffer.render(None).surface;

        assert_eq!(surface.palette, ScreenSurfacePalette::default());
        assert!(
            surface.lines.iter().any(|line| line.text.contains("plain")),
            "plain output should still render after OSC 104 reset-all: {:?}",
            surface.lines
        );
    }

    #[test]
    fn holds_synchronized_output_surface_snapshot_until_end() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"stable");

        buffer.advance(
            b"\x1b[?2026h\
              \x1b]2;Hidden title\x07\
              \x1b]10;rgb:11/22/33\x07\
              \x1b[38;2;1;2;3mhidden",
        );

        let during = buffer.render(Some("Fallback".to_string())).surface;
        assert_eq!(during.title.as_deref(), Some("Fallback"));
        assert_eq!(during.palette, ScreenSurfacePalette::default());
        assert!(
            during.lines.iter().any(|line| line.text.contains("stable")),
            "synchronized output should keep the last stable frame visible: {:?}",
            during.lines
        );
        assert!(
            !during.lines.iter().any(|line| line.text.contains("hidden")),
            "synchronized output should not leak pending text before ESU: {:?}",
            during.lines
        );

        buffer.advance(b"\x1b[0m\x1b[?2026l");
        let after = buffer.render(Some("Fallback".to_string())).surface;
        assert_eq!(after.title.as_deref(), Some("Hidden title"));
        assert_eq!(
            after.palette,
            ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 0x11, g: 0x22, b: 0x33 }),
                background: None,
                cursor: None,
            }
        );
        let line = after
            .lines
            .iter()
            .find(|line| line.text.contains("stablehidden"))
            .expect("synchronized output should be revealed after ESU");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "hidden"
                    && span.style.foreground == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
            }),
            "synchronized colored text should keep its SGR style after ESU: {:?}",
            line.spans
        );
    }

    #[test]
    fn holds_split_chunk_synchronized_output_until_end() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"base\x1b[?2026h");
        buffer.advance(b"\x1b[32mpending");

        let during = buffer.render(None).surface;
        assert!(
            during.lines.iter().any(|line| line.text.contains("base")),
            "split synchronized output should keep pre-BSU text visible: {:?}",
            during.lines
        );
        assert!(
            !during.lines.iter().any(|line| line.text.contains("pending")),
            "split synchronized output should not leak pending text: {:?}",
            during.lines
        );

        buffer.advance(b"\x1b[0m\x1b[?2026l");
        let after = buffer.render(None).surface;
        let line = after
            .lines
            .iter()
            .find(|line| line.text.contains("basepending"))
            .expect("split synchronized output should render after ESU");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "pending"
                    && span.style.foreground
                        == Some(ScreenColor::Named { name: "green".to_string() })
            }),
            "split synchronized output should preserve pending SGR state: {:?}",
            line.spans
        );
    }

    #[test]
    fn holds_c1_synchronized_output_surface_snapshot_until_end() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"base\x9b?2026h\x1b]2;Pending C1\x07visible");

        let during = buffer.render(Some("Fallback".to_string())).surface;
        assert!(
            during.lines.iter().any(|line| line.text.contains("base")),
            "C1 synchronized output should keep the last stable frame visible: {:?}",
            during.lines
        );
        assert!(
            !during.lines.iter().any(|line| line.text.contains("visible")),
            "C1 synchronized output should not leak pending text before reset: {:?}",
            during.lines
        );
        assert_eq!(during.title.as_deref(), Some("Fallback"));

        buffer.advance(b"\x9b?2026l");
        let after = buffer.render(Some("Fallback".to_string())).surface;
        assert_eq!(after.title.as_deref(), Some("Pending C1"));
        assert!(
            after.lines.iter().any(|line| line.text.contains("basevisible")),
            "C1 synchronized output should render pending text after reset: {:?}",
            after.lines
        );
    }

    #[test]
    fn preserves_dec_special_graphics_line_drawing_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b(0lqk\x1b(B ascii \x1b(0x\x1b(B");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("ascii"))
            .expect("line drawing output should render");

        assert_eq!(line.text, "┌─┐ ascii │");
    }

    #[test]
    fn counts_terminal_bell_events_in_rendered_surface() {
        let buffer = EmulatorBuffer::new(4, 80);

        assert_eq!(buffer.render(None).surface.bell_count, 0);

        buffer.advance(b"\x07ready\x07");
        let surface = buffer.render(None).surface;

        assert_eq!(surface.bell_count, 2);
        assert!(
            surface.lines.iter().any(|line| line.text.contains("ready")),
            "BEL should not suppress surrounding terminal text: {:?}",
            surface.lines
        );
    }

    #[test]
    fn tracks_terminal_title_from_osc_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        assert_eq!(
            buffer.render(Some("Fallback".to_string())).surface.title.as_deref(),
            Some("Fallback")
        );

        buffer.advance(b"\x1b]2;Shell title\x07prompt");
        let osc2_surface = buffer.render(Some("Fallback".to_string())).surface;

        assert_eq!(osc2_surface.title.as_deref(), Some("Shell title"));
        assert!(
            osc2_surface.lines.iter().any(|line| line.text.contains("prompt")),
            "title OSC output should not suppress surrounding text: {:?}",
            osc2_surface.lines
        );

        buffer.advance(b"\x1b]0;Icon and window title\x07");
        let osc0_surface = buffer.render(Some("Fallback".to_string())).surface;

        assert_eq!(osc0_surface.title.as_deref(), Some("Icon and window title"));
    }

    #[test]
    fn tracks_terminal_icon_title_from_osc1_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b]1;Icon shell\x07prompt");
        let surface = buffer.render(Some("Fallback".to_string())).surface;

        assert_eq!(surface.title.as_deref(), Some("Icon shell"));
        assert!(
            surface.lines.iter().any(|line| line.text.contains("prompt")),
            "OSC 1 output should not suppress surrounding text: {:?}",
            surface.lines
        );
    }

    #[test]
    fn tracks_terminal_working_directory_uri_from_osc7_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        assert_eq!(buffer.render(None).surface.working_directory_uri, None);

        buffer
            .advance(b"\x1b]7;file://MacBook-Pro-belief.local/Users/belief/dev%20space\x07prompt");
        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.working_directory_uri.as_deref(),
            Some("file://MacBook-Pro-belief.local/Users/belief/dev%20space")
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("prompt")),
            "OSC 7 metadata should not suppress surrounding text: {:?}",
            surface.lines
        );
    }

    #[test]
    fn tracks_terminal_working_directory_uri_with_st_terminator() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]7;file://localhost/tmp/project\x1b\\");

        assert_eq!(
            buffer.render(None).surface.working_directory_uri.as_deref(),
            Some("file://localhost/tmp/project")
        );
    }

    #[test]
    fn tracks_terminal_metadata_from_c1_osc_output() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x9d7;file://localhost/tmp/c1-project\x9cready");

        let surface = buffer.render(None).surface;
        assert_eq!(
            surface.working_directory_uri.as_deref(),
            Some("file://localhost/tmp/c1-project")
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("ready")),
            "C1 OSC metadata should not suppress surrounding text: {:?}",
            surface.lines
        );
    }

    #[test]
    fn tracks_tmux_wrapped_terminal_working_directory_uri() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x1bPtmux;\x1b\x1b]7;file://localhost/tmp/tmux-project\x07\x1b\\prompt");

        let surface = buffer.render(None).surface;
        assert_eq!(
            surface.working_directory_uri.as_deref(),
            Some("file://localhost/tmp/tmux-project")
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("prompt")),
            "tmux passthrough metadata should not suppress surrounding text: {:?}",
            surface.lines
        );
    }

    #[test]
    fn preserves_tmux_wrapped_osc8_hyperlink_output() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1bPtmux;\x1b\x1b]8;;https://example.test\x1b\x1b\\\x1b\\link\
              \x1bPtmux;\x1b\x1b]8;;\x1b\x1b\\\x1b\\ plain",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("link"))
            .expect("line around tmux wrapped OSC 8 hyperlink should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "link"
                    && span.style.hyperlink.as_deref() == Some("https://example.test")
            }),
            "tmux wrapped OSC 8 hyperlink should apply to link text: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.contains("plain") && span.style.hyperlink.is_none()),
            "tmux wrapped OSC 8 reset should clear hyperlink before plain text: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_c1_tmux_wrapped_osc8_hyperlink_output() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x90tmux;\x1b\x1b]8;;https://c1.example\x1b\x1b\\\x9clink\
              \x90tmux;\x1b\x1b]8;;\x1b\x1b\\\x9c plain",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("link"))
            .expect("line around C1 tmux wrapped OSC 8 hyperlink should render");

        assert!(
            line.spans.iter().any(|span| span.text == "link"
                && span.style.hyperlink.as_deref() == Some("https://c1.example")),
            "C1 tmux wrapped OSC 8 hyperlink should apply to link text: {:?}",
            line.spans
        );
        assert!(
            line.spans
                .iter()
                .any(|span| span.text.contains("plain") && span.style.hyperlink.is_none()),
            "C1 tmux wrapped OSC 8 reset should clear hyperlink: {:?}",
            line.spans
        );
    }

    #[test]
    fn preserves_osc8_hyperlink_params_and_switches_without_leaking_payloads() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1b]8;id=one;https://one.example\x1b\\one\
              \x1b]8;id=two;https://two.example\x1b\\two\
              \x1b]8;;\x1b\\ plain",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("onetwo"))
            .expect("line around OSC 8 hyperlink switches should render");

        assert_eq!(line.text, "onetwo plain");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "one" && span.style.hyperlink.as_deref() == Some("https://one.example")
            }),
            "OSC 8 hyperlink params should not hide the first link: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text == "two" && span.style.hyperlink.as_deref() == Some("https://two.example")
            }),
            "OSC 8 should switch to the next link without an explicit close: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| span.text == " plain" && span.style.hyperlink.is_none()),
            "OSC 8 reset should clear hyperlink before plain text: {:?}",
            line.spans
        );
        assert!(!line.text.contains("id=one"));
        assert!(!line.text.contains("id=two"));
    }

    #[test]
    fn preserves_tmux_wrapped_extended_sgr_visual_output() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(
            b"\x1bPtmux;\x1b\x1b[58;2;1;2;3m\x1b\x1b[4m\x1b\\colored\
              \x1bPtmux;\x1b\x1b[0m\x1b\\ plain",
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("colored"))
            .expect("line around tmux wrapped extended SGR should render");

        assert!(
            line.spans.iter().any(|span| {
                span.text == "colored"
                    && span.style.underline == Some(ScreenUnderlineStyle::Single)
                    && span.style.underline_color == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
            }),
            "tmux wrapped extended SGR should preserve underline color: {:?}",
            line.spans
        );
        assert!(
            line.spans.iter().any(|span| {
                span.text.contains("plain")
                    && span.style.underline.is_none()
                    && span.style.underline_color.is_none()
            }),
            "tmux wrapped SGR reset should clear extended style: {:?}",
            line.spans
        );
    }

    #[test]
    fn tracks_terminal_working_directory_uri_from_windows_terminal_osc99_output() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(b"\x1b]9;9;\"C:\\Users\\belief\\dev space\"\x07prompt");
        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.working_directory_uri.as_deref(),
            Some("file:///C:/Users/belief/dev%20space")
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("prompt")),
            "OSC 9;9 metadata should not suppress surrounding text: {:?}",
            surface.lines
        );
    }

    #[test]
    fn tracks_terminal_working_directory_uri_from_vscode_and_iterm2_output() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(b"\x1b]633;P;Cwd=/tmp/dev space;IsWindows=False\x07");
        assert_eq!(
            buffer.render(None).surface.working_directory_uri.as_deref(),
            Some("file://localhost/tmp/dev%20space")
        );

        buffer.advance(b"\x1b]1337;CurrentDir=/Users/belief/next project\x1b\\");
        assert_eq!(
            buffer.render(None).surface.working_directory_uri.as_deref(),
            Some("file://localhost/Users/belief/next%20project")
        );
    }

    #[test]
    fn tracks_terminal_user_variables_from_osc1337_set_user_var_output() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(b"\x1b]1337;SetUserVar=WEZTERM_PROG=Y2FyZ28gdGVzdA==\x07prompt");
        let surface = buffer.render(None).surface;

        assert_eq!(surface.user_variables.get("WEZTERM_PROG"), Some(&"cargo test".to_string()));
        assert!(
            surface.lines.iter().any(|line| line.text.contains("prompt")),
            "SetUserVar metadata should not suppress surrounding text: {:?}",
            surface.lines
        );
        assert!(
            surface.lines.iter().all(|line| !line.text.contains("Y2FyZ28")),
            "SetUserVar payload should not leak into visible text: {:?}",
            surface.lines
        );

        buffer.advance(b"\x1b]1337;SetUserVar=WEZTERM_USER=YmVsaWVm\x1b\\");
        assert_eq!(
            buffer.render(None).surface.user_variables.get("WEZTERM_USER"),
            Some(&"belief".to_string())
        );
    }

    #[test]
    fn ignores_invalid_terminal_user_variable_metadata() {
        use base64::Engine as _;

        let buffer = EmulatorBuffer::new(4, 160);

        buffer.advance(b"\x1b]1337;SetUserVar=WEZTERM_PROG=Z2l0IHN0YXR1cw==\x07");
        buffer.advance(b"\x1b]1337;SetUserVar=BAD KEY=YmFk\x07");
        buffer.advance(b"\x1b]1337;SetUserVar=BAD=not-valid-base64\x07");
        buffer.advance(b"\x1b]1337;SetUserVar=BAD_CONTROL=YQpi\x07");
        let large = BASE64_STANDARD.encode("x".repeat(4097));
        buffer.advance(format!("\x1b]1337;SetUserVar=TOO_LARGE={large}\x07").as_bytes());

        let user_variables = buffer.render(None).surface.user_variables;
        assert_eq!(user_variables.get("WEZTERM_PROG"), Some(&"git status".to_string()));
        assert_eq!(user_variables.len(), 1, "invalid SetUserVar payloads must be ignored");
    }

    #[test]
    fn ignores_invalid_terminal_working_directory_uri_metadata() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]7;file://localhost/tmp/project\x07");
        buffer.advance(b"\x1b]7;https://example.com/not-a-cwd\x07");
        buffer.advance(b"\x1b]9;9;relative/project\x07");
        buffer.advance(b"\x1b]633;P;Cwd=relative/project;IsWindows=False\x07");

        assert_eq!(
            buffer.render(None).surface.working_directory_uri.as_deref(),
            Some("file://localhost/tmp/project")
        );
    }

    #[test]
    fn tracks_terminal_progress_from_osc94_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        assert_eq!(buffer.render(None).surface.progress, ScreenProgress::default());

        buffer.advance(b"\x1b]9;4;1;42\x07build");
        let surface = buffer.render(None).surface;

        assert_eq!(
            surface.progress,
            ScreenProgress { state: ScreenProgressState::Normal, value: Some(42) }
        );
        assert!(
            surface.lines.iter().any(|line| line.text.contains("build")),
            "OSC 9;4 metadata should not suppress surrounding text: {:?}",
            surface.lines
        );

        buffer.advance(b"\x1b]9;4;2;180\x1b\\");
        assert_eq!(
            buffer.render(None).surface.progress,
            ScreenProgress { state: ScreenProgressState::Error, value: Some(100) }
        );

        buffer.advance(b"\x1b]9;4;3\x07");
        assert_eq!(
            buffer.render(None).surface.progress,
            ScreenProgress { state: ScreenProgressState::Indeterminate, value: None }
        );

        buffer.advance(b"\x1b]9;4\x07");
        assert_eq!(buffer.render(None).surface.progress, ScreenProgress::default());

        buffer.advance(b"\x1b]9;4;1;12\x07");
        buffer.advance(b"\x1b]9;4;0\x07");
        assert_eq!(buffer.render(None).surface.progress, ScreenProgress::default());
    }

    #[test]
    fn ignores_invalid_terminal_progress_metadata() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]9;4;1;55\x07");
        buffer.advance(b"\x1b]9;4;9;99\x07");
        buffer.advance(b"\x1b]9;4;warning;99\x07");

        assert_eq!(
            buffer.render(None).surface.progress,
            ScreenProgress { state: ScreenProgressState::Normal, value: Some(55) }
        );
    }

    #[test]
    fn tracks_shell_integration_marks_from_osc133_output() {
        let buffer = EmulatorBuffer::new(6, 120);

        buffer.advance(
            b"\x1b]133;A\x07$ \x1b]133;B\x07echo hi\r\n\
              \x1b]133;C\x07hi\r\n\
              \x1b]133;D;127\x07",
        );

        let surface = buffer.render(None).surface;
        assert!(
            surface.lines[0].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::PromptStart
                    && mark.col == 0
                    && mark.exit_code.is_none()
            }),
            "prompt start mark should stay on prompt row: {:?}",
            surface.lines[0].semantic_marks
        );
        assert!(
            surface.lines[0].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::InputStart
                    && mark.col == 2
                    && mark.exit_code.is_none()
            }),
            "input start mark should stay on command row: {:?}",
            surface.lines[0].semantic_marks
        );
        assert!(
            surface.lines[1].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::OutputStart
                    && mark.col == 0
                    && mark.exit_code.is_none()
            }),
            "output start mark should stay on first output row: {:?}",
            surface.lines[1].semantic_marks
        );
        assert!(
            surface.lines[2].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::CommandFinished
                    && mark.col == 0
                    && mark.exit_code == Some(127)
            }),
            "command-finished mark should include exit code: {:?}",
            surface.lines[2].semantic_marks
        );
    }

    #[test]
    fn tracks_shell_integration_marks_from_vscode_osc633_output() {
        let buffer = EmulatorBuffer::new(6, 120);

        buffer.advance(
            b"\x1b]633;A\x07> \x1b]633;B\x07cargo test\r\n\
              \x1b]633;C\x1b\\ok\r\n\
              \x1b]633;D;2\x07",
        );

        let surface = buffer.render(None).surface;
        assert!(
            surface.lines[0].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::PromptStart
                    && mark.col == 0
                    && mark.exit_code.is_none()
            }),
            "VS Code prompt start mark should stay on prompt row: {:?}",
            surface.lines[0].semantic_marks
        );
        assert!(
            surface.lines[0].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::InputStart
                    && mark.col == 2
                    && mark.exit_code.is_none()
            }),
            "VS Code prompt end mark should become input start: {:?}",
            surface.lines[0].semantic_marks
        );
        assert!(
            surface.lines[1].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::OutputStart
                    && mark.col == 0
                    && mark.exit_code.is_none()
            }),
            "VS Code pre-execution mark should become output start: {:?}",
            surface.lines[1].semantic_marks
        );
        assert!(
            surface.lines[2].semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::CommandFinished
                    && mark.col == 0
                    && mark.exit_code == Some(2)
            }),
            "VS Code command-finished mark should include exit code: {:?}",
            surface.lines[2].semantic_marks
        );
    }

    #[test]
    fn tracks_shell_integration_marks_with_st_terminator() {
        let buffer = EmulatorBuffer::new(3, 80);

        buffer.advance(b"\x1b]133;A\x1b\\prompt");

        let surface = buffer.render(None).surface;
        assert!(
            surface.lines[0]
                .semantic_marks
                .iter()
                .any(|mark| mark.kind == ScreenLineSemanticMarkKind::PromptStart),
            "OSC 133 ST-terminated prompt mark should survive: {:?}",
            surface.lines[0].semantic_marks
        );
    }

    #[test]
    fn preserves_shell_integration_marks_outside_partial_clear_ranges() {
        let buffer = EmulatorBuffer::new(3, 80);

        buffer.advance(b"\x1b]133;A\x07PS1 \x1b]133;B\x07cmdtail\r\x1b[7C\x1b[K");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("PS1"))
            .expect("partially cleared prompt line should still render");

        assert_eq!(line.text, "PS1 cmd");
        assert!(
            line.semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::PromptStart && mark.col == 0
            }),
            "prompt mark to the left of EL range should survive: {:?}",
            line.semantic_marks
        );
        assert!(
            line.semantic_marks.iter().any(|mark| {
                mark.kind == ScreenLineSemanticMarkKind::InputStart && mark.col == 4
            }),
            "input mark to the left of EL range should survive: {:?}",
            line.semantic_marks
        );
    }

    #[test]
    fn ignores_invalid_shell_integration_marks() {
        let buffer = EmulatorBuffer::new(3, 80);

        buffer.advance(b"\x1b]133;Z\x07ready");
        buffer.advance(b"\x1b]633;E;echo ignored;nonce\x07");

        let surface = buffer.render(None).surface;
        assert!(surface.lines.iter().all(|line| line.semantic_marks.is_empty()));
    }

    #[test]
    fn queues_terminal_device_attribute_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[c");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?6c");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_terminal_secondary_device_attribute_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[>c");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[>0;2600;1c");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_terminal_secondary_device_attribute_response_bytes_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[?2026h\x1b[>c");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[>0;2600;1c");

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_c1_terminal_secondary_device_attribute_response_bytes_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x9b?2026h\x9b>c");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[>0;2600;1c");

        buffer.advance(b"\x9b?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_terminal_xterm_version_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[>0q");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            terminal_xterm_version_response_bytes()
        );
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_terminal_xterm_version_response_bytes_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[?2026h\x1b[>q");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            terminal_xterm_version_response_bytes()
        );

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_c1_terminal_xterm_version_response_bytes_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x9b?2026h\x9b>0q");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            terminal_xterm_version_response_bytes()
        );

        buffer.advance(b"\x9b?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_default_sgr_decrqss_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1bP$qm\x1b\\");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1bP1$r0m\x1b\\");

        let surface = buffer.render(None).surface;
        assert!(surface.lines.iter().all(|line| !line.text.contains("$qm")));
    }

    #[test]
    fn queues_truecolor_sgr_decrqss_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[38;2;1;2;3;48;5;22;4:3;58;2;9;8;7m\x1bP$qm\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1bP1$r4:3;38;2;1;2;3;48;5;22;58;2;9;8;7m\x1b\\"
        );
    }

    #[test]
    fn queues_named_and_extended_sgr_decrqss_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[1;2;3;5;7;8;9;33;104;51;53;73m\x1bP$qm\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1bP1$r1;2;3;5;7;8;9;33;104;53;51;73m\x1b\\"
        );
    }

    #[test]
    fn queues_reset_sgr_decrqss_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[38;2;1;2;3;48;5;22;1;4:3m\x1b[0m\x1bP$qm\x1b\\");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1bP1$r0m\x1b\\");
    }

    #[test]
    fn queues_c1_sgr_decrqss_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[96m\x90$qm\x9c");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1bP1$r96m\x1b\\");
    }

    #[test]
    fn queues_sgr_decrqss_response_bytes_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[?2026h\x1b[38;2;1;2;3m\x1bP$qm\x1b\\");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1bP1$r38;2;1;2;3m\x1b\\");

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_xtgettcap_truecolor_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1bP+q524742\x1b\\");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1bP1+r524742=38\x1b\\");

        let surface = buffer.render(None).surface;
        assert!(surface.lines.iter().all(|line| !line.text.contains("+q524742")));
    }

    #[test]
    fn queues_xtgettcap_truecolor_boolean_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1bP+q5463\x1b\\");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1bP1+r5463\x1b\\");
    }

    #[test]
    fn queues_xtgettcap_truecolor_string_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer
            .advance(b"\x1bP+q434f;73657472676266;73657472676262;736574663234;736574623234\x1b\\");

        let expected = format!(
            "\x1bP1+r434f=38\x1b\\\
             \x1bP1+r73657472676266={}\x1b\\\
             \x1bP1+r73657472676262={}\x1b\\\
             \x1bP1+r736574663234={}\x1b\\\
             \x1bP1+r736574623234={}\x1b\\",
            encode_terminal_hex_bytes(b"\x1b[38;2;%p1%d;%p2%d;%p3%dm"),
            encode_terminal_hex_bytes(b"\x1b[48;2;%p1%d;%p2%d;%p3%dm"),
            encode_terminal_hex_bytes(
                b"\x1b[38;2;%p1%{65536}%/%d;%p1%{256}%/%{255}%&%d;%p1%{255}%&%dm"
            ),
            encode_terminal_hex_bytes(
                b"\x1b[48;2;%p1%{65536}%/%d;%p1%{256}%/%{255}%&%d;%p1%{255}%&%dm"
            ),
        )
        .into_bytes();

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), expected);
    }

    #[test]
    fn queues_xtgettcap_indexed_color_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        let query = format!(
            "\x1bP+q{};{};{};{};{};{};{};{}\x1b\\",
            encode_terminal_hex_bytes(b"setaf"),
            encode_terminal_hex_bytes(b"setab"),
            encode_terminal_hex_bytes(b"op"),
            encode_terminal_hex_bytes(b"initc"),
            encode_terminal_hex_bytes(b"oc"),
            encode_terminal_hex_bytes(b"ccc"),
            encode_terminal_hex_bytes(b"pairs"),
            encode_terminal_hex_bytes(b"bce"),
        );
        buffer.advance(query.as_bytes());

        let expected = format!(
            "\x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}\x1b\\",
            encode_terminal_hex_bytes(b"setaf"),
            encode_terminal_hex_bytes(
                b"\x1b[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m"
            ),
            encode_terminal_hex_bytes(b"setab"),
            encode_terminal_hex_bytes(
                b"\x1b[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m"
            ),
            encode_terminal_hex_bytes(b"op"),
            encode_terminal_hex_bytes(b"\x1b[39;49m"),
            encode_terminal_hex_bytes(b"initc"),
            encode_terminal_hex_bytes(
                b"\x1b]4;%p1%d;rgb:%p2%{255}%*%{1000}%/%2.2X/%p3%{255}%*%{1000}%/%2.2X/%p4%{255}%*%{1000}%/%2.2X\x1b\\"
            ),
            encode_terminal_hex_bytes(b"oc"),
            encode_terminal_hex_bytes(b"\x1b]104\x1b\\"),
            encode_terminal_hex_bytes(b"ccc"),
            encode_terminal_hex_bytes(b"pairs"),
            encode_terminal_hex_bytes(b"65536"),
            encode_terminal_hex_bytes(b"bce"),
        )
        .into_bytes();

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), expected);
    }

    #[test]
    fn queues_xtgettcap_common_terminal_flag_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        let query = format!(
            "\x1bP+q{};{};{};{};{}\x1b\\",
            encode_terminal_hex_bytes(b"AX"),
            encode_terminal_hex_bytes(b"XT"),
            encode_terminal_hex_bytes(b"mir"),
            encode_terminal_hex_bytes(b"msgr"),
            encode_terminal_hex_bytes(b"xenl"),
        );
        buffer.advance(query.as_bytes());

        let expected = format!(
            "\x1bP1+r{}\x1b\\\
             \x1bP1+r{}\x1b\\\
             \x1bP1+r{}\x1b\\\
             \x1bP1+r{}\x1b\\\
             \x1bP1+r{}\x1b\\",
            encode_terminal_hex_bytes(b"AX"),
            encode_terminal_hex_bytes(b"XT"),
            encode_terminal_hex_bytes(b"mir"),
            encode_terminal_hex_bytes(b"msgr"),
            encode_terminal_hex_bytes(b"xenl"),
        )
        .into_bytes();

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), expected);
    }

    #[test]
    fn queues_xtgettcap_line_drawing_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        let query = format!(
            "\x1bP+q{};{};{}\x1b\\",
            encode_terminal_hex_bytes(b"smacs"),
            encode_terminal_hex_bytes(b"rmacs"),
            encode_terminal_hex_bytes(b"acsc"),
        );
        buffer.advance(query.as_bytes());

        let expected = format!(
            "\x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\",
            encode_terminal_hex_bytes(b"smacs"),
            encode_terminal_hex_bytes(b"\x1b(0"),
            encode_terminal_hex_bytes(b"rmacs"),
            encode_terminal_hex_bytes(b"\x1b(B"),
            encode_terminal_hex_bytes(b"acsc"),
            encode_terminal_hex_bytes(b"``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~"),
        )
        .into_bytes();

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), expected);
    }

    #[test]
    fn queues_xtgettcap_rich_text_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        let query = format!(
            "\x1bP+q{};{};{};{};{};{};{};{};{}\x1b\\",
            encode_terminal_hex_bytes(b"Smulx"),
            encode_terminal_hex_bytes(b"Setulc"),
            encode_terminal_hex_bytes(b"Setulc1"),
            encode_terminal_hex_bytes(b"ol"),
            encode_terminal_hex_bytes(b"sitm"),
            encode_terminal_hex_bytes(b"smxx"),
            encode_terminal_hex_bytes(b"Smol"),
            encode_terminal_hex_bytes(b"Rmol"),
            encode_terminal_hex_bytes(b"sgr0"),
        );
        buffer.advance(query.as_bytes());

        let expected = format!(
            "\x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\",
            encode_terminal_hex_bytes(b"Smulx"),
            encode_terminal_hex_bytes(b"\x1b[4::%p1%dm"),
            encode_terminal_hex_bytes(b"Setulc"),
            encode_terminal_hex_bytes(
                b"\x1b[58::2::%p1%{65536}%/%d::%p1%{256}%/%{255}%&%d::%p1%{255}%&%d%;m"
            ),
            encode_terminal_hex_bytes(b"Setulc1"),
            encode_terminal_hex_bytes(b"\x1b[58::5::%p1%dm"),
            encode_terminal_hex_bytes(b"ol"),
            encode_terminal_hex_bytes(b"\x1b[59m"),
            encode_terminal_hex_bytes(b"sitm"),
            encode_terminal_hex_bytes(b"\x1b[3m"),
            encode_terminal_hex_bytes(b"smxx"),
            encode_terminal_hex_bytes(b"\x1b[9m"),
            encode_terminal_hex_bytes(b"Smol"),
            encode_terminal_hex_bytes(b"\x1b[53m"),
            encode_terminal_hex_bytes(b"Rmol"),
            encode_terminal_hex_bytes(b"\x1b[55m"),
            encode_terminal_hex_bytes(b"sgr0"),
            encode_terminal_hex_bytes(b"\x1b(B\x1b[0m"),
        )
        .into_bytes();

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), expected);
    }

    #[test]
    fn queues_xtgettcap_cursor_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        let query = format!(
            "\x1bP+q{};{};{};{}\x1b\\",
            encode_terminal_hex_bytes(b"Ss"),
            encode_terminal_hex_bytes(b"Se"),
            encode_terminal_hex_bytes(b"Cs"),
            encode_terminal_hex_bytes(b"Cr"),
        );
        buffer.advance(query.as_bytes());

        let expected = format!(
            "\x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\",
            encode_terminal_hex_bytes(b"Ss"),
            encode_terminal_hex_bytes(b"\x1b[%p1%d q"),
            encode_terminal_hex_bytes(b"Se"),
            encode_terminal_hex_bytes(b"\x1b[2 q"),
            encode_terminal_hex_bytes(b"Cs"),
            encode_terminal_hex_bytes(b"\x1b]12;%p1%s\x1b\\"),
            encode_terminal_hex_bytes(b"Cr"),
            encode_terminal_hex_bytes(b"\x1b]112\x1b\\"),
        )
        .into_bytes();

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), expected);
    }

    #[test]
    fn queues_xtgettcap_terminal_metadata_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        let query = format!(
            "\x1bP+q{};{};{};{};{}\x1b\\",
            encode_terminal_hex_bytes(b"Hls"),
            encode_terminal_hex_bytes(b"Swd"),
            encode_terminal_hex_bytes(b"Spb"),
            encode_terminal_hex_bytes(b"tsl"),
            encode_terminal_hex_bytes(b"fsl"),
        );
        buffer.advance(query.as_bytes());

        let expected = format!(
            "\x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\\
             \x1bP1+r{}={}\x1b\\",
            encode_terminal_hex_bytes(b"Hls"),
            encode_terminal_hex_bytes(b"\x1b]8;%?%p1%l%tid=%p1%s%;;%p2%s\x1b\\"),
            encode_terminal_hex_bytes(b"Swd"),
            encode_terminal_hex_bytes(b"\x1b]7;"),
            encode_terminal_hex_bytes(b"Spb"),
            encode_terminal_hex_bytes(b"\x1b]9;4;%p1%d;%p2%d\x1b\\"),
            encode_terminal_hex_bytes(b"tsl"),
            encode_terminal_hex_bytes(b"\x1b]0;"),
            encode_terminal_hex_bytes(b"fsl"),
            encode_terminal_hex_bytes(b"\x07"),
        )
        .into_bytes();

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), expected);
    }

    #[test]
    fn queues_xtgettcap_common_partial_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1bP+q544e;436f;636f6c6f7273\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1bP1+r544e=787465726d2d323536636f6c6f72\x1b\\\
              \x1bP1+r436f=323536\x1b\\\
              \x1bP1+r636f6c6f7273=323536\x1b\\"
        );
    }

    #[test]
    fn queues_xtgettcap_invalid_capability_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1bP+q6d697373696e67\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1bP0+r6d697373696e67\x1b\\"
        );
    }

    #[test]
    fn queues_c1_xtgettcap_truecolor_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x90+q524742\x9c");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1bP1+r524742=38\x1b\\");
    }

    #[test]
    fn queues_terminal_device_status_cursor_position_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abc\x1b[6n");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[1;4R");
    }

    #[test]
    fn queues_terminal_dec_cursor_position_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abc\x1b[?6n");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?1;4R");
    }

    #[test]
    fn queues_terminal_dec_cursor_position_response_bytes_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abc\x1b[?2026h\x1b[?6n");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?1;4R");

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_c1_terminal_dec_cursor_position_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abc\x9b?6n");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?1;4R");
    }

    #[test]
    fn queues_terminal_text_area_size_response_bytes_after_resize() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.resize(9, 132);
        buffer.advance(b"\x1b[18t");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[8;9;132t");
    }

    #[test]
    fn queues_terminal_text_area_pixel_size_response_bytes_after_resize() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.resize(9, 132);
        buffer.advance(b"\x1b[14t");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[4;144;1056t");
    }

    #[test]
    fn queues_terminal_screen_pixel_size_response_bytes_after_resize() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.resize(9, 132);
        buffer.advance(b"\x1b[15t");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[5;144;1056t");
    }

    #[test]
    fn queues_terminal_character_cell_size_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[16t");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[6;16;8t");
    }

    #[test]
    fn queues_iterm2_terminal_cell_size_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]1337;ReportCellSize\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]1337;ReportCellSize=16.00;8.00;1.0\x1b\\"
        );
    }

    #[test]
    fn queues_c1_iterm2_terminal_cell_size_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x9d1337;ReportCellSize\x9c");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]1337;ReportCellSize=16.00;8.00;1.0\x1b\\"
        );
    }

    #[test]
    fn queues_iterm2_terminal_feature_report_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]1337;Capabilities\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            format!("\x1b]1337;Capabilities={}\x1b\\", crate::TERMINAL_FEATURE_REPORT).as_bytes()
        );
    }

    #[test]
    fn queues_terminal_screen_char_size_response_bytes_after_resize() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.resize(9, 132);
        buffer.advance(b"\x1b[19t");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[9;9;132t");
    }

    #[test]
    fn queues_synchronized_output_mode_report_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?2026$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?2026;2$y");

        buffer.advance(b"\x1b[?2026h\x1b[?2026$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?2026;1$y");

        buffer.advance(b"\x1b[?2026l\x1b[?2026$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?2026;2$y");
    }

    #[test]
    fn queues_left_right_margin_mode_report_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?69$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?69;2$y");

        buffer.advance(b"\x1b[?69h\x1b[?69$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?69;1$y");

        buffer.advance(b"\x1b[?69l\x1b[?69$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?69;2$y");
    }

    #[test]
    fn queues_split_chunk_synchronized_output_mode_report_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?202");
        assert!(buffer.take_response_bytes().is_empty());

        buffer.advance(b"6$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?2026;2$y");
    }

    #[test]
    fn queues_c1_synchronized_output_mode_report_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x9b?2026h\x9b?2026$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?2026;1$y");

        buffer.advance(b"\x9b?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_common_private_mode_report_responses_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(
            b"\x1b[?1h\x1b[?6h\x1b[?7h\x1b[?12h\x1b[?25l\
              \x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1004h\x1b[?1005h\x1b[?1006h\
              \x1b[?1007l\x1b[?1042h\x1b[?1049h\x1b[?2004h",
        );
        assert!(buffer.take_response_bytes().is_empty());

        buffer.advance(
            b"\x1b[?2026h\
              \x1b[?1$p\x1b[?6$p\x1b[?7$p\x1b[?12$p\x1b[?25$p\
              \x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1005$p\
              \x1b[?1006$p\x1b[?1007$p\x1b[?1042$p\x1b[?1049$p\x1b[?2004$p",
        );
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?1;1$y\x1b[?6;1$y\x1b[?7;1$y\x1b[?12;1$y\x1b[?25;2$y\
              \x1b[?1000;2$y\x1b[?1002;2$y\x1b[?1003;1$y\x1b[?1004;1$y\
              \x1b[?1005;2$y\x1b[?1006;1$y\x1b[?1007;2$y\x1b[?1042;1$y\
              \x1b[?1049;1$y\x1b[?2004;1$y"
        );

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_alternate_screen_variant_mode_report_responses_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?1047h");
        assert!(buffer.take_response_bytes().is_empty());

        buffer.advance(b"\x1b[?2026h\x1b[?47$p\x1b[?1047$p\x1b[?1049$p");
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?47;1$y\x1b[?1047;1$y\x1b[?1049;1$y"
        );

        buffer.advance(b"\x1b[?2026l\x1b[?1047l");
        assert!(buffer.take_response_bytes().is_empty());

        buffer.advance(b"\x1b[?2026h\x1b[?47$p\x1b[?1047$p\x1b[?1049$p");
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?47;2$y\x1b[?1047;2$y\x1b[?1049;2$y"
        );

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_alternate_screen_variant_mode_report_responses_outside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?1047h\x1b[?47$p\x1b[?1047$p\x1b[?1049$p");
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?47;1$y\x1b[?1047;1$y\x1b[?1049;1$y"
        );

        buffer.advance(b"\x1b[?1047l\x1b[?47$p\x1b[?1047$p\x1b[?1049$p");
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?47;2$y\x1b[?1047;2$y\x1b[?1049;2$y"
        );
    }

    #[test]
    fn queues_reset_private_mode_report_responses_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(
            b"\x1b[?1l\x1b[?6l\x1b[?7l\x1b[?12l\x1b[?25h\
              \x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1004l\x1b[?1005l\x1b[?1006l\
              \x1b[?1007h\x1b[?1042l\x1b[?1049l\x1b[?2004l",
        );
        assert!(buffer.take_response_bytes().is_empty());

        buffer.advance(
            b"\x1b[?2026h\
              \x1b[?1$p\x1b[?6$p\x1b[?7$p\x1b[?12$p\x1b[?25$p\
              \x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1005$p\
              \x1b[?1006$p\x1b[?1007$p\x1b[?1042$p\x1b[?1049$p\x1b[?2004$p",
        );
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?1;2$y\x1b[?6;2$y\x1b[?7;2$y\x1b[?12;2$y\x1b[?25;1$y\
              \x1b[?1000;2$y\x1b[?1002;2$y\x1b[?1003;2$y\x1b[?1004;2$y\
              \x1b[?1005;2$y\x1b[?1006;2$y\x1b[?1007;1$y\x1b[?1042;2$y\
              \x1b[?1049;2$y\x1b[?2004;2$y"
        );

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_public_mode_report_responses_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[4h\x1b[20l");
        assert!(buffer.take_response_bytes().is_empty());

        buffer.advance(b"\x1b[?2026h\x1b[4$p\x1b[20$p\x1b[99$p");
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[4;1$y\x1b[20;2$y\x1b[99;0$y"
        );

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_unsupported_private_mode_report_response_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?2026h\x1b[?3$p\x1b[?4242$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?3;0$y\x1b[?4242;0$y");

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_split_chunk_private_mode_report_response_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?2004h\x1b[?2026h\x1b[?20");
        assert!(buffer.take_response_bytes().is_empty());

        buffer.advance(b"04$p");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?2004;1$y");

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_common_terminal_query_responses_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.resize(9, 132);

        buffer.advance(b"abc\x1b[?2026h\x1b[c\x1b[5n\x1b[6n\x1b[18t");
        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b[?6c\x1b[0n\x1b[1;4R\x1b[8;9;132t"
        );

        buffer.advance(b"\x1b[?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_c1_common_terminal_query_responses_inside_synchronized_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"abc\x9b?2026h\x9bc\x9b5n\x9b6n");
        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b[?6c\x1b[0n\x1b[1;4R");

        buffer.advance(b"\x9b?2026l");
        assert!(buffer.take_response_bytes().is_empty());
    }

    #[test]
    fn queues_terminal_dynamic_default_color_query_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]10;rgb:01/02/03\x07\
              \x1b]11;rgb:04/05/06\x07\
              \x1b]12;rgb:07/08/09\x07\
              \x1b]10;?\x07\x1b]11;?\x07\x1b]12;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]10;rgb:0101/0202/0303\x07\
              \x1b]11;rgb:0404/0505/0606\x07\
              \x1b]12;rgb:0707/0808/0909\x07"
        );
    }

    #[test]
    fn queues_terminal_dynamic_default_color_query_with_st_terminator() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]10;rgb:10/20/30\x1b\\\x1b]10;?\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]10;rgb:1010/2020/3030\x1b\\"
        );
    }

    #[test]
    fn queues_terminal_indexed_color_query_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]4;22;rgb:aa/bb/cc\x07\x1b]4;22;?\x07");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;22;rgb:aaaa/bbbb/cccc\x07"
        );
    }

    #[test]
    fn queues_terminal_color_query_response_bytes_after_legacy_hex_color_specs() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]4;22;#aabbcc\x07\
              \x1b]10;#010203\x07\
              \x1b]11;#0a0b0c\x07\
              \x1b]12;#ddeeff\x07\
              \x1b]4;22;?\x07\x1b]10;?\x07\x1b]11;?\x07\x1b]12;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;22;rgb:aaaa/bbbb/cccc\x07\
              \x1b]10;rgb:0101/0202/0303\x07\
              \x1b]11;rgb:0a0a/0b0b/0c0c\x07\
              \x1b]12;rgb:dddd/eeee/ffff\x07"
        );
    }

    #[test]
    fn queues_terminal_color_query_response_bytes_after_named_color_specs() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]4;22;gray50\x07\
              \x1b]10;white\x07\
              \x1b]11;black\x07\
              \x1b]12;Light Blue\x07\
              \x1b]4;22;?\x07\x1b]10;?\x07\x1b]11;?\x07\x1b]12;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;22;rgb:8080/8080/8080\x07\
              \x1b]10;rgb:ffff/ffff/ffff\x07\
              \x1b]11;rgb:0000/0000/0000\x07\
              \x1b]12;rgb:adad/d8d8/e6e6\x07"
        );
    }

    #[test]
    fn queues_terminal_color_query_response_bytes_after_rgba_color_specs() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]4;22;rgba:1212/3434/5656/7878\x07\
              \x1b]10;rgba(10, 11, 12, 0.5)\x07\
              \x1b]4;22;?\x07\x1b]10;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;22;rgb:1212/3434/5656\x07\x1b]10;rgb:0a0a/0b0b/0c0c\x07"
        );
    }

    #[test]
    fn queues_terminal_color_query_response_bytes_after_iterm2_set_colors() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]1337;SetColors=red=00ff00\x07\
              \x1b]1337;SetColors=fg=112233\x07\
              \x1b]4;1;?\x07\x1b]10;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;1;rgb:0000/ffff/0000\x07\x1b]10;rgb:1111/2222/3333\x07"
        );
    }

    #[test]
    fn queues_kitty_color_control_query_response_bytes_after_palette_updates() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]21;foreground=#112233;1=#00ff00\x1b\\\
              \x1b]21;foreground=?;1=?;selection_background=?\x1b\\",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]21;foreground=rgb:1111/2222/3333\x1b\\\
              \x1b]21;1=rgb:0000/ffff/0000\x1b\\\
              \x1b]21;selection_background=?\x1b\\"
        );
    }

    #[test]
    fn queues_terminal_color_query_response_bytes_after_legacy_linux_console_palette_updates() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]P1aabbcc\x07\x1b]4;1;?\x07\x1b]R\x07\x1b]4;1;?\x07");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;1;rgb:aaaa/bbbb/cccc\x07\x1b]4;1;rgb:efef/4444/4444\x07"
        );
    }

    #[test]
    fn queues_terminal_extended_color_slot_query_response_bytes_after_osc4_update() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]4;256;rgb:01/02/03;260;rgb:04/05/06\x07\
              \x1b]4;256;?\x07\x1b]4;260;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;256;rgb:0101/0202/0303\x07\
              \x1b]4;260;rgb:0404/0505/0606\x07"
        );
    }

    #[test]
    fn queues_terminal_iterm2_osc4_default_alias_query_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]4;-1;rgb:01/02/03;-2;rgb:04/05/06\x07\
              \x1b]4;-1;?\x07\x1b]4;-2;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;-1;rgb:0101/0202/0303\x07\
              \x1b]4;-2;rgb:0404/0505/0606\x07"
        );
    }

    #[test]
    fn queues_terminal_extended_color_slot_query_with_st_terminator() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]4;256;rgb:10/20/30\x1b\\\x1b]4;256;?\x1b\\");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;256;rgb:1010/2020/3030\x1b\\"
        );
    }

    #[test]
    fn queues_terminal_extended_color_slot_query_after_osc104_reset() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]4;256;rgb:01/02/03\x07\x1b]104;256\x07\x1b]4;256;?\x07");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;256;rgb:e8e8/eded/f6f6\x07"
        );
    }

    #[test]
    fn queues_terminal_extended_color_slot_query_after_osc104_reset_all() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]4;256;rgb:01/02/03\x07\x1b]104\x07\x1b]4;256;?\x07");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;256;rgb:e8e8/eded/f6f6\x07"
        );
    }

    #[test]
    fn queues_terminal_fallback_color_query_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]4;1;?\x07\x1b]4;22;?\x07\x1b]10;?\x07\x1b]11;?\x07\x1b]12;?\x07");

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;1;rgb:efef/4444/4444\x07\
              \x1b]4;22;rgb:0000/5f5f/0000\x07\
              \x1b]10;rgb:e8e8/eded/f6f6\x07\
              \x1b]11;rgb:0505/0707/0b0b\x07\
              \x1b]12;rgb:7d7d/d3d3/fcfc\x07"
        );
    }

    #[test]
    fn queues_terminal_color_query_after_reset_with_default_response() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(
            b"\x1b]4;22;rgb:aa/bb/cc\x07\x1b]10;rgb:01/02/03\x07\
              \x1b]104;22\x07\x1b]110\x07\x1b]4;22;?\x07\x1b]10;?\x07",
        );

        assert_eq!(
            join_response_bytes(buffer.take_response_bytes()),
            b"\x1b]4;22;rgb:0000/5f5f/0000\x07\x1b]10;rgb:e8e8/eded/f6f6\x07"
        );
    }

    #[test]
    fn detects_terminal_media_sequences_without_storing_payloads() {
        assert_eq!(
            detect_terminal_media_kind(b"\x1b_Ga=T,f=100;AAAA\x1b\\"),
            Some(ScreenLineMediaKind::KittyGraphics)
        );
        assert_eq!(
            detect_terminal_media_kind(b"\x9fGa=T,f=100;AAAA\x9c"),
            Some(ScreenLineMediaKind::KittyGraphics)
        );
        assert_eq!(
            detect_terminal_media_kind(b"\x1b]1337;File=name=test.png:inline=1:AAAA\x07"),
            Some(ScreenLineMediaKind::Iterm2Image)
        );
        assert_eq!(
            detect_terminal_media_kind(b"\x9d1337;File=name=test.png:inline=1:AAAA\x9c"),
            Some(ScreenLineMediaKind::Iterm2Image)
        );
        assert_eq!(
            detect_terminal_media_kind(
                b"\x1b]1337;MultipartFile=name=test.png:inline=1\x07\
                  \x1b]1337;FilePart=AAAA\x07\
                  \x1b]1337;FileEnd\x07"
            ),
            Some(ScreenLineMediaKind::Iterm2Image)
        );
        assert_eq!(
            detect_terminal_media_kind(b"\x1bPq#0;2;0;0;0~~\x1b\\"),
            Some(ScreenLineMediaKind::Sixel)
        );
        assert_eq!(
            detect_terminal_media_kind(b"\x90q#0;2;0;0;0~~\x9c"),
            Some(ScreenLineMediaKind::Sixel)
        );
        assert_eq!(detect_terminal_media_kind(b"\x1b]0;title\x07"), None);
    }

    #[test]
    fn marks_iterm2_inline_image_output_with_safe_data_payload() {
        let buffer = EmulatorBuffer::new(4, 120);
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";

        buffer.advance(
            format!(
                "before \x1b]1337;File=name=dGlueS5wbmc=;size=68;width=2;height=1;preserveAspectRatio=1;inline=1:{PNG_1X1}\x07 after"
            )
            .as_bytes(),
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around iTerm2 image output should render");

        assert_eq!(
            line.media,
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
        assert!(
            !line.text.contains(PNG_1X1),
            "iTerm2 image payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn marks_iterm2_multipart_inline_image_output_with_safe_data_payload() {
        let buffer = EmulatorBuffer::new(4, 120);
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
        let split_at = PNG_1X1.len() / 2;

        buffer.advance(
            format!(
                "before \x1b]1337;MultipartFile=name=dGlueS5wbmc=;size=68;width=2;height=1;preserveAspectRatio=1;inline=1\x07\
                 \x1b]1337;FilePart={}\x07\
                 \x1b]1337;FilePart={}\x07\
                 \x1b]1337;FileEnd\x07 after",
                &PNG_1X1[..split_at],
                &PNG_1X1[split_at..],
            )
            .as_bytes(),
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around multipart iTerm2 image output should render");

        assert_eq!(line.media.len(), 1);
        assert_eq!(line.media[0].kind, ScreenLineMediaKind::Iterm2Image);
        assert_eq!(line.media[0].name.as_deref(), Some("tiny.png"));
        assert_eq!(line.media[0].data_base64.as_deref(), Some(PNG_1X1));
        assert!(
            !line.text.contains(PNG_1X1),
            "multipart iTerm2 image payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn marks_tmux_wrapped_iterm2_inline_image_output_with_safe_data_payload() {
        let buffer = EmulatorBuffer::new(4, 120);
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";

        buffer.advance(
            format!(
                "before \x1bPtmux;\x1b\x1b]1337;File=name=dGlueS5wbmc=;size=68;width=2;height=1;preserveAspectRatio=1;inline=1:{PNG_1X1}\x07\x1b\\ after"
            )
            .as_bytes(),
        );

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around tmux wrapped iTerm2 image output should render");

        assert_eq!(
            line.media,
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
        assert!(
            !line.text.contains(PNG_1X1),
            "tmux wrapped iTerm2 image payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn marks_kitty_graphics_output_as_terminal_media() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"before \x1b_Ga=T,f=100;AAAA\x1b\\ after");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around kitty graphics output should render");

        assert_eq!(line.media, vec![ScreenLineMedia::marker(ScreenLineMediaKind::KittyGraphics)]);
        assert!(
            !line.text.contains("AAAA"),
            "terminal media payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn marks_kitty_png_graphics_output_with_safe_data_payload() {
        let buffer = EmulatorBuffer::new(4, 120);
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";

        buffer.advance(format!("before \x1b_Ga=T,f=100,c=4,r=2;{PNG_1X1}\x1b\\ after").as_bytes());

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around kitty graphics output should render");

        assert_eq!(
            line.media,
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
        assert!(
            !line.text.contains(PNG_1X1),
            "kitty graphics payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn assembles_chunked_kitty_png_graphics_output_with_safe_data_payload() {
        let buffer = EmulatorBuffer::new(4, 120);
        const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";
        let (first_chunk, second_chunk) = PNG_1X1.split_at(44);

        buffer
            .advance(format!("before \x1b_Ga=T,f=100,c=4,r=2,m=1;{first_chunk}\x1b\\").as_bytes());
        buffer.advance(format!("\x1b_Gm=0;{second_chunk}\x1b\\").as_bytes());
        buffer.advance(b"\x1b_Ga=T,f=24,s=1,v=1;/wAA\x1b\\");

        let media = buffer
            .render(None)
            .surface
            .lines
            .iter()
            .flat_map(|line| line.media.iter())
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(
            media,
            vec![
                ScreenLineMedia {
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
                },
                ScreenLineMedia::marker(ScreenLineMediaKind::KittyGraphics),
            ]
        );
    }

    #[test]
    fn queues_kitty_graphics_query_response_bytes_without_rendering_media() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(b"before \x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\ after");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b_Gi=31;OK\x1b\\");
        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around kitty graphics query should render");

        assert_eq!(line.text, "before  after");
        assert!(line.media.is_empty());
        assert!(
            !line.text.contains("AAAA"),
            "kitty graphics query payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn queues_c1_kitty_graphics_query_response_bytes_without_rendering_media() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(b"before \x9fGi=32,s=1,v=1,a=q,t=d,f=32;AAAAAA==\x9c after");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b_Gi=32;OK\x1b\\");
        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around C1 kitty graphics query should render");

        assert_eq!(line.text, "before  after");
        assert!(line.media.is_empty());
    }

    #[test]
    fn queues_unsupported_kitty_graphics_query_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(b"\x1b_Gi=33,a=q,t=f;L3RtcC9pbWcucG5n\x1b\\");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b_Gi=33;ENOTSUP\x1b\\");
        assert!(buffer.render(None).surface.lines.iter().all(|line| line.media.is_empty()));
    }

    #[test]
    fn queues_invalid_kitty_graphics_query_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(b"\x1b_Gi=35,s=1,v=1,a=q,t=d,f=24;not-base64!\x1b\\");

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b_Gi=35;ENOTSUP\x1b\\");
    }

    #[test]
    fn queues_tmux_wrapped_kitty_graphics_query_response_bytes() {
        let buffer = EmulatorBuffer::new(4, 120);

        buffer.advance(
            b"before \x1bPtmux;\x1b\x1b_Gi=34,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\x1b\\\x1b\\ after",
        );

        assert_eq!(join_response_bytes(buffer.take_response_bytes()), b"\x1b_Gi=34;OK\x1b\\");
        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around tmux-wrapped kitty graphics query should render");

        assert_eq!(line.text, "before  after");
        assert!(line.media.is_empty());
    }

    #[test]
    fn marks_sixel_graphics_output_as_terminal_media() {
        let buffer = EmulatorBuffer::new(4, 120);
        let sixel = b"#0;2;0;0;0#1;2;100;100;0#1~~@@vv@@~~@@~~";
        buffer.advance(b"before \x1bPq");
        buffer.advance(sixel);
        buffer.advance(b"\x1b\\ after");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around sixel output should render");

        assert_eq!(line.media, vec![ScreenLineMedia::marker(ScreenLineMediaKind::Sixel)]);
        assert!(
            !line.text.contains("#1~~@@"),
            "sixel payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn marks_tmux_wrapped_sixel_graphics_output_as_terminal_media() {
        let buffer = EmulatorBuffer::new(4, 120);
        let sixel = b"#0;2;0;0;0#1;2;100;100;0#1~~@@vv@@~~@@~~";
        buffer.advance(b"before \x1bPtmux;\x1b\x1bPq");
        buffer.advance(sixel);
        buffer.advance(b"\x1b\x1b\\\x1b\\ after");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around tmux wrapped sixel output should render");

        assert_eq!(line.media, vec![ScreenLineMedia::marker(ScreenLineMediaKind::Sixel)]);
        assert!(
            !line.text.contains("#1~~@@"),
            "tmux wrapped sixel payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn marks_osc52_clipboard_write_as_blocked_side_effect() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]52;c;SGVsbG8=\x07ready");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("ready"))
            .expect("line around OSC 52 output should render");

        assert_eq!(
            line.side_effects,
            vec![ScreenLineSideEffect {
                kind: ScreenLineSideEffectKind::ClipboardWrite,
                disposition: ScreenLineSideEffectDisposition::Blocked,
                target: Some(ScreenLineSideEffectTarget::Clipboard),
                message: None,
            }]
        );
        assert!(
            !line.text.contains("SGVsbG8"),
            "OSC 52 clipboard payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn marks_osc52_clipboard_read_as_blocked_side_effect() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b]52;s;?\x07ready");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("ready"))
            .expect("line around OSC 52 output should render");

        assert_eq!(
            line.side_effects,
            vec![ScreenLineSideEffect {
                kind: ScreenLineSideEffectKind::ClipboardRead,
                disposition: ScreenLineSideEffectDisposition::Blocked,
                target: Some(ScreenLineSideEffectTarget::Selection),
                message: None,
            }]
        );
    }

    #[test]
    fn marks_osc9_desktop_notification_as_blocked_side_effect() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"\x1b]9;Build finished\x07ready");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("ready"))
            .expect("line around OSC 9 output should render");

        assert_eq!(
            line.side_effects,
            vec![ScreenLineSideEffect {
                kind: ScreenLineSideEffectKind::DesktopNotification,
                disposition: ScreenLineSideEffectDisposition::Blocked,
                target: Some(ScreenLineSideEffectTarget::DesktopNotification),
                message: Some("Build finished".to_string()),
            }]
        );
        assert!(
            !line.text.contains("Build finished"),
            "OSC 9 notification payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn strips_privacy_control_strings_without_leaking_payloads() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"before \x1bXsecret\x1b\\ middle \x1b^private\x1b\\ after");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around privacy strings should render");

        assert_eq!(line.text, "before  middle  after");
        assert!(
            !line.text.contains("secret") && !line.text.contains("private"),
            "privacy string payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn strips_raw_c1_privacy_control_strings_without_leaking_payloads() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"before \x98secret\x9c middle \x9eprivate\x9c after");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around raw C1 privacy strings should render");

        assert_eq!(line.text, "before  middle  after");
        assert!(
            !line.text.contains("secret") && !line.text.contains("private"),
            "raw C1 privacy string payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn strips_unknown_apc_control_strings_without_leaking_payloads() {
        let buffer = EmulatorBuffer::new(4, 120);
        buffer.advance(b"before \x1b_not-kitty-secret\x1b\\ middle \x9fraw-secret\x9c after");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around APC strings should render");

        assert_eq!(line.text, "before  middle  after");
        assert!(
            !line.text.contains("secret"),
            "APC payload should not leak into visible text: {:?}",
            line
        );
    }

    #[test]
    fn preserves_raw_c1_index_and_next_line_controls() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"alpha\x84beta\x85gamma");

        let lines =
            buffer.render(None).surface.lines.into_iter().map(|line| line.text).collect::<Vec<_>>();

        assert_eq!(lines[0], "alpha");
        assert_eq!(lines[1], "     beta");
        assert_eq!(lines[2], "gamma");
    }

    #[test]
    fn preserves_raw_c1_single_shift_controls_without_visible_garbage() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"before \x8eA middle \x8fB after");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around raw C1 single-shift controls should render");

        assert_eq!(line.text, "before A middle B after");
    }

    #[test]
    fn preserves_raw_c1_guard_and_terminal_id_controls_without_visible_garbage() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"before \x96guarded\x97 \x9aafter");

        let surface = buffer.render(None).surface;
        let line = surface
            .lines
            .iter()
            .find(|line| line.text.contains("before"))
            .expect("line around raw C1 guard controls should render");

        assert_eq!(line.text, "before guarded after");
    }

    #[test]
    fn native_decsca_selective_line_erase_preserves_protected_cells_and_style() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"ab\x1b[31m\x1b[1\"qCD\x1b[0\"q\x1b[0mef\r\x1b[?KZ");

        let surface = buffer.render(None).surface;
        let line = &surface.lines[0];

        assert_eq!(line.text, "Z CD");
        assert!(
            line.spans.iter().any(|span| {
                span.text == "CD"
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            }),
            "DECSEL must keep protected cell style with text: {:?}",
            line.spans
        );
    }

    #[test]
    fn native_spa_epa_selective_line_erase_preserves_raw_c1_protected_cells() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"aa\x96BB\x97cc\r\x9b?KZ");

        let line = &buffer.render(None).surface.lines[0];

        assert_eq!(line.text, "Z BB");
    }

    #[test]
    fn native_decsed_selective_display_erase_preserves_protected_cells() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"aa\x1b[1\"qBB\x1b[0\"qcc\x1b[H\x1b[?JZ");

        let line = &buffer.render(None).surface.lines[0];

        assert_eq!(line.text, "Z BB");
    }

    #[test]
    fn native_unprotected_overwrite_removes_previous_protected_cell_snapshot() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[1\"qAB\x1b[0\"q\rxy\r\x1b[?KZ");

        let line = &buffer.render(None).surface.lines[0];

        assert_eq!(line.text, "Z");
    }

    #[test]
    fn native_decfra_fills_rectangular_area_with_current_style_and_preserves_cursor() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[1;1Hab\x1b[2;1Hcd\x1b[31m\x1b[88;1;1;2;2$xZ\x1b[0m");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "XX");
        assert_eq!(lines[1].text, "XXZ");
        assert!(lines.iter().take(2).all(|line| {
            line.spans.iter().any(|span| {
                span.text.contains('X')
                    && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
            })
        }));
    }

    #[test]
    fn native_decfra_ignores_insert_mode_for_rectangle_cells() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[4habcdef\x1b[88;1;2;1;4$x");

        let line = &buffer.render(None).surface.lines[0];

        assert_eq!(line.text, "aXXXef");
    }

    #[test]
    fn native_decfra_preserves_cursor_when_origin_mode_is_enabled() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[2;4r\x1b[?6habc\x1b[88;1;1;1;2$xZ");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "XX");
        assert_eq!(lines[1].text, "abcZ");
    }

    #[test]
    fn native_decera_erases_rectangular_area_to_blank_cells() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[1;1Habcdef\x1b[2;1Huvwxyz\x1b[1;2;2;4$z");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "a   ef");
        assert_eq!(lines[1].text, "u   yz");
    }

    #[test]
    fn native_decera_uses_current_background_color_for_blank_cells() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"\x1b[1;1Habcdef\x1b[2;1Huvwxyz\x1b[48;2;9;8;7m\x1b[1;2;2;4$z");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "a   ef");
        assert_eq!(lines[1].text, "u   yz");
        assert!(lines.iter().filter(|line| !line.text.is_empty()).all(|line| {
            line.spans.iter().any(|span| {
                span.text == "   "
                    && span.style.background == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
            })
        }));
    }

    #[test]
    fn native_decsera_preserves_protected_cells_in_rectangular_area() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abcdef\x1b[1;3H\x1b[1\"qCD\x1b[0\"q\x1b[1;1;1;6${");

        let line = &buffer.render(None).surface.lines[0];

        assert_eq!(line.text, "  CD");
    }

    #[test]
    fn native_deccra_copies_rectangular_area_with_rich_styles_without_moving_cursor() {
        let buffer = EmulatorBuffer::new(5, 80);
        buffer.advance(
            b"\x1b[31;48;2;1;2;3mAB\x1b[0mcd\r\n\x1b[4:3;58;2;9;8;7mEF\x1b[0mgh\x1b[1;1;2;2;1;1;3;1$vZ",
        );

        let lines = buffer.render(None).surface.lines;

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
    fn native_deccra_uses_snapshot_when_source_and_destination_overlap() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abcdef\x1b[1;1;1;4;1;1;3;1$v");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "ababcd");
    }

    #[test]
    fn native_deccra_preserves_protected_attributes_for_later_selective_erase() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abcdef\x1b[1;2H\x1b[1\"qBC\x1b[0\"q\x1b[1;2;1;3;1;1;5;1$v\x1b[1;1;1;6${");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, " BC BC");
    }

    #[test]
    fn native_decic_inserts_columns_inside_viewport_width() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"abcdef\r\n123456\x1b[1;3H\x1b[2'}");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "ab  cd");
        assert_eq!(lines[1].text, "12  34");
    }

    #[test]
    fn native_decdc_deletes_columns_inside_viewport_width() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"abcdef\r\n123456\x1b[1;3H\x1b[2'~");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "abef");
        assert_eq!(lines[1].text, "1256");
    }

    #[test]
    fn native_decic_and_decdc_preserve_cursor_position() {
        let inserted = EmulatorBuffer::new(2, 6);
        inserted.advance(b"abcdef\x1b[1;3H\x1b[2'}");

        let deleted = EmulatorBuffer::new(2, 6);
        deleted.advance(b"abcdef\x1b[1;3H\x1b[2'~");

        assert_eq!(
            inserted.render(None).surface.cursor.map(|cursor| (cursor.row, cursor.col)),
            Some((0, 2))
        );
        assert_eq!(
            deleted.render(None).surface.cursor.map(|cursor| (cursor.row, cursor.col)),
            Some((0, 2))
        );
    }

    #[test]
    fn native_decic_and_decdc_only_affect_rows_inside_scroll_margins() {
        let inserted = EmulatorBuffer::new(4, 6);
        inserted.advance(b"one111\r\ntwo222\r\nthr333\r\nfor444\x1b[2;3r\x1b[2;4H\x1b[2'}");

        let deleted = EmulatorBuffer::new(4, 6);
        deleted.advance(b"one111\r\ntwo222\r\nthr333\r\nfor444\x1b[2;3r\x1b[2;4H\x1b[2'~");

        let inserted_lines = inserted.render(None).surface.lines;
        let deleted_lines = deleted.render(None).surface.lines;

        assert_eq!(inserted_lines[0].text, "one111");
        assert_eq!(inserted_lines[1].text, "two  2");
        assert_eq!(inserted_lines[2].text, "thr  3");
        assert_eq!(inserted_lines[3].text, "for444");
        assert_eq!(deleted_lines[0].text, "one111");
        assert_eq!(deleted_lines[1].text, "two2");
        assert_eq!(deleted_lines[2].text, "thr3");
        assert_eq!(deleted_lines[3].text, "for444");
    }

    #[test]
    fn native_decdc_moves_protected_cells_with_shifted_content() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"\x1b[1\"qabcdef\x1b[0\"q\x1b[1;3H\x1b[2'~\x1b[1;1H\x1b[?2K");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "abef");
    }

    #[test]
    fn native_sl_scrolls_left_inside_viewport_width() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"abcdef\r\n123456\x1b[2 @");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "cdef");
        assert_eq!(lines[1].text, "3456");
    }

    #[test]
    fn native_sr_scrolls_right_inside_viewport_width() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"abcdef\r\n123456\x1b[2 A");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "  abcd");
        assert_eq!(lines[1].text, "  1234");
    }

    #[test]
    fn native_sr_uses_current_background_color_for_inserted_blank_cells() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"abcdef\x1b[48;2;9;8;7m\x1b[2 A");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "  abcd");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "  "
                && span.style.background == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn native_decbi_and_decfi_move_cursor_inside_viewport_width() {
        let back = EmulatorBuffer::new(2, 6);
        back.advance(b"abc\x1b6Z");

        let forward = EmulatorBuffer::new(2, 6);
        forward.advance(b"abc\x1b[1;2H\x1b9Z");

        assert_eq!(back.render(None).surface.lines[0].text, "abZ");
        assert_eq!(forward.render(None).surface.lines[0].text, "abZ");
    }

    #[test]
    fn native_decbi_and_decfi_shift_content_at_viewport_edges() {
        let back = EmulatorBuffer::new(2, 6);
        back.advance(b"abcdef\r\n123456\x1b[1;1H\x1b6");

        let forward = EmulatorBuffer::new(2, 6);
        forward.advance(b"abcdef\r\n123456\x1b[1;6H\x1b9");

        let back_lines = back.render(None).surface.lines;
        let forward_lines = forward.render(None).surface.lines;

        assert_eq!(back_lines[0].text, " abcde");
        assert_eq!(back_lines[1].text, " 12345");
        assert_eq!(forward_lines[0].text, "bcdef");
        assert_eq!(forward_lines[1].text, "23456");
    }

    #[test]
    fn native_decbi_and_decfi_preserve_cursor_position_when_shifting_content() {
        let back = EmulatorBuffer::new(2, 6);
        back.advance(b"abcdef\x1b[1;1H\x1b6");

        let forward = EmulatorBuffer::new(2, 6);
        forward.advance(b"abcdef\x1b[1;6H\x1b9");

        assert_eq!(
            back.render(None).surface.cursor.map(|cursor| (cursor.row, cursor.col)),
            Some((0, 0))
        );
        assert_eq!(
            forward.render(None).surface.cursor.map(|cursor| (cursor.row, cursor.col)),
            Some((0, 5))
        );
    }

    #[test]
    fn native_decbi_uses_current_background_color_for_inserted_blank_cells() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"abcdef\x1b[48;2;9;8;7m\x1b[1;1H\x1b6");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, " abcde");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == " " && span.style.background == Some(ScreenColor::Rgb { r: 9, g: 8, b: 7 })
        }));
    }

    #[test]
    fn native_left_right_margins_limit_scroll_left_and_right_columns() {
        let left = EmulatorBuffer::new(2, 6);
        left.advance(b"abcdef\r\n123456\x1b[?69h\x1b[2;5s\x1b[1 @");

        let right = EmulatorBuffer::new(2, 6);
        right.advance(b"abcdef\r\n123456\x1b[?69h\x1b[2;5s\x1b[1 A");

        let left_lines = left.render(None).surface.lines;
        let right_lines = right.render(None).surface.lines;

        assert_eq!(left_lines[0].text, "acde f");
        assert_eq!(left_lines[1].text, "1345 6");
        assert_eq!(right_lines[0].text, "a bcdf");
        assert_eq!(right_lines[1].text, "1 2346");
    }

    #[test]
    fn native_left_right_margins_limit_insert_and_delete_columns() {
        let inserted = EmulatorBuffer::new(2, 6);
        inserted.advance(b"abcdef\r\n123456\x1b[?69h\x1b[2;5s\x1b[1;3H\x1b[1'}");

        let deleted = EmulatorBuffer::new(2, 6);
        deleted.advance(b"abcdef\r\n123456\x1b[?69h\x1b[2;5s\x1b[1;3H\x1b[1'~");

        let inserted_lines = inserted.render(None).surface.lines;
        let deleted_lines = deleted.render(None).surface.lines;

        assert_eq!(inserted_lines[0].text, "ab cdf");
        assert_eq!(inserted_lines[1].text, "12 346");
        assert_eq!(deleted_lines[0].text, "abde f");
        assert_eq!(deleted_lines[1].text, "1245 6");
    }

    #[test]
    fn native_left_right_margins_limit_back_and_forward_index() {
        let back = EmulatorBuffer::new(2, 6);
        back.advance(b"abcdef\r\n123456\x1b[?69h\x1b[2;5s\x1b[1;2H\x1b6");

        let forward = EmulatorBuffer::new(2, 6);
        forward.advance(b"abcdef\r\n123456\x1b[?69h\x1b[2;5s\x1b[1;5H\x1b9");

        let back_lines = back.render(None).surface.lines;
        let forward_lines = forward.render(None).surface.lines;

        assert_eq!(back_lines[0].text, "a bcdf");
        assert_eq!(back_lines[1].text, "1 2346");
        assert_eq!(forward_lines[0].text, "acde f");
        assert_eq!(forward_lines[1].text, "1345 6");
    }

    #[test]
    fn native_resetting_left_right_margin_mode_restores_full_width_column_operations() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"abcdef\r\n123456\x1b[?69h\x1b[2;5s\x1b[?69l\x1b[1 @");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "bcdef");
        assert_eq!(lines[1].text, "23456");
    }

    #[test]
    fn native_sl_and_sr_preserve_cursor_position() {
        let left = EmulatorBuffer::new(2, 6);
        left.advance(b"abcdef\x1b[1;3H\x1b[2 @");

        let right = EmulatorBuffer::new(2, 6);
        right.advance(b"abcdef\x1b[1;3H\x1b[2 A");

        assert_eq!(
            left.render(None).surface.cursor.map(|cursor| (cursor.row, cursor.col)),
            Some((0, 2))
        );
        assert_eq!(
            right.render(None).surface.cursor.map(|cursor| (cursor.row, cursor.col)),
            Some((0, 2))
        );
    }

    #[test]
    fn native_sl_and_sr_only_affect_rows_inside_scroll_margins() {
        let left = EmulatorBuffer::new(4, 6);
        left.advance(b"one111\r\ntwo222\r\nthr333\r\nfor444\x1b[2;3r\x1b[2 @");

        let right = EmulatorBuffer::new(4, 6);
        right.advance(b"one111\r\ntwo222\r\nthr333\r\nfor444\x1b[2;3r\x1b[2 A");

        let left_lines = left.render(None).surface.lines;
        let right_lines = right.render(None).surface.lines;

        assert_eq!(left_lines[0].text, "one111");
        assert_eq!(left_lines[1].text, "o222");
        assert_eq!(left_lines[2].text, "r333");
        assert_eq!(left_lines[3].text, "for444");
        assert_eq!(right_lines[0].text, "one111");
        assert_eq!(right_lines[1].text, "  two2");
        assert_eq!(right_lines[2].text, "  thr3");
        assert_eq!(right_lines[3].text, "for444");
    }

    #[test]
    fn native_sl_moves_protected_cells_with_shifted_content() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"\x1b[1\"qabcdef\x1b[0\"q\x1b[2 @\x1b[1;1H\x1b[?2K");

        let lines = buffer.render(None).surface.lines;

        assert_eq!(lines[0].text, "cdef");
    }

    #[test]
    fn native_sl_and_sr_preserve_extra_styles_with_shifted_content() {
        let left = EmulatorBuffer::new(2, 6);
        left.advance(b"\x1b[31mabcdef\x1b[0m\x1b[2 @");

        let right = EmulatorBuffer::new(2, 6);
        right.advance(b"\x1b[31mabcdef\x1b[0m\x1b[2 A");

        let left_line = &left.render(None).surface.lines[0];
        let right_line = &right.render(None).surface.lines[0];

        assert!(left_line.spans.iter().any(|span| {
            span.text == "cdef"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert!(right_line.spans.iter().any(|span| {
            span.text == "abcd"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn native_decic_preserves_extra_styles_with_shifted_content() {
        let buffer = EmulatorBuffer::new(2, 6);
        buffer.advance(b"\x1b[31mabcdef\x1b[0m\x1b[1;3H\x1b[2'}");

        let line = &buffer.render(None).surface.lines[0];

        assert!(line.spans.iter().any(|span| {
            span.text == "ab"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
        assert!(line.spans.iter().any(|span| {
            span.text == "cd"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }));
    }

    #[test]
    fn reports_alternate_screen_buffer_kind_from_vt_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"normal");

        let normal = buffer.render(None);
        assert_eq!(normal.buffer_kind, ScreenBufferKind::Normal);
        assert!(
            normal.surface.lines.iter().any(|line| line.text.contains("normal")),
            "normal screen should contain normal output: {:?}",
            normal.surface.lines
        );

        buffer.advance(b"\x1b[?1049halt");
        let alternate = buffer.render(None);
        assert_eq!(alternate.buffer_kind, ScreenBufferKind::Alternate);
        assert!(
            alternate.surface.lines.iter().any(|line| line.text.contains("alt")),
            "alternate screen should contain alternate output: {:?}",
            alternate.surface.lines
        );
        assert!(
            !alternate.surface.lines.iter().any(|line| line.text.contains("normal")),
            "alternate screen should not expose normal buffer text: {:?}",
            alternate.surface.lines
        );

        buffer.advance(b"\x1b[?1049lback");
        let restored = buffer.render(None);
        assert_eq!(restored.buffer_kind, ScreenBufferKind::Normal);
        assert!(
            restored.surface.lines.iter().any(|line| line.text.contains("normalback")),
            "leaving alternate screen should restore normal buffer cursor/content: {:?}",
            restored.surface.lines
        );
    }

    #[test]
    fn reports_legacy_private_47_alternate_screen_buffer_kind_from_vt_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"normal");

        buffer.advance(b"\x1b[?47hlegacy");
        let alternate = buffer.render(None);
        assert_eq!(alternate.buffer_kind, ScreenBufferKind::Alternate);
        assert!(
            alternate.surface.lines.iter().any(|line| line.text.contains("legacy")),
            "legacy alternate screen should contain alternate output: {:?}",
            alternate.surface.lines
        );
        assert!(
            !alternate.surface.lines.iter().any(|line| line.text.contains("normal")),
            "legacy alternate screen should not expose normal output: {:?}",
            alternate.surface.lines
        );

        buffer.advance(b"\x1b[?47lback");
        let restored = buffer.render(None);
        assert_eq!(restored.buffer_kind, ScreenBufferKind::Normal);
        assert!(
            restored.surface.lines.iter().any(|line| line.text.contains("normalback")),
            "leaving legacy alternate screen should restore normal output: {:?}",
            restored.surface.lines
        );
    }

    #[test]
    fn reports_private_1047_alternate_screen_buffer_kind_from_vt_output() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"normal");

        buffer.advance(b"\x1b[?1047halt");
        let alternate = buffer.render(None);
        assert_eq!(alternate.buffer_kind, ScreenBufferKind::Alternate);
        assert!(
            alternate.surface.lines.iter().any(|line| line.text.contains("alt")),
            "1047 alternate screen should contain alternate output: {:?}",
            alternate.surface.lines
        );
        assert!(
            !alternate.surface.lines.iter().any(|line| line.text.contains("normal")),
            "1047 alternate screen should not expose normal output: {:?}",
            alternate.surface.lines
        );

        buffer.advance(b"\x1b[?1047lback");
        let restored = buffer.render(None);
        assert_eq!(restored.buffer_kind, ScreenBufferKind::Normal);
        assert!(
            restored.surface.lines.iter().any(|line| line.text.contains("normalback")),
            "leaving 1047 alternate screen should restore normal output: {:?}",
            restored.surface.lines
        );
    }

    #[test]
    fn private_1048_saves_and_restores_cursor_without_switching_buffer() {
        let buffer = EmulatorBuffer::new(4, 80);
        buffer.advance(b"abc\x1b[?1048hXYZ\x1b[?1048lQ");

        let rendered = buffer.render(None);
        assert_eq!(rendered.buffer_kind, ScreenBufferKind::Normal);
        assert!(
            rendered.surface.lines.iter().any(|line| line.text.contains("abcQYZ")),
            "1048 should restore cursor in the normal buffer: {:?}",
            rendered.surface.lines
        );
    }

    #[test]
    fn reports_private_12_cursor_blink_mode() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b[?12habc");
        let blinking = buffer.render(None).surface.cursor.expect("cursor should render");
        assert!(blinking.blinking, "DECSET ?12 should enable cursor blinking: {:?}", blinking);

        buffer.advance(b"\x1b[?12l");
        let steady = buffer.render(None).surface.cursor.expect("cursor should render");
        assert!(!steady.blinking, "DECRST ?12 should disable cursor blinking: {:?}", steady);
    }

    #[test]
    fn tracks_iterm2_cursor_shape_from_osc1337_output() {
        let buffer = EmulatorBuffer::new(4, 80);

        buffer.advance(b"\x1b]1337;CursorShape=1\x07bar");
        let beam = buffer.render(None).surface.cursor.expect("cursor should render");
        assert_eq!(beam.shape, Some(ScreenCursorShape::Beam));
        assert!(!beam.blinking, "OSC 1337 CursorShape should not force blinking: {:?}", beam);

        buffer.advance(b"\x1b]1337;CursorShape=2\x07under");
        let underline = buffer.render(None).surface.cursor.expect("cursor should render");
        assert_eq!(underline.shape, Some(ScreenCursorShape::Underline));

        buffer.advance(b"\x1b]1337;CursorShape=99\x07ignored");
        let ignored = buffer.render(None).surface.cursor.expect("cursor should render");
        assert_eq!(ignored.shape, Some(ScreenCursorShape::Underline));
    }

    #[test]
    fn tracks_bracketed_paste_private_mode() {
        let buffer = EmulatorBuffer::new(4, 80);

        assert!(!buffer.bracketed_paste_enabled());

        buffer.advance(b"\x1b[?2004h");
        assert!(buffer.bracketed_paste_enabled());

        buffer.advance(b"\x1b[?2004l");
        assert!(!buffer.bracketed_paste_enabled());
    }

    #[test]
    fn preserves_soft_wrap_metadata_without_changing_line_text() {
        let colors = Colors::default();
        let mut cell = Cell::default();
        cell.c = 'x';
        cell.flags.insert(Flags::WRAPLINE);

        let mut builder = RichScreenLineBuilder::default();
        builder.push_cell_at_col(0, &cell, ExtraTextStyle::default(), &colors);

        let line = builder.finish();

        assert_eq!(line.text, "x");
        assert!(line.wrapped);
    }

    fn detect_terminal_media_kind(input: &[u8]) -> Option<ScreenLineMediaKind> {
        let mut tracker = TerminalMediaSequenceTracker::default();
        input.iter().find_map(|byte| {
            tracker.advance(*byte).and_then(|event| event.media.map(|media| media.kind))
        })
    }

    fn join_response_bytes(responses: Vec<Vec<u8>>) -> Vec<u8> {
        responses.into_iter().flatten().collect()
    }
}
