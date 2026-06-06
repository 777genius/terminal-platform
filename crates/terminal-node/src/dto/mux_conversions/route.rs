use crate::dto::{prelude::*, *};

impl TryFrom<&NodeSessionRoute> for SessionRoute {
    type Error = ProtocolError;

    fn try_from(value: &NodeSessionRoute) -> Result<Self, Self::Error> {
        Ok(Self {
            backend: (&value.backend).into(),
            authority: (&value.authority).into(),
            external: value.external.as_ref().map(|external| terminal_domain::ExternalSessionRef {
                namespace: external.namespace.clone(),
                value: external.value.clone(),
            }),
        })
    }
}

impl From<&NodeBackendKind> for BackendKind {
    fn from(value: &NodeBackendKind) -> Self {
        match value {
            NodeBackendKind::Native => Self::Native,
            NodeBackendKind::Tmux => Self::Tmux,
            NodeBackendKind::Zellij => Self::Zellij,
        }
    }
}

impl From<&NodeRouteAuthority> for RouteAuthority {
    fn from(value: &NodeRouteAuthority) -> Self {
        match value {
            NodeRouteAuthority::LocalDaemon => Self::LocalDaemon,
            NodeRouteAuthority::ImportedForeign => Self::ImportedForeign,
        }
    }
}
