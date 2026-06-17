use std::{fs, path::Path};

use ts_rs::{Config, TS};

use crate::dto::*;

pub fn export_typescript_bindings_to(out_dir: impl AsRef<Path>) -> std::io::Result<()> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)?;
    let cfg = Config::default().with_out_dir(out_dir).with_import_extension(Some("js"));

    NodeBindingVersion::export_all(&cfg).map_err(export_error)?;
    NodeCreateSessionRequest::export_all(&cfg).map_err(export_error)?;
    NodeHandshakeInfo::export_all(&cfg).map_err(export_error)?;
    NodeSessionSummary::export_all(&cfg).map_err(export_error)?;
    NodeDiscoveredSession::export_all(&cfg).map_err(export_error)?;
    NodeBackendCapabilitiesInfo::export_all(&cfg).map_err(export_error)?;
    NodeSavedSessionSummary::export_all(&cfg).map_err(export_error)?;
    NodeSavedSessionRecord::export_all(&cfg).map_err(export_error)?;
    NodeRestoredSession::export_all(&cfg).map_err(export_error)?;
    NodeDeleteSavedSessionResult::export_all(&cfg).map_err(export_error)?;
    NodePruneSavedSessionsResult::export_all(&cfg).map_err(export_error)?;
    NodePaneHistoryReplayStrategy::export_all(&cfg).map_err(export_error)?;
    NodePaneHistoryRestoreEvidence::export_all(&cfg).map_err(export_error)?;
    NodePaneHistoryRestorePlan::export_all(&cfg).map_err(export_error)?;
    NodePaneHistoryScreenSnapshot::export_all(&cfg).map_err(export_error)?;
    NodePaneHistorySegment::export_all(&cfg).map_err(export_error)?;
    NodePaneHistoryGap::export_all(&cfg).map_err(export_error)?;
    NodePaneHistory::export_all(&cfg).map_err(export_error)?;
    NodeCommandHistoryEntry::export_all(&cfg).map_err(export_error)?;
    NodeTopologySnapshot::export_all(&cfg).map_err(export_error)?;
    NodeScreenBufferKind::export_all(&cfg).map_err(export_error)?;
    NodeScreenCursorShape::export_all(&cfg).map_err(export_error)?;
    NodeScreenColor::export_all(&cfg).map_err(export_error)?;
    NodeScreenSurfacePalette::export_all(&cfg).map_err(export_error)?;
    NodeScreenProgressState::export_all(&cfg).map_err(export_error)?;
    NodeScreenProgress::export_all(&cfg).map_err(export_error)?;
    NodeScreenUnderlineStyle::export_all(&cfg).map_err(export_error)?;
    NodeScreenTextBorderStyle::export_all(&cfg).map_err(export_error)?;
    NodeScreenTextBaseline::export_all(&cfg).map_err(export_error)?;
    NodeScreenTextStyle::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineSpan::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineMediaKind::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineMedia::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineSideEffectKind::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineSideEffectDisposition::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineSideEffectTarget::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineSideEffect::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineSemanticMarkKind::export_all(&cfg).map_err(export_error)?;
    NodeScreenLineSemanticMark::export_all(&cfg).map_err(export_error)?;
    NodeScreenSnapshot::export_all(&cfg).map_err(export_error)?;
    NodeScreenDelta::export_all(&cfg).map_err(export_error)?;
    NodeSessionHealthPhase::export_all(&cfg).map_err(export_error)?;
    NodeSessionHealthReason::export_all(&cfg).map_err(export_error)?;
    NodeSessionHealthSnapshot::export_all(&cfg).map_err(export_error)?;
    NodeMuxCommand::export_all(&cfg).map_err(export_error)?;
    NodeMuxCommandResult::export_all(&cfg).map_err(export_error)?;
    NodeSubscriptionSpec::export_all(&cfg).map_err(export_error)?;
    NodeSubscriptionEvent::export_all(&cfg).map_err(export_error)?;
    NodeSubscriptionMeta::export_all(&cfg).map_err(export_error)?;
    NodeAttachedSession::export_all(&cfg).map_err(export_error)?;

    Ok(())
}

pub fn export_typescript_bindings() -> std::io::Result<()> {
    export_typescript_bindings_to("./bindings")
}

fn export_error(error: ts_rs::ExportError) -> std::io::Error {
    std::io::Error::other(error)
}
