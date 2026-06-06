mod access;
mod attach;
mod capture;
mod capture_start;
mod health;
mod helpers;
mod lifecycle;
mod routes;
#[cfg(test)]
mod tests;

use terminal_persistence::SqliteSessionStore;

use crate::{backend_catalog::BackendCatalog, registry::SessionRegistry};

pub(super) use helpers::{
    collect_pane_ids_from_node, collect_pane_ids_from_topology, command_updates_summary_title,
    saved_session_title, tab_id_for_pane, tab_snapshot_by_id,
};
#[cfg(test)]
pub(super) use helpers::{session_health_from_attach_error, session_route_fingerprint};

const V2_RAW_CAPTURE_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
const V2_RAW_CAPTURE_MAX_BATCH_BYTES: usize = 64 * 1024;
const V2_RENDERED_CAPTURE_FLUSH_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(250);
const V2_CAPTURE_READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone)]
pub(super) struct SessionRuntime<'a> {
    backends: &'a BackendCatalog,
    registry: std::sync::Arc<dyn SessionRegistry>,
    persistence: &'a SqliteSessionStore,
}

impl<'a> SessionRuntime<'a> {
    pub(super) fn new(
        backends: &'a BackendCatalog,
        registry: std::sync::Arc<dyn SessionRegistry>,
        persistence: &'a SqliteSessionStore,
    ) -> Self {
        Self { backends, registry, persistence }
    }
}
