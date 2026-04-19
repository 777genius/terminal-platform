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
