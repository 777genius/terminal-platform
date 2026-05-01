use serde::{Deserialize, Serialize};

use super::RestoreEvidence;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreDrillRecord {
    pub id: String,
    pub session_id: String,
    pub drill_kind: String,
    pub result: String,
    pub restore_guarantee_level: String,
    pub checked_at_ms: i64,
    pub duration_ms: Option<i64>,
    pub source_snapshot_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreReplaySafetyRecord {
    pub session_id: String,
    pub scanned_segment_count: i64,
    pub osc52_clipboard_count: i64,
    pub title_sequence_count: i64,
    pub hyperlink_sequence_count: i64,
    pub cwd_sequence_count: i64,
    pub shell_marker_sequence_count: i64,
    pub bel_byte_count: i64,
    pub side_effects_suppressed: bool,
    pub prompt_injection_text_is_data: bool,
}

impl RestoreReplaySafetyRecord {
    pub(crate) fn to_restore_evidence(&self) -> Vec<RestoreEvidence> {
        vec![
            RestoreEvidence {
                kind: "historical_replay_side_effects_suppressed".to_string(),
                value: self.side_effects_suppressed.to_string(),
            },
            RestoreEvidence {
                kind: "historical_replay_prompt_injection_text_is_data".to_string(),
                value: self.prompt_injection_text_is_data.to_string(),
            },
            RestoreEvidence {
                kind: "historical_replay_osc52_clipboard_count".to_string(),
                value: self.osc52_clipboard_count.to_string(),
            },
            RestoreEvidence {
                kind: "historical_replay_title_sequence_count".to_string(),
                value: self.title_sequence_count.to_string(),
            },
            RestoreEvidence {
                kind: "historical_replay_hyperlink_sequence_count".to_string(),
                value: self.hyperlink_sequence_count.to_string(),
            },
            RestoreEvidence {
                kind: "historical_replay_cwd_sequence_count".to_string(),
                value: self.cwd_sequence_count.to_string(),
            },
            RestoreEvidence {
                kind: "historical_replay_shell_marker_sequence_count".to_string(),
                value: self.shell_marker_sequence_count.to_string(),
            },
            RestoreEvidence {
                kind: "historical_replay_bel_byte_count".to_string(),
                value: self.bel_byte_count.to_string(),
            },
        ]
    }
}
