use terminal_backend_api::BackendError;
use terminal_domain::{RouteAuthority, SessionRoute};

use crate::constants::ZELLIJ_ROUTE_NAMESPACE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZellijTarget {
    pub(crate) session_name: String,
}

impl ZellijTarget {
    pub(crate) fn from_route(route: &SessionRoute) -> Result<Self, BackendError> {
        if route.authority != RouteAuthority::ImportedForeign {
            return Err(BackendError::invalid_input("zellij route must be imported_foreign"));
        }
        let external = route.external.as_ref().ok_or_else(|| {
            BackendError::invalid_input("zellij route is missing external reference")
        })?;
        if external.namespace != ZELLIJ_ROUTE_NAMESPACE {
            return Err(BackendError::invalid_input("zellij route namespace is invalid"));
        }
        let session_name = external
            .value
            .strip_prefix("session=")
            .ok_or_else(|| BackendError::invalid_input("zellij route is missing session"))?;

        Ok(Self { session_name: session_name.to_string() })
    }
}
