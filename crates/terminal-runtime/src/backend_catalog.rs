use std::{collections::HashMap, sync::Arc};

use terminal_backend_api::{BackendError, MuxBackendPort};
use terminal_domain::BackendKind;

#[derive(Clone, Default)]
pub struct BackendCatalog {
    backends: HashMap<BackendKind, Arc<dyn MuxBackendPort>>,
}

impl BackendCatalog {
    #[must_use]
    pub fn new(backends: impl IntoIterator<Item = Arc<dyn MuxBackendPort>>) -> Self {
        let backends = backends.into_iter().map(|backend| (backend.kind(), backend)).collect();

        Self { backends }
    }

    pub fn backend(&self, kind: BackendKind) -> Result<Arc<dyn MuxBackendPort>, BackendError> {
        self.backends
            .get(&kind)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("backend {kind:?} is not configured")))
    }

    #[must_use]
    pub fn kinds(&self) -> Vec<BackendKind> {
        let mut kinds = self.backends.keys().copied().collect::<Vec<_>>();
        kinds.sort_by_key(|kind| match kind {
            BackendKind::Native => 0,
            BackendKind::Tmux => 1,
            BackendKind::Zellij => 2,
        });
        kinds
    }
}
