use std::sync::{Arc, Mutex as StdMutex};

use terminal_backend_api::{BackendError, BackendScope, BackendSessionPort, BoxFuture};
use terminal_domain::{DegradedModeReason, SessionId, SessionRoute};
use tokio::sync::Mutex;

use crate::{probe::ZellijSurface, session::ZellijAttachedSession, target::ZellijTarget};

use super::ZellijBackend;

impl ZellijBackend {
    pub(super) fn attach_session_inner(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        let backend = self.clone();
        Box::pin(async move {
            let target = ZellijTarget::from_route(&route)?;
            let probe = backend.probe()?;
            let sessions = backend.discover_sessions_inner(BackendScope::CurrentUser).await?;
            if !sessions.iter().any(|session| session.route == route) {
                return Err(BackendError::not_found(format!(
                    "zellij session '{}' is not active",
                    target.session_name
                )));
            }

            match probe.surface {
                ZellijSurface::RichCli044Plus => {
                    let attached = ZellijAttachedSession {
                        backend: Arc::new(backend),
                        session_id,
                        target,
                        io_lane: Arc::new(StdMutex::new(())),
                        command_lane: Arc::new(Mutex::new(())),
                    };
                    attached.snapshot()?;

                    Ok(Box::new(attached) as Box<dyn BackendSessionPort>)
                }
                ZellijSurface::LegacyCli043 => Err(BackendError::unsupported(
                    format!(
                        "zellij {} does not expose the list-panes/list-tabs/subscribe surface required for imported attach",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                )),
                ZellijSurface::Unknown => Err(BackendError::unsupported(
                    format!(
                        "zellij {} could not be matched to a supported control surface",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                )),
            }
        })
    }
}
