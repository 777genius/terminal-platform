use terminal_backend_api::{BackendError, BackendScope, BoxFuture, DiscoveredSession};
use terminal_domain::{BackendKind, ExternalSessionRef, RouteAuthority, SessionRoute};

use crate::{cli::is_transient_zellij_backend_error, constants::ZELLIJ_ROUTE_NAMESPACE};

use super::ZellijBackend;

impl ZellijBackend {
    pub(super) fn discover_sessions_inner(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async move {
            let output = match self.run(None, &["list-sessions", "--short", "--no-formatting"]) {
                Ok(output) => output,
                Err(error) if is_transient_zellij_backend_error(&error) => return Ok(Vec::new()),
                Err(error) => return Err(error),
            };
            let sessions = output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && *line != "No active zellij sessions found.")
                .map(|session_name| {
                    let route = SessionRoute {
                        backend: BackendKind::Zellij,
                        authority: RouteAuthority::ImportedForeign,
                        external: Some(ExternalSessionRef {
                            namespace: ZELLIJ_ROUTE_NAMESPACE.to_string(),
                            value: format!("session={session_name}"),
                        }),
                    };

                    DiscoveredSession { route, title: Some(session_name.to_string()) }
                })
                .collect();

            Ok(sessions)
        })
    }
}
