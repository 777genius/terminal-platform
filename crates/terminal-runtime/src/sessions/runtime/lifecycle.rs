use terminal_backend_api::{BackendError, BackendSessionSummary, CreateSessionSpec};
use terminal_domain::BackendKind;
use terminal_projection::SessionHealthSnapshot;

use super::SessionRuntime;
use crate::registry::SessionDescriptor;

impl SessionRuntime<'_> {
    pub(in crate::sessions) async fn create_native_session(
        &self,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        let binding = self.backend(BackendKind::Native)?.create_session(spec.clone()).await?;
        let descriptor = SessionDescriptor {
            session_id: binding.session_id,
            route: binding.route,
            title: spec.title.clone(),
            launch: spec.launch.clone(),
            health: SessionHealthSnapshot::ready(binding.session_id),
        };
        let summary = Self::to_summary(descriptor.clone());
        self.upsert_session_route(descriptor.session_id, &descriptor.route)?;
        self.registry.insert(descriptor);
        if let Ok(session) = self
            .backend(BackendKind::Native)?
            .attach_session(summary.session_id, summary.route.clone())
            .await
        {
            self.start_v2_history_capture(
                SessionDescriptor {
                    session_id: summary.session_id,
                    route: summary.route.clone(),
                    title: summary.title.clone(),
                    launch: spec.launch,
                    health: SessionHealthSnapshot::ready(summary.session_id),
                },
                session,
            )
            .await;
        }

        Ok(summary)
    }
}
