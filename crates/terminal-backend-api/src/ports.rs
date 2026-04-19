use std::path::PathBuf;

use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};
use terminal_domain::{BackendKind, SessionId, SessionRoute};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

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
    pub launch: Option<ShellLaunchSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellLaunchSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

impl ShellLaunchSpec {
    #[must_use]
    pub fn new(program: impl Into<String>) -> Self {
        Self { program: program.into(), args: Vec::new(), cwd: None }
    }

    #[must_use]
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredSession {
    pub route: SessionRoute,
    pub title: Option<String>,
}

pub trait MuxBackendPort: Send + Sync {
    fn kind(&self) -> BackendKind;

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>>;

    fn discover_sessions(
        &self,
        scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>>;

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

    fn screen_delta(
        &self,
        pane_id: terminal_domain::PaneId,
        from_sequence: u64,
    ) -> BoxFuture<'_, Result<ScreenDelta, BackendError>>;

    fn dispatch(
        &self,
        command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>>;

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>>;
}
