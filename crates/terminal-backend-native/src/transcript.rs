use std::sync::Mutex;

use terminal_projection::{ScreenCursor, ScreenLine};

const MAX_TRANSCRIPT_BYTES: usize = 256 * 1024;

#[derive(Default)]
pub(super) struct TranscriptBuffer {
    inner: Mutex<TranscriptState>,
}

#[derive(Default)]
struct TranscriptState {
    bytes: Vec<u8>,
    sequence: u64,
}

pub(super) struct RenderedTranscript {
    pub sequence: u64,
    pub cursor: Option<ScreenCursor>,
    pub lines: Vec<ScreenLine>,
}

impl TranscriptBuffer {
    pub(super) fn append(&self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }

        if let Ok(mut state) = self.inner.lock() {
            state.bytes.extend_from_slice(chunk);
            if state.bytes.len() > MAX_TRANSCRIPT_BYTES {
                let overflow = state.bytes.len() - MAX_TRANSCRIPT_BYTES;
                state.bytes.drain(0..overflow);
            }
            state.sequence += 1;
        }
    }

    pub(super) fn render(&self, rows: u16, cols: u16) -> RenderedTranscript {
        let Ok(state) = self.inner.lock() else {
            return RenderedTranscript {
                sequence: 0,
                cursor: Some(ScreenCursor { row: 0, col: 0 }),
                lines: vec![ScreenLine { text: String::new() }],
            };
        };
        let stripped = strip_ansi_escapes::strip(&state.bytes);
        let normalized =
            String::from_utf8_lossy(&stripped).replace("\r\n", "\n").replace('\r', "\n");
        let mut lines = normalized
            .lines()
            .map(|line| ScreenLine { text: truncate_columns(line, cols) })
            .collect::<Vec<_>>();

        if lines.is_empty() {
            lines.push(ScreenLine { text: String::new() });
        }

        let max_rows = usize::from(rows.max(1));
        if lines.len() > max_rows {
            lines = lines.split_off(lines.len() - max_rows);
        }

        RenderedTranscript {
            sequence: state.sequence,
            cursor: Some(ScreenCursor {
                row: lines.len().saturating_sub(1) as u16,
                col: lines.last().map(|line| line.text.chars().count() as u16).unwrap_or(0),
            }),
            lines,
        }
    }
}

fn truncate_columns(line: &str, cols: u16) -> String {
    let max_cols = usize::from(cols.max(1));
    line.chars().take(max_cols).collect()
}
