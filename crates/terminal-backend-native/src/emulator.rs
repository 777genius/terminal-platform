use std::sync::Mutex;

use alacritty_terminal::{
    event::VoidListener,
    grid::Dimensions,
    term::{Config, Term},
    vte::ansi,
};
use terminal_projection::{ScreenCursor, ScreenLine, ScreenSurface};

pub(super) struct EmulatorBuffer {
    inner: Mutex<EmulatorState>,
}

struct EmulatorState {
    term: Term<VoidListener>,
    parser: ansi::Processor,
    sequence: u64,
}

#[derive(Debug, Clone)]
pub(super) struct RenderedEmulator {
    pub sequence: u64,
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
        let term = Term::new(Config::default(), &dimensions, VoidListener);

        Self {
            inner: Mutex::new(EmulatorState { term, parser: ansi::Processor::new(), sequence: 0 }),
        }
    }

    pub(super) fn advance(&self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }

        if let Ok(mut state) = self.inner.lock() {
            let EmulatorState { term, parser, sequence } = &mut *state;
            parser.advance(term, chunk);
            *sequence += 1;
        }
    }

    pub(super) fn resize(&self, rows: u16, cols: u16) {
        if let Ok(mut state) = self.inner.lock() {
            state.term.resize(TerminalDimensions::new(rows, cols));
            state.sequence += 1;
        }
    }

    pub(super) fn render(&self, title: Option<String>) -> RenderedEmulator {
        let Ok(state) = self.inner.lock() else {
            return RenderedEmulator {
                sequence: 0,
                surface: ScreenSurface {
                    title,
                    cursor: Some(ScreenCursor { row: 0, col: 0 }),
                    lines: vec![ScreenLine { text: String::new() }],
                },
            };
        };

        let content = state.term.renderable_content();
        let rows = state.term.screen_lines();
        let cols = state.term.columns();
        let mut lines = vec![String::new(); rows];

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

            let line = &mut lines[row];
            while line.chars().count() < col {
                line.push(' ');
            }
            if line.chars().count() == col {
                line.push(indexed.cell.c);
            }
            if let Some(zerowidth) = indexed.cell.zerowidth() {
                for ch in zerowidth {
                    line.push(*ch);
                }
            }
        }

        let rendered_lines = lines
            .into_iter()
            .map(|line| ScreenLine { text: line.trim_end().to_string() })
            .collect::<Vec<_>>();
        let cursor = usize::try_from(content.cursor.point.line.0)
            .ok()
            .map(|row| ScreenCursor { row: row as u16, col: content.cursor.point.column.0 as u16 });

        RenderedEmulator {
            sequence: state.sequence,
            surface: ScreenSurface { title, cursor, lines: rendered_lines },
        }
    }
}
