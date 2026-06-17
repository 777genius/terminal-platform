use std::{
    io::{Read as _, Write as _},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

use terminal_backend_api::{BackendRawOutputBytes, BackendRawOutputEvent};
use terminal_domain::PaneId;
use tokio::sync::{broadcast, watch};

use crate::{emulator::EmulatorBuffer, transcript::TranscriptBuffer};

use super::super::signals::bump_watch;

pub(super) struct ReaderThreadParts {
    pub pane_id: PaneId,
    pub reader: Box<dyn std::io::Read + Send>,
    pub writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    pub transcript: Arc<TranscriptBuffer>,
    pub emulator: Arc<EmulatorBuffer>,
    pub raw_output_sequence: Arc<AtomicU64>,
    pub raw_output_tick: broadcast::Sender<BackendRawOutputEvent>,
    pub surface_tick: watch::Sender<u64>,
}

pub(super) fn spawn_reader_thread(parts: ReaderThreadParts) {
    let ReaderThreadParts {
        pane_id,
        mut reader,
        writer,
        transcript,
        emulator,
        raw_output_sequence,
        raw_output_tick,
        surface_tick,
    } = parts;

    thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    transcript.append(&chunk[..read]);
                    emulator.advance(&chunk[..read]);
                    flush_terminal_response_bytes(&emulator, &writer);
                    let sequence = raw_output_sequence.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ =
                        raw_output_tick.send(BackendRawOutputEvent::Bytes(BackendRawOutputBytes {
                            pane_id,
                            sequence,
                            payload: chunk[..read].to_vec(),
                        }));
                    bump_watch(&surface_tick);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });
}

fn flush_terminal_response_bytes(
    emulator: &Arc<EmulatorBuffer>,
    writer: &Arc<Mutex<Box<dyn std::io::Write + Send>>>,
) {
    let responses = emulator.take_response_bytes();
    if responses.is_empty() {
        return;
    }

    if let Ok(mut writer) = writer.lock() {
        for response in responses {
            let _ = writer.write_all(&response);
        }
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct SharedBufferWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    impl std::io::Write for SharedBufferWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.bytes.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn flushes_terminal_response_bytes_to_pty_writer() {
        let emulator = Arc::new(EmulatorBuffer::new(4, 80));
        emulator.advance(b"\x1b[c");
        let writer = SharedBufferWriter::default();
        let bytes = Arc::clone(&writer.bytes);
        let writer: Arc<Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(Mutex::new(Box::new(writer)));

        flush_terminal_response_bytes(&emulator, &writer);

        assert_eq!(&*bytes.lock().unwrap(), b"\x1b[?6c");
        assert!(emulator.take_response_bytes().is_empty());
    }
}
