mod pane_surface;
mod topology;

use terminal_backend_api::{BackendError, BackendSubscription, SubscriptionSpec};

use super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(super) fn open_subscription(
        &self,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        match spec {
            SubscriptionSpec::SessionTopology => topology::open_topology_subscription(self),
            SubscriptionSpec::PaneSurface { pane_id } => {
                pane_surface::open_pane_surface_subscription(self, pane_id)
            }
        }
    }
}
