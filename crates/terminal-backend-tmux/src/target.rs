use crate::{TMUX_ROUTE_NAMESPACE, prelude::*};

#[derive(Debug, Clone)]
pub(crate) struct TmuxTarget {
    pub(crate) socket_name: Option<String>,
    pub(crate) session_name: String,
}

impl TmuxTarget {
    pub(crate) fn from_route(route: &SessionRoute) -> Result<Self, BackendError> {
        if route.authority != RouteAuthority::ImportedForeign {
            return Err(BackendError::invalid_input("tmux route must be imported_foreign"));
        }
        let external = route.external.as_ref().ok_or_else(|| {
            BackendError::invalid_input("tmux route is missing external reference")
        })?;
        if external.namespace != TMUX_ROUTE_NAMESPACE {
            return Err(BackendError::invalid_input("tmux route namespace is invalid"));
        }
        let mut socket_name = None;
        let mut session_name = None;
        for part in external.value.split(';') {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            match key {
                "socket" if !value.is_empty() => socket_name = Some(value.to_string()),
                "session" if !value.is_empty() => session_name = Some(value.to_string()),
                _ => {}
            }
        }
        let session_name = session_name
            .ok_or_else(|| BackendError::invalid_input("tmux route is missing session"))?;

        Ok(Self { socket_name, session_name })
    }

    pub(crate) fn route(&self) -> SessionRoute {
        let mut value = String::new();
        if let Some(socket_name) = &self.socket_name {
            value.push_str("socket=");
            value.push_str(socket_name);
            value.push(';');
        }
        value.push_str("session=");
        value.push_str(&self.session_name);

        SessionRoute {
            backend: BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(ExternalSessionRef {
                namespace: TMUX_ROUTE_NAMESPACE.to_string(),
                value,
            }),
        }
    }
}
