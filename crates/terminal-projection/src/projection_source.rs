use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionSource {
    NativeEmulator,
    NativeTranscript,
    TmuxCapturePane,
    TmuxRawOutputImport,
    ZellijViewportSubscribe,
    ZellijDumpSnapshot,
}
