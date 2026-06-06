mod guard;
mod layout_rebuilder;
mod native_rebuilder;
mod topology_diff;

use terminal_backend_api::{BackendError, BackendSessionSummary, CreateSessionSpec};
use terminal_domain::SessionId;

use super::{
    super::runtime::SessionRuntime, SavedSessionsService, restore::guard::initial_restore_title,
    restore::guard::validate_native_restore,
    restore::native_rebuilder::SavedNativeSessionRebuilder,
};

impl SavedSessionsService<'_> {
    pub(in crate::sessions) async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        let saved = self.saved_session(session_id)?;
        validate_native_restore(&saved)?;

        let restored = self
            .runtime
            .create_native_session(CreateSessionSpec {
                title: initial_restore_title(&saved),
                launch: saved.launch.clone(),
            })
            .await?;

        SavedNativeSessionRebuilder::new(self.runtime.clone())
            .rebuild(restored.session_id, &saved)
            .await?;

        self.runtime.registry().get(restored.session_id).map(SessionRuntime::to_summary).ok_or_else(
            || BackendError::internal("restored native session is missing from registry"),
        )
    }
}
