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

pub(super) fn spawn_reader_thread(
    pane_id: PaneId,
    mut reader: Box<dyn std::io::Read + Send>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    transcript: Arc<TranscriptBuffer>,
    emulator: Arc<EmulatorBuffer>,
    raw_output_sequence: Arc<AtomicU64>,
    raw_output_tick: broadcast::Sender<BackendRawOutputEvent>,
    surface_tick: watch::Sender<u64>,
) {
    thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    respond_to_cursor_inherit_query(&chunk[..read], &writer);
                    transcript.append(&chunk[..read]);
                    emulator.advance(&chunk[..read]);
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

fn respond_to_cursor_inherit_query(
    chunk: &[u8],
    writer: &Arc<Mutex<Box<dyn std::io::Write + Send>>>,
) {
    #[cfg(windows)]
    {
        // CreatePseudoConsole warns that inheriting the cursor can deadlock unless the host
        // answers the cursor-position query received on the output pipe. v1 now pins the
        // vendored portable-pty path to dwFlags = 0, but keep this safeguard so unexpected
        // ConPTY hosts or future vendor drift do not wedge the pipe.
        if chunk.windows(4).any(|window| window == b"\x1b[6n")
            && let Ok(mut writer) = writer.lock()
        {
            let _ = writer.write_all(b"\x1b[1;1R");
            let _ = writer.flush();
        }
    }

    #[cfg(not(windows))]
    let _ = (chunk, writer);
}
