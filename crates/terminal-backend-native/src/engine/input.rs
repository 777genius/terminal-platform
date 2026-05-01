use std::{borrow::Cow, io::Write as _};

use portable_pty::PtySize;
use terminal_backend_api::BackendError;

use super::model::NativePaneRuntime;

impl NativePaneRuntime {
    pub(super) fn write_text(&self, text: &str) -> Result<(), BackendError> {
        let normalized = normalize_pty_input(text);
        self.write_all(normalized.as_bytes())
    }

    fn write_all(&self, bytes: &[u8]) -> Result<(), BackendError> {
        let process = self
            .process
            .lock()
            .map_err(|_| BackendError::internal("native pane process lock poisoned"))?;
        let mut writer = process
            .writer
            .lock()
            .map_err(|_| BackendError::internal("native pane writer lock poisoned"))?;
        writer.write_all(bytes).map_err(|error| {
            BackendError::transport(format!("failed to write to pty - {error}"))
        })?;
        writer.flush().map_err(|error| {
            BackendError::transport(format!("failed to flush pty writer - {error}"))
        })?;
        Ok(())
    }

    pub(super) fn resize(&self, rows: u16, cols: u16) -> Result<(), BackendError> {
        let process = self
            .process
            .lock()
            .map_err(|_| BackendError::internal("native pane process lock poisoned"))?;
        process
            .master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|error| BackendError::transport(format!("failed to resize pty - {error}")))?;
        drop(process);
        let mut geometry = self
            .geometry
            .lock()
            .map_err(|_| BackendError::internal("native pane geometry lock poisoned"))?;
        geometry.rows = rows;
        geometry.cols = cols;
        drop(geometry);
        self.emulator.resize(rows, cols);
        self.mark_surface_dirty();
        Ok(())
    }
}

fn normalize_pty_input(text: &str) -> Cow<'_, str> {
    #[cfg(windows)]
    {
        normalize_windows_pty_input(text)
    }

    #[cfg(not(windows))]
    {
        Cow::Borrowed(text)
    }
}

#[cfg(any(test, windows))]
fn normalize_windows_pty_input(text: &str) -> Cow<'_, str> {
    if !text.bytes().any(|byte| matches!(byte, b'\r' | b'\n')) {
        return Cow::Borrowed(text);
    }

    let mut normalized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                normalized.push('\r');
            }
            '\n' => normalized.push('\r'),
            _ => normalized.push(ch),
        }
    }

    Cow::Owned(normalized)
}

#[cfg(test)]
mod tests {
    use super::normalize_windows_pty_input;

    #[test]
    fn normalize_windows_pty_input_preserves_plain_text() {
        assert_eq!(normalize_windows_pty_input("plain text").as_ref(), "plain text");
    }

    #[test]
    fn normalize_windows_pty_input_collapses_newline_variants_to_carriage_return() {
        assert_eq!(normalize_windows_pty_input("alpha\r\nbeta").as_ref(), "alpha\rbeta");
        assert_eq!(normalize_windows_pty_input("alpha\nbeta").as_ref(), "alpha\rbeta");
        assert_eq!(normalize_windows_pty_input("alpha\rbeta").as_ref(), "alpha\rbeta");
    }
}
