mod native_snapshot;
mod orchestrator;

use terminal_backend_api::{BackendError, MuxCommandResult};
use terminal_domain::SessionId;

use super::SavedSessionsService;
use native_snapshot::SavedNativeSessionSnapshotCollector;
use orchestrator::SavedSessionSaveOrchestrator;

impl SavedSessionsService<'_> {
    pub(in crate::sessions) async fn save_session(
        &self,
        session_id: SessionId,
    ) -> Result<MuxCommandResult, BackendError> {
        let snapshot = SavedNativeSessionSnapshotCollector::new(self.runtime.clone())
            .collect(session_id)
            .await?;
        SavedSessionSaveOrchestrator::new(self.runtime.persistence()).save_native(snapshot)?;

        Ok(MuxCommandResult { changed: false })
    }
}
