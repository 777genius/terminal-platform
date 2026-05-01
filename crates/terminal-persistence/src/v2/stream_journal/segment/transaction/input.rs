use super::super::super::super::*;

pub(in crate::v2::stream_journal::segment) struct AppendStreamSegmentTransaction<'a> {
    pub(in crate::v2::stream_journal::segment) input: &'a StreamSegmentInput,
    pub(in crate::v2::stream_journal::segment) stream_id: &'a str,
    pub(in crate::v2::stream_journal::segment) payload_len: i64,
    pub(in crate::v2::stream_journal::segment) payload_checksum: &'a str,
    pub(in crate::v2::stream_journal::segment) metadata_json: Option<String>,
    pub(in crate::v2::stream_journal::segment) source_event_id_hash: Option<String>,
    pub(in crate::v2::stream_journal::segment) capture_source_kind: Option<String>,
    pub(in crate::v2::stream_journal::segment) buffer_mode_transitions:
        &'a [BufferModeTransition],
    pub(in crate::v2::stream_journal::segment) occurred_at_ms: i64,
    pub(in crate::v2::stream_journal::segment) now: i64,
    pub(in crate::v2::stream_journal::segment) fail_after_segment_insert: bool,
}
