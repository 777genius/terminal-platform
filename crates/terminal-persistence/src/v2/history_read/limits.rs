use super::super::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct PaneHistoryLimits {
    pub(super) from_event_seq: i64,
    pub(super) max_segments: i64,
    pub(super) max_bytes: i64,
}

impl PaneHistoryLimits {
    pub(super) fn from_inputs(
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Self {
        Self {
            from_event_seq: from_event_seq.unwrap_or(1).max(1),
            max_segments: max_segments
                .unwrap_or(DEFAULT_HISTORY_SEGMENT_LIMIT)
                .clamp(1, MAX_HISTORY_SEGMENT_LIMIT),
            max_bytes: max_bytes
                .unwrap_or(DEFAULT_HISTORY_BYTE_LIMIT)
                .clamp(1, MAX_HISTORY_BYTE_LIMIT),
        }
    }
}

pub(super) fn command_history_limit(limit: i64) -> i64 {
    if limit <= 0 { DEFAULT_COMMAND_HISTORY_LIMIT } else { limit.min(MAX_COMMAND_HISTORY_LIMIT) }
}
