use terminal_protocol::{ProtocolError, RequestEnvelope, ResponseEnvelope};

use crate::application::{
    TerminalDaemonActiveSessionPort, TerminalDaemonCatalogPort, TerminalDaemonSavedSessionsPort,
    TerminalDaemonSubscriptionPort, TerminalDaemonSubscriptionService,
};

mod active_sessions;
mod catalog;
mod payload;
mod saved_sessions;

#[cfg(test)]
mod tests;

pub struct TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions> {
    catalog: Catalog,
    saved_sessions: SavedSessions,
    active_sessions: ActiveSessions,
    subscriptions: TerminalDaemonSubscriptionService<Subscriptions>,
}

impl<Catalog, SavedSessions, ActiveSessions, Subscriptions>
    TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions>
{
    #[must_use]
    pub fn new(
        catalog: Catalog,
        saved_sessions: SavedSessions,
        active_sessions: ActiveSessions,
        subscriptions: TerminalDaemonSubscriptionService<Subscriptions>,
    ) -> Self {
        Self { catalog, saved_sessions, active_sessions, subscriptions }
    }
}

impl<Catalog, SavedSessions, ActiveSessions, Subscriptions>
    TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions>
where
    Catalog: TerminalDaemonCatalogPort,
    SavedSessions: TerminalDaemonSavedSessionsPort,
    ActiveSessions: TerminalDaemonActiveSessionPort,
    Subscriptions: TerminalDaemonSubscriptionPort,
{
    pub async fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        let operation_id = request.operation_id;
        let payload = self.dispatch_payload(request.payload).await?;

        Ok(ResponseEnvelope { operation_id, payload })
    }
}
