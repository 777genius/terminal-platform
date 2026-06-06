use crate::dto::{prelude::*, *};

impl From<&BackendKind> for NodeBackendKind {
    fn from(value: &BackendKind) -> Self {
        match value {
            BackendKind::Native => Self::Native,
            BackendKind::Tmux => Self::Tmux,
            BackendKind::Zellij => Self::Zellij,
        }
    }
}

impl From<&RouteAuthority> for NodeRouteAuthority {
    fn from(value: &RouteAuthority) -> Self {
        match value {
            RouteAuthority::LocalDaemon => Self::LocalDaemon,
            RouteAuthority::ImportedForeign => Self::ImportedForeign,
        }
    }
}

impl From<&SessionRoute> for NodeSessionRoute {
    fn from(value: &SessionRoute) -> Self {
        Self {
            backend: (&value.backend).into(),
            authority: (&value.authority).into(),
            external: value.external.as_ref().map(Into::into),
        }
    }
}

impl From<&terminal_domain::ExternalSessionRef> for NodeExternalSessionRef {
    fn from(value: &terminal_domain::ExternalSessionRef) -> Self {
        Self { namespace: value.namespace.clone(), value: value.value.clone() }
    }
}
