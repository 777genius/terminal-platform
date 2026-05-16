use terminal_backend_api::BackendSessionPort;
use tokio::sync::oneshot;

use super::{SessionRuntime, V2_CAPTURE_READY_TIMEOUT, capture::run_v2_history_capture};
use crate::registry::SessionDescriptor;

impl SessionRuntime<'_> {
    pub(in crate::sessions) async fn start_v2_history_capture(
        &self,
        descriptor: SessionDescriptor,
        session: Box<dyn BackendSessionPort>,
    ) {
        let persistence = self.persistence.clone();
        let registry = self.registry.clone();
        let (ready_tx, ready_rx) = oneshot::channel();
        tokio::spawn(async move {
            run_v2_history_capture(persistence, registry, descriptor, session, ready_tx).await;
        });
        let _ = tokio::time::timeout(V2_CAPTURE_READY_TIMEOUT, ready_rx).await;
    }
}
