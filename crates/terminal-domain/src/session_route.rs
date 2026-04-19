use serde::{Deserialize, Serialize};

use crate::BackendKind;

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
