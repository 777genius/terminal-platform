use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInput {
    pub id: Option<String>,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub source: Option<String>,
    pub durability_profile: Option<DurabilityProfile>,
    pub retention_policy_id: Option<String>,
    pub private_mode: bool,
    pub metadata: Option<Value>,
}

impl SessionInput {
    #[must_use]
    pub fn new(route: SessionRoute) -> Self {
        Self {
            id: None,
            route,
            title: None,
            launch: None,
            source: None,
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneInput {
    pub id: Option<String>,
    pub session_id: String,
    pub tab_id: Option<String>,
    pub stream_id: Option<String>,
    pub title: Option<String>,
    pub rows: i32,
    pub cols: i32,
    pub metadata: Option<Value>,
}

impl PaneInput {
    #[must_use]
    pub fn new(session_id: impl Into<String>, rows: i32, cols: i32) -> Self {
        Self {
            id: None,
            session_id: session_id.into(),
            tab_id: None,
            stream_id: None,
            title: None,
            rows,
            cols,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapabilityReportInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub backend_kind: String,
    pub backend_version: Option<String>,
    pub backend_binary_path_hash: Option<String>,
    pub route_kind: String,
    pub probe_status: String,
    pub capture_strategy: String,
    pub capture_semantics: String,
    pub can_preserve_process_when_live: bool,
    pub can_capture_scrollback: bool,
    pub command_boundary_confidence: String,
    pub evidence: Option<Value>,
    pub expires_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapabilityStaleInput {
    pub session_id: Option<String>,
    pub backend_kind: Option<String>,
    pub route_kind: Option<String>,
    pub stale_reason: String,
}

impl BackendCapabilityReportInput {
    #[must_use]
    pub fn from_backend_capabilities(
        backend_kind: BackendKind,
        route_kind: impl Into<String>,
        capabilities: &BackendCapabilities,
    ) -> Self {
        Self {
            id: None,
            session_id: None,
            backend_kind: format!("{backend_kind:?}").to_lowercase(),
            backend_version: None,
            backend_binary_path_hash: None,
            route_kind: route_kind.into(),
            probe_status: "passed".to_string(),
            capture_strategy: if capabilities.raw_output_stream {
                "raw_stream".to_string()
            } else if capabilities.rendered_viewport_stream {
                "rendered_stream".to_string()
            } else if capabilities.rendered_viewport_snapshot
                || capabilities.rendered_scrollback_snapshot
            {
                "rendered_snapshot".to_string()
            } else {
                "unknown".to_string()
            },
            capture_semantics: if capabilities.raw_output_stream {
                "raw_vt_stream".to_string()
            } else {
                "rendered_plaintext_snapshot".to_string()
            },
            can_preserve_process_when_live: capabilities.explicit_session_restore,
            can_capture_scrollback: capabilities.rendered_scrollback_snapshot,
            command_boundary_confidence: "unknown".to_string(),
            evidence: None,
            expires_at_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSegmentInput {
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
    pub writer_generation: String,
    pub payload: Vec<u8>,
    pub event_type: Option<String>,
    pub event_count: i64,
    pub occurred_at_ms: Option<i64>,
    pub capture_semantics: Option<String>,
    pub trust_level: Option<String>,
    pub payload_json: Option<Value>,
    pub source_event_id_hash: Option<String>,
    pub metadata: Option<Value>,
}

impl StreamSegmentInput {
    #[must_use]
    pub fn terminal_output(
        session_id: impl Into<String>,
        pane_id: impl Into<String>,
        writer_generation: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            pane_id: pane_id.into(),
            stream_id: None,
            writer_generation: writer_generation.into(),
            payload: payload.into(),
            event_type: Some("terminal_output".to_string()),
            event_count: 1,
            occurred_at_ms: None,
            capture_semantics: None,
            trust_level: None,
            payload_json: None,
            source_event_id_hash: None,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEventInput {
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: Option<String>,
    pub writer_generation: String,
    pub event_type: String,
    pub commit_kind: Option<String>,
    pub payload_json: Option<Value>,
    pub source_event_id_hash: Option<String>,
    pub occurred_at_ms: Option<i64>,
    pub capture_semantics: Option<String>,
    pub trust_level: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryClientInput {
    pub id: Option<String>,
    pub client_kind: String,
    pub install_ref_hash: Option<String>,
    pub browser_profile_ref_hash: Option<String>,
    pub user_agent_hash: Option<String>,
    pub trust_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOffsetInput {
    pub client_id: String,
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryProgressInput {
    pub client_id: String,
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
    pub last_sent_event_seq: Option<i64>,
    pub last_acked_event_seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessageInput {
    pub message_kind: String,
    pub payload: Value,
    pub dedupe_key: Option<String>,
    pub max_attempts: Option<i64>,
    pub next_run_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePressureEventInput {
    pub id: Option<String>,
    pub state: Option<String>,
    pub db_file_bytes: Option<i64>,
    pub wal_file_bytes: Option<i64>,
    pub disk_free_bytes: Option<i64>,
    pub temp_free_bytes: Option<i64>,
    pub quota_bytes: Option<i64>,
    pub action_taken: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequestInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub request_kind: Option<String>,
    pub policy_id: Option<String>,
    pub requester_ref: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequestInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub export_kind: Option<String>,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub output_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportApprovalInput {
    pub export_request_id: String,
    pub approver_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportArtifactVerificationInput {
    pub export_request_id: String,
    pub artifact_ref: String,
    pub require_encrypted: bool,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportBundleInput {
    pub id: Option<String>,
    pub scope: Value,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub output_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportBundleCompletionInput {
    pub support_bundle_id: String,
    pub artifact_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoKeyInput {
    pub id: Option<String>,
    pub key_kind: String,
    pub key_ref: String,
    pub protection_kind: String,
    pub state: Option<String>,
    pub capability_report: Option<Value>,
    pub error: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoKeyEventInput {
    pub id: Option<String>,
    pub key_id: Option<String>,
    pub event_kind: String,
    pub actor: String,
    pub status: String,
    pub error: Option<Value>,
    pub metadata: Option<Value>,
    pub occurred_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoEraseInput {
    pub id: Option<String>,
    pub key_id: String,
    pub session_id: Option<String>,
    pub requester_ref: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalArtifactInput {
    pub id: Option<String>,
    pub artifact_kind: String,
    pub artifact_ref: String,
    pub state: Option<String>,
    pub encryption_state: Option<String>,
    pub key_ref: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
    pub verified_at_ms: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocumentInput {
    pub document_id: Option<String>,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub document_kind: Option<String>,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_profile_id: Option<String>,
    pub raw_text: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiContextPackageInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub max_items: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionApprovalInput {
    pub id: Option<String>,
    pub package_id: String,
    pub action_kind: String,
    pub requester_ref: Option<String>,
    pub expires_at_ms: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionDecisionInput {
    pub approval_id: String,
    pub approved: bool,
    pub approver_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInputEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub data: String,
    pub is_paste: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_event_id: Option<String>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub shell_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellMetadataProfile {
    pub shell_kind: Option<String>,
    pub command_boundary_confidence: String,
    pub cwd_source: String,
    pub input_terminator: String,
    pub windows_profile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub tab_id: Option<String>,
    pub payload: Vec<u8>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub source_sequence: Option<u64>,
    pub occurred_at_ms: Option<i64>,
    pub capture_semantics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryGapEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub tab_id: Option<String>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub skipped_events: u64,
    pub estimated_dropped_bytes: Option<i64>,
    pub reason: String,
    pub occurred_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshotEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub tab_id: Option<String>,
    pub screen: ScreenSnapshot,
    pub buffer_kind: Option<String>,
    pub capture_semantics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySnapshotEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub topology: TopologySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBlockInput {
    pub id: Option<String>,
    pub session_id: String,
    pub pane_id: String,
    pub commit_id: Option<String>,
    pub command_text: Option<String>,
    pub display_text: Option<String>,
    pub redacted_text: Option<String>,
    pub command_text_source: Option<String>,
    pub trust_level: Option<String>,
    pub state: Option<String>,
    pub cwd: Option<String>,
    pub cwd_source: Option<String>,
    pub exit_code: Option<i32>,
    pub started_event_seq: Option<i64>,
    pub submitted_event_seq: Option<i64>,
    pub finished_event_seq: Option<i64>,
    pub output_event_seq_low: Option<i64>,
    pub output_event_seq_high: Option<i64>,
    pub output_byte_low: Option<i64>,
    pub output_byte_high: Option<i64>,
    pub sensitivity_class: Option<String>,
    pub created_at_ms: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntryInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub scope_kind: String,
    pub command_text: Option<String>,
    pub display_text: String,
    pub redacted_text: Option<String>,
    pub command_hash: Option<String>,
    pub cwd: Option<String>,
    pub shell_kind: Option<String>,
    pub trust_level: Option<String>,
    pub source: Option<String>,
    pub sensitivity_class: Option<String>,
    pub redaction_state: Option<String>,
    pub rerun_policy: Option<String>,
    pub first_used_at_ms: Option<i64>,
    pub last_used_at_ms: Option<i64>,
    pub use_count: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshotInput {
    pub id: Option<String>,
    pub session_id: String,
    pub pane_id: String,
    pub writer_generation: String,
    pub projection_source: Option<String>,
    pub buffer_kind: Option<String>,
    pub rows: i32,
    pub cols: i32,
    pub base_event_seq: i64,
    pub high_water_event_seq: i64,
    pub high_water_byte_seq: Option<i64>,
    pub screen: Value,
    pub parser_version: Option<String>,
    pub projection_version: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySnapshotInput {
    pub id: Option<String>,
    pub session_id: String,
    pub writer_generation: String,
    pub pane_high_water: Value,
    pub topology: Value,
    pub source: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreGuaranteeLevel {
    None,
    VisualSnapshotOnly,
    BasicHistory,
    DegradedHistory,
    RawStreamReplay,
    LiveMuxAttach,
}

impl RestoreGuaranteeLevel {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::VisualSnapshotOnly => "visual_snapshot_only",
            Self::BasicHistory => "basic_history",
            Self::DegradedHistory => "degraded_history",
            Self::RawStreamReplay => "raw_stream_replay",
            Self::LiveMuxAttach => "live_mux_attach",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreEvidence {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestorePlan {
    pub session_id: String,
    pub guarantee_level: RestoreGuaranteeLevel,
    pub latest_screen_snapshot_id: Option<String>,
    pub latest_topology_snapshot_id: Option<String>,
    pub high_water_commit_seq: i64,
    pub latest_restore_drill_status: Option<String>,
    pub evidence: Vec<RestoreEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneHistoryReplayStrategy {
    Empty,
    RawVtStream,
    RenderedSnapshot,
    Mixed,
    Degraded,
}

impl PaneHistoryReplayStrategy {
    #[must_use]
    pub(super) fn from_evidence(
        segments: &[StreamSegmentRecord],
        latest_screen_snapshot: Option<&ScreenSnapshotRecord>,
        gaps: &[HistoryGapRecord],
    ) -> Self {
        if !gaps.is_empty() {
            return Self::Degraded;
        }
        let has_raw = segments.iter().any(|segment| segment.capture_semantics == "raw_vt_stream");
        let has_rendered =
            segments.iter().any(|segment| segment.capture_semantics != "raw_vt_stream")
                || latest_screen_snapshot.is_some();
        match (has_raw, has_rendered) {
            (true, false) => Self::RawVtStream,
            (false, true) => Self::RenderedSnapshot,
            (true, true) => Self::Mixed,
            (false, false) => Self::Empty,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::RawVtStream => "raw_vt_stream",
            Self::RenderedSnapshot => "rendered_snapshot",
            Self::Mixed => "mixed",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSnapshotRecord {
    pub id: String,
    pub session_id: String,
    pub pane_id: String,
    pub projection_source: String,
    pub buffer_kind: String,
    pub rows: i32,
    pub cols: i32,
    pub base_event_seq: i64,
    pub high_water_event_seq: i64,
    pub high_water_byte_seq: Option<i64>,
    pub screen_json: String,
    pub parser_version: String,
    pub projection_version: String,
    pub checksum: String,
    pub created_at_ms: i64,
}

impl From<ScreenSnapshotRow> for ScreenSnapshotRecord {
    fn from(row: ScreenSnapshotRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            projection_source: row.projection_source,
            buffer_kind: row.buffer_kind,
            rows: row.rows,
            cols: row.cols,
            base_event_seq: row.base_event_seq,
            high_water_event_seq: row.high_water_event_seq,
            high_water_byte_seq: row.high_water_byte_seq,
            screen_json: row.screen_json,
            parser_version: row.parser_version,
            projection_version: row.projection_version,
            checksum: row.checksum,
            created_at_ms: row.created_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryGapRecord {
    pub id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: String,
    pub gap_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub estimated_dropped_bytes: Option<i64>,
    pub estimated_dropped_events: Option<i64>,
    pub reason: String,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
}

impl From<HistoryGapRow> for HistoryGapRecord {
    fn from(row: HistoryGapRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            stream_id: row.stream_id,
            gap_kind: row.gap_kind,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            estimated_dropped_bytes: row.estimated_dropped_bytes,
            estimated_dropped_events: row.estimated_dropped_events,
            reason: row.reason,
            opened_at_ms: row.opened_at_ms,
            closed_at_ms: row.closed_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryHydrationRecord {
    pub session_id: String,
    pub pane_id: String,
    pub from_event_seq: i64,
    pub max_segments: i64,
    pub max_bytes: i64,
    pub restore_plan: RestorePlan,
    pub latest_screen_snapshot: Option<ScreenSnapshotRecord>,
    pub segments: Vec<StreamSegmentRecord>,
    pub gaps: Vec<HistoryGapRecord>,
    pub replay_strategy: PaneHistoryReplayStrategy,
    pub has_more_segments: bool,
    pub next_event_seq: Option<i64>,
    pub total_payload_bytes: i64,
}

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
    pub(super) fn to_restore_evidence(&self) -> Vec<RestoreEvidence> {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityCheckRecord {
    pub id: String,
    pub check_kind: String,
    pub scope_kind: String,
    pub scope_ref: Option<String>,
    pub result: String,
    pub checked_at_ms: i64,
    pub details_json: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataHealthRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub detection_kind: String,
    pub severity: String,
    pub first_bad_event_seq: Option<i64>,
    pub affected_ref: Option<String>,
    pub action_state: String,
    pub detected_at_ms: i64,
    pub resolved_at_ms: Option<i64>,
    pub details_json: Option<Value>,
}

impl TryFrom<DataHealthRecordRow> for DataHealthRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: DataHealthRecordRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            detection_kind: row.detection_kind,
            severity: row.severity,
            first_bad_event_seq: row.first_bad_event_seq,
            affected_ref: row.affected_ref,
            action_state: row.action_state,
            detected_at_ms: row.detected_at_ms,
            resolved_at_ms: row.resolved_at_ms,
            details_json: row.details_json.map(|value| serde_json::from_str(&value)).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub backup_kind: String,
    pub state: String,
    pub target_ref_hash: Option<String>,
    pub manifest_json: Option<Value>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub source_db_path_hash: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub quick_check_result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceRunInput {
    pub id: Option<String>,
    pub run_kind: Option<String>,
    pub selected_policy_id: Option<String>,
    pub run_wal_checkpoint: bool,
    pub run_optimize: bool,
    pub metadata: Option<Value>,
}

impl Default for MaintenanceRunInput {
    fn default() -> Self {
        Self {
            id: None,
            run_kind: Some("scheduled_maintenance".to_string()),
            selected_policy_id: None,
            run_wal_checkpoint: true,
            run_optimize: true,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceRunRecord {
    pub id: String,
    pub run_kind: String,
    pub state: String,
    pub selected_policy_id: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub summary_json: Option<Value>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<MaintenanceRunRow> for MaintenanceRunRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: MaintenanceRunRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            run_kind: row.run_kind,
            state: row.state,
            selected_policy_id: row.selected_policy_id,
            started_at_ms: row.started_at_ms,
            finished_at_ms: row.finished_at_ms,
            summary_json: row.summary_json.as_deref().map(serde_json::from_str).transpose()?,
            error: row.error,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePressureRecord {
    pub id: String,
    pub state: String,
    pub db_file_bytes: Option<i64>,
    pub wal_file_bytes: Option<i64>,
    pub disk_free_bytes: Option<i64>,
    pub temp_free_bytes: Option<i64>,
    pub quota_bytes: Option<i64>,
    pub action_taken: String,
    pub reason: Option<String>,
    pub created_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<StoragePressureEventRow> for StoragePressureRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: StoragePressureEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl From<NewStoragePressureEventRow> for StoragePressureRecord {
    fn from(row: NewStoragePressureEventRow) -> Self {
        Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row.metadata_json.and_then(|value| serde_json::from_str(&value).ok()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteRequestRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub request_kind: String,
    pub state: String,
    pub policy_id: Option<String>,
    pub requested_at_ms: i64,
    pub approved_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub requester_ref_hash: Option<String>,
    pub reason: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<DeleteRequestRow> for DeleteRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: DeleteRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            request_kind: row.request_kind,
            state: row.state,
            policy_id: row.policy_id,
            requested_at_ms: row.requested_at_ms,
            approved_at_ms: row.approved_at_ms,
            completed_at_ms: row.completed_at_ms,
            requester_ref_hash: row.requester_ref_hash,
            reason: row.reason,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<NewDeleteRequestRow> for DeleteRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewDeleteRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            request_kind: row.request_kind,
            state: row.state,
            policy_id: row.policy_id,
            requested_at_ms: row.requested_at_ms,
            approved_at_ms: row.approved_at_ms,
            completed_at_ms: row.completed_at_ms,
            requester_ref_hash: row.requester_ref_hash,
            reason: row.reason,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletionTombstoneRecord {
    pub id: String,
    pub delete_request_id: Option<String>,
    pub session_id: Option<String>,
    pub deleted_scope: String,
    pub policy_id: Option<String>,
    pub deleted_at_ms: i64,
    pub evidence_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewDeletionTombstoneRow> for DeletionTombstoneRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewDeletionTombstoneRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            delete_request_id: row.delete_request_id,
            session_id: row.session_id,
            deleted_scope: row.deleted_scope,
            policy_id: row.policy_id,
            deleted_at_ms: row.deleted_at_ms,
            evidence_json: row
                .evidence_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRequestRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub export_kind: String,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub approved_at_ms: Option<i64>,
    pub requested_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub manifest_json: Option<Value>,
    pub output_ref_hash: Option<String>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewExportRequestRow> for ExportRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewExportRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            export_kind: row.export_kind,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            approved_at_ms: row.approved_at_ms,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<ExportRequestRow> for ExportRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: ExportRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            export_kind: row.export_kind,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            approved_at_ms: row.approved_at_ms,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportArtifactVerificationRecord {
    pub export_request_id: String,
    pub artifact_id: String,
    pub artifact_ref_hash: String,
    pub export_state: String,
    pub artifact_state: String,
    pub encryption_state: String,
    pub raw_export: bool,
    pub encrypted_required: bool,
    pub verified_at_ms: i64,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub manifest_json: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportBundleRecord {
    pub id: String,
    pub scope_json: Value,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub requested_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub manifest_json: Option<Value>,
    pub output_ref_hash: Option<String>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewSupportBundleRow> for SupportBundleRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewSupportBundleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            scope_json: serde_json::from_str(&row.scope_json)?,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<SupportBundleRow> for SupportBundleRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: SupportBundleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            scope_json: serde_json::from_str(&row.scope_json)?,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportBundleDiagnosticsRecord {
    pub support_bundle_id: String,
    pub generated_at_ms: i64,
    pub include_raw: bool,
    pub raw_content_included: bool,
    pub manifest_json: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoKeyRecord {
    pub id: String,
    pub key_kind: String,
    pub key_ref_hash: String,
    pub protection_kind: String,
    pub state: String,
    pub created_at_ms: i64,
    pub rotated_at_ms: Option<i64>,
    pub destroyed_at_ms: Option<i64>,
    pub capability_report_json: Option<Value>,
    pub error_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<CryptoKeyRow> for CryptoKeyRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: CryptoKeyRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            key_kind: row.key_kind,
            key_ref_hash: blake3_hash_text(&row.key_ref),
            protection_kind: row.protection_kind,
            state: row.state,
            created_at_ms: row.created_at_ms,
            rotated_at_ms: row.rotated_at_ms,
            destroyed_at_ms: row.destroyed_at_ms,
            capability_report_json: row
                .capability_report_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            error_json: row.error_json.as_deref().map(serde_json::from_str).transpose()?,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

impl TryFrom<NewCryptoKeyRow> for CryptoKeyRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewCryptoKeyRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            key_kind: row.key_kind,
            key_ref_hash: blake3_hash_text(&row.key_ref),
            protection_kind: row.protection_kind,
            state: row.state,
            created_at_ms: row.created_at_ms,
            rotated_at_ms: row.rotated_at_ms,
            destroyed_at_ms: row.destroyed_at_ms,
            capability_report_json: row
                .capability_report_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            error_json: row.error_json.as_deref().map(serde_json::from_str).transpose()?,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoKeyEventRecord {
    pub id: String,
    pub key_id: Option<String>,
    pub event_kind: String,
    pub actor: String,
    pub occurred_at_ms: i64,
    pub status: String,
    pub error_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewCryptoKeyEventRow> for CryptoKeyEventRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewCryptoKeyEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            key_id: row.key_id,
            event_kind: row.event_kind,
            actor: row.actor,
            occurred_at_ms: row.occurred_at_ms,
            status: row.status,
            error_json: row.error_json.as_deref().map(serde_json::from_str).transpose()?,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoEraseRecord {
    pub key_id: String,
    pub key_ref_hash: String,
    pub delete_request_id: String,
    pub tombstone_id: String,
    pub state: String,
    pub secure_deletion_limitation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionCapabilityRecord {
    pub feature_gate_state: String,
    pub active_database_key_count: i64,
    pub active_non_test_database_key_count: i64,
    pub test_plaintext_database_key_count: i64,
    pub unavailable_key_count: i64,
    pub can_enable_encrypted_history: bool,
    pub plaintext_fallback_allowed: bool,
    pub key_material_exported: bool,
    pub action_required: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalArtifactRecord {
    pub id: String,
    pub artifact_kind: String,
    pub artifact_ref_hash: String,
    pub state: String,
    pub encryption_state: String,
    pub key_ref: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at_ms: i64,
    pub verified_at_ms: Option<i64>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewExternalArtifactRow> for ExternalArtifactRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewExternalArtifactRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            artifact_kind: row.artifact_kind,
            artifact_ref_hash: row.artifact_ref_hash,
            state: row.state,
            encryption_state: row.encryption_state,
            key_ref: row.key_ref,
            checksum_algorithm: row.checksum_algorithm,
            checksum: row.checksum,
            size_bytes: row.size_bytes,
            created_at_ms: row.created_at_ms,
            verified_at_ms: row.verified_at_ms,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

impl TryFrom<ExternalArtifactRow> for ExternalArtifactRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: ExternalArtifactRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            artifact_kind: row.artifact_kind,
            artifact_ref_hash: row.artifact_ref_hash,
            state: row.state,
            encryption_state: row.encryption_state,
            key_ref: row.key_ref,
            checksum_algorithm: row.checksum_algorithm,
            checksum: row.checksum,
            size_bytes: row.size_bytes,
            created_at_ms: row.created_at_ms,
            verified_at_ms: row.verified_at_ms,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocumentRecord {
    pub document_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub document_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_profile_id: Option<String>,
    pub redaction_state: String,
    pub source_hash_algorithm: String,
    pub source_hash: String,
    pub text_preview: String,
    pub updated_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<SearchDocumentRow> for SearchDocumentRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: SearchDocumentRow) -> Result<Self, Self::Error> {
        Ok(Self {
            document_id: row.document_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            command_block_id: row.command_block_id,
            document_kind: row.document_kind,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            redaction_profile_id: row.redaction_profile_id,
            redaction_state: row.redaction_state,
            source_hash_algorithm: row.source_hash_algorithm,
            source_hash: row.source_hash,
            text_preview: row.text_preview,
            updated_at_ms: row.updated_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextPackageRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub requested_at_ms: i64,
    pub built_at_ms: Option<i64>,
    pub item_count: i64,
    pub manifest_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiContextPackageRow> for AiContextPackageRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiContextPackageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            requested_at_ms: row.requested_at_ms,
            built_at_ms: row.built_at_ms,
            item_count: row.item_count,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextItemRecord {
    pub id: String,
    pub package_id: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_state: String,
    pub data_only: bool,
    pub content_preview: String,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiContextItemRow> for AiContextItemRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiContextItemRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            source_kind: row.source_kind,
            source_ref: row.source_ref,
            session_id: row.session_id,
            pane_id: row.pane_id,
            command_block_id: row.command_block_id,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            redaction_state: row.redaction_state,
            data_only: row.data_only != 0,
            content_preview: row.content_preview,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptInjectionFindingRecord {
    pub id: String,
    pub package_id: Option<String>,
    pub item_id: Option<String>,
    pub severity: String,
    pub pattern_kind: String,
    pub action_state: String,
    pub detected_at_ms: i64,
    pub evidence_preview: String,
    pub metadata_json: Option<Value>,
}

impl TryFrom<PromptInjectionFindingRow> for PromptInjectionFindingRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: PromptInjectionFindingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            item_id: row.item_id,
            severity: row.severity,
            pattern_kind: row.pattern_kind,
            action_state: row.action_state,
            detected_at_ms: row.detected_at_ms,
            evidence_preview: row.evidence_preview,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiActionApprovalRecord {
    pub id: String,
    pub package_id: Option<String>,
    pub action_kind: String,
    pub state: String,
    pub requester_ref_hash: Option<String>,
    pub approver_ref_hash: Option<String>,
    pub requested_at_ms: i64,
    pub decided_at_ms: Option<i64>,
    pub expires_at_ms: Option<i64>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiActionApprovalRow> for AiActionApprovalRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiActionApprovalRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            action_kind: row.action_kind,
            state: row.state,
            requester_ref_hash: row.requester_ref_hash,
            approver_ref_hash: row.approver_ref_hash,
            requested_at_ms: row.requested_at_ms,
            decided_at_ms: row.decided_at_ms,
            expires_at_ms: row.expires_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<NewAiActionApprovalRow> for AiActionApprovalRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewAiActionApprovalRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            action_kind: row.action_kind,
            state: row.state,
            requester_ref_hash: row.requester_ref_hash,
            approver_ref_hash: row.approver_ref_hash,
            requested_at_ms: row.requested_at_ms,
            decided_at_ms: row.decided_at_ms,
            expires_at_ms: row.expires_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryClientRecord {
    pub id: String,
    pub client_kind: String,
    pub last_seen_at_ms: i64,
    pub trust_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryOffsetRecord {
    pub id: String,
    pub client_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: String,
    pub last_sent_event_seq: i64,
    pub last_acked_event_seq: i64,
    pub last_persisted_event_seq: i64,
    pub replay_from_event_seq: Option<i64>,
    pub gap_state: String,
    pub updated_at_ms: i64,
}

impl From<DeliveryOffsetRow> for DeliveryOffsetRecord {
    fn from(row: DeliveryOffsetRow) -> Self {
        Self {
            id: row.id,
            client_id: row.client_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            stream_id: row.stream_id,
            last_sent_event_seq: row.last_sent_event_seq,
            last_acked_event_seq: row.last_acked_event_seq,
            last_persisted_event_seq: row.last_persisted_event_seq,
            replay_from_event_seq: row.replay_from_event_seq,
            gap_state: row.gap_state,
            updated_at_ms: row.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReplayWindow {
    pub from_event_seq: Option<i64>,
    pub to_event_seq: i64,
    pub gap_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxMessageRecord {
    pub id: String,
    pub message_kind: String,
    pub dedupe_key: Option<String>,
    pub state: String,
    pub payload_json: Value,
    pub attempts: i64,
    pub max_attempts: i64,
    pub claimed_by: Option<String>,
    pub lease_token: Option<String>,
    pub claimed_until_ms: Option<i64>,
    pub next_run_at_ms: i64,
    pub last_error: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub pending_count: i64,
    pub due_pending_count: i64,
    pub claimed_count: i64,
    pub stale_claim_count: i64,
    pub done_count: i64,
    pub failed_count: i64,
    pub quarantined_count: i64,
    pub oldest_due_pending_age_ms: Option<i64>,
    pub next_pending_due_in_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub feature_gate_state: String,
    pub raw_segment_count: i64,
    pub zstd_segment_count: i64,
    pub unsupported_segment_count: i64,
    pub rewrite_candidate_count: i64,
    pub segments_rewritten: i64,
    pub restore_drill_required: bool,
    pub action_taken: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub policy_id: String,
    pub policy_kind: String,
    pub pressure_behavior: String,
    pub raw_history_prune_behavior: String,
    pub sessions_scanned: i64,
    pub scan_mode: String,
    pub maintenance_deletes_raw_history: bool,
    pub action_taken: String,
}

impl TryFrom<OutboxMessageRow> for OutboxMessageRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: OutboxMessageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            message_kind: row.message_kind,
            dedupe_key: row.dedupe_key,
            state: row.state,
            payload_json: serde_json::from_str(&row.payload_json)?,
            attempts: row.attempts,
            max_attempts: row.max_attempts,
            claimed_by: row.claimed_by,
            lease_token: row.lease_token,
            claimed_until_ms: row.claimed_until_ms,
            next_run_at_ms: row.next_run_at_ms,
            last_error: row.last_error,
            created_at_ms: row.created_at_ms,
            updated_at_ms: row.updated_at_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterGenerationLease {
    pub id: String,
    pub process_id: String,
    pub lease_token: String,
    pub lease_expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSegmentReceipt {
    pub commit_id: String,
    pub commit_seq: i64,
    pub segment_id: String,
    pub event_id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEventReceipt {
    pub commit_id: String,
    pub commit_seq: i64,
    pub event_id: String,
    pub event_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamSegmentRecord {
    pub id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub payload: Vec<u8>,
    pub checksum: String,
    pub capture_semantics: String,
    pub created_at_ms: i64,
}

impl From<StreamSegmentRow> for StreamSegmentRecord {
    fn from(row: StreamSegmentRow) -> Self {
        Self {
            id: row.id,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            payload: row.payload,
            checksum: row.checksum,
            capture_semantics: row.capture_semantics,
            created_at_ms: row.created_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandHistoryEntryRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub display_text: String,
    pub last_used_at_ms: i64,
    pub use_count: i64,
}

impl From<CommandHistoryEntryRow> for CommandHistoryEntryRecord {
    fn from(row: CommandHistoryEntryRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            display_text: row.display_text,
            last_used_at_ms: row.last_used_at_ms,
            use_count: row.use_count,
        }
    }
}
