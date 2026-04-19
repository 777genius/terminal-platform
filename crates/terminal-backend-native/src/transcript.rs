use std::sync::Mutex;

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
}
