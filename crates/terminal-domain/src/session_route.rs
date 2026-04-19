use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::{BackendKind, SessionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAuthority {
    LocalDaemon,
    ImportedForeign,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSessionRef {
    pub namespace: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRoute {
    pub backend: BackendKind,
    pub authority: RouteAuthority,
    pub external: Option<ExternalSessionRef>,
}

const NATIVE_SESSION_NAMESPACE: &str = "native_session";

#[must_use]
pub fn imported_session_id(route: &SessionRoute) -> Option<SessionId> {
    if route.authority != RouteAuthority::ImportedForeign {
        return None;
    }

    let external = route.external.as_ref()?;
    let fingerprint = format!(
        "terminal-platform/imported/{:?}/{}/{}",
        route.backend, external.namespace, external.value
    );

    Some(SessionId::from(Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes())))
}

#[must_use]
pub fn local_native_route(session_id: SessionId) -> SessionRoute {
    SessionRoute {
        backend: BackendKind::Native,
        authority: RouteAuthority::LocalDaemon,
        external: Some(ExternalSessionRef {
            namespace: NATIVE_SESSION_NAMESPACE.to_string(),
            value: session_id.0.to_string(),
        }),
    }
}

#[must_use]
pub fn local_native_session_id(route: &SessionRoute) -> Option<SessionId> {
    if route.backend != BackendKind::Native || route.authority != RouteAuthority::LocalDaemon {
        return None;
    }

    let external = route.external.as_ref()?;
    if external.namespace != NATIVE_SESSION_NAMESPACE {
        return None;
    }

    Uuid::parse_str(&external.value).ok().map(SessionId::from)
}

#[cfg(test)]
mod tests {
    use crate::{BackendKind, RouteAuthority};

    use super::{local_native_route, local_native_session_id};

    #[test]
    fn roundtrips_local_native_route_identity() {
        let session_id = crate::SessionId::new();
        let route = local_native_route(session_id);

        assert_eq!(route.backend, BackendKind::Native);
        assert_eq!(route.authority, RouteAuthority::LocalDaemon);
        assert_eq!(local_native_session_id(&route), Some(session_id));
    }
}
