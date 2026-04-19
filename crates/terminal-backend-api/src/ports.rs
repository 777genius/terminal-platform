use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};
use terminal_domain::{BackendKind, SessionId, SessionRoute};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};

use crate::{
    BackendCapabilities, BackendError, BackendSubscription, MuxCommand, MuxCommandResult,
    SubscriptionSpec,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendScope {
    CurrentUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CreateSessionSpec {
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendSessionBinding {
    pub session_id: SessionId,
    pub route: SessionRoute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendSessionSummary {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
}

pub trait MuxBackendPort: Send + Sync {
    fn kind(&self) -> BackendKind;

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>>;

    fn create_session(
        &self,
        spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>>;

    fn attach_session(
        &self,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>>;

    fn list_sessions(
        &self,
        scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>>;
}

pub trait BackendSessionPort: Send + Sync {
    fn topology_snapshot(&self) -> BoxFuture<'_, Result<TopologySnapshot, BackendError>>;

    fn screen_snapshot(
        &self,
        pane_id: terminal_domain::PaneId,
    ) -> BoxFuture<'_, Result<ScreenSnapshot, BackendError>>;

    fn dispatch(
        &self,
        command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>>;

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>>;
}
