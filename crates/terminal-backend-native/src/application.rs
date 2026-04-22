use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BoxFuture, CreateSessionSpec, DiscoveredSession,
    MuxBackendPort, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, SessionId, SessionRoute, local_native_route,
    local_native_session_id,
};

use crate::{engine::NativeSessionEngine, subscriptions::open_native_subscription};

#[derive(Default)]
pub struct NativeBackend {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<NativeSessionEngine>>>>,
}

impl NativeBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Native
    }
}

impl MuxBackendPort for NativeBackend {
    fn kind(&self) -> BackendKind {
        self.kind()
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async { Ok(native_capabilities()) })
    }

    fn discover_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "native backend is created through canonical session creation",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }

    fn create_session(
        &self,
        spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async move {
            let session_id = SessionId::new();
            let route = local_native_route(session_id);
            let runtime = Arc::new(NativeSessionEngine::spawn(session_id, route.clone(), spec)?);

            let mut sessions = self
                .sessions
                .write()
                .map_err(|_| BackendError::internal("native backend write lock poisoned"))?;
            sessions.insert(session_id, runtime);

            Ok(BackendSessionBinding { session_id, route })
        })
    }

    fn attach_session(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async move {
            let runtime = self.resolve_session_runtime(session_id, route)?;
            Ok(Box::new(NativeAttachedSession { runtime }) as Box<dyn BackendSessionPort>)
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async move {
            let sessions = self
                .sessions
                .read()
                .map_err(|_| BackendError::internal("native backend read lock poisoned"))?;
            let mut summaries = Vec::with_capacity(sessions.len());
            for runtime in sessions.values() {
                summaries.push(runtime.summary()?);
            }
            Ok(summaries)
        })
    }
}

impl NativeBackend {
    fn resolve_session_runtime(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> Result<Arc<NativeSessionEngine>, BackendError> {
        if route.backend != BackendKind::Native {
            return Err(BackendError::invalid_input(
                "native backend can only attach native routes",
            ));
        }

        let sessions = self
            .sessions
            .read()
            .map_err(|_| BackendError::internal("native backend read lock poisoned"))?;

        if let Some(route_session_id) = local_native_session_id(&route) {
            if route_session_id != session_id {
                return Err(BackendError::invalid_input(
                    "native attach session id does not match route identity",
                ));
            }
            return sessions
                .get(&route_session_id)
                .map(Arc::clone)
                .ok_or_else(|| BackendError::not_found("native route is not registered"));
        }

        sessions
            .values()
            .find(|runtime| runtime.summary().is_ok_and(|summary| summary.route == route))
            .map(Arc::clone)
            .ok_or_else(|| BackendError::not_found("native route is not registered"))
    }
}

struct NativeAttachedSession {
    runtime: Arc<NativeSessionEngine>,
}

impl BackendSessionPort for NativeAttachedSession {
    fn topology_snapshot(
        &self,
    ) -> BoxFuture<'_, Result<terminal_projection::TopologySnapshot, BackendError>> {
        Box::pin(async move { self.runtime.topology_snapshot() })
    }

    fn screen_snapshot(
        &self,
        pane_id: terminal_domain::PaneId,
    ) -> BoxFuture<'_, Result<terminal_projection::ScreenSnapshot, BackendError>> {
        Box::pin(async move { self.runtime.screen_snapshot(pane_id) })
    }

    fn screen_delta(
        &self,
        pane_id: terminal_domain::PaneId,
        from_sequence: u64,
    ) -> BoxFuture<'_, Result<terminal_projection::ScreenDelta, BackendError>> {
        Box::pin(async move { self.runtime.screen_delta(pane_id, from_sequence) })
    }

    fn dispatch(
        &self,
        command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        Box::pin(async move {
            let changed = match command {
                MuxCommand::NewTab(spec) => self.runtime.new_tab(spec)?,
                MuxCommand::SplitPane(spec) => self.runtime.split_pane(spec)?,
                MuxCommand::FocusTab { tab_id } => self.runtime.focus_tab(tab_id)?,
                MuxCommand::RenameTab { tab_id, title } => {
                    self.runtime.rename_tab(tab_id, title)?
                }
                MuxCommand::FocusPane { pane_id } => self.runtime.focus_pane(pane_id)?,
                MuxCommand::ClosePane { pane_id } => self.runtime.close_pane(pane_id)?,
                MuxCommand::CloseTab { tab_id } => self.runtime.close_tab(tab_id)?,
                MuxCommand::ResizePane(spec) => self.runtime.resize_pane(spec)?,
                MuxCommand::OverrideLayout(spec) => self.runtime.override_layout(spec)?,
                MuxCommand::SendInput(spec) => self.runtime.send_input(spec)?,
                MuxCommand::SendPaste(spec) => self.runtime.send_paste(spec)?,
                MuxCommand::Detach | MuxCommand::SaveSession => {
                    return Err(BackendError::unsupported(
                        "native mux command is not wired in v1 start phase",
                        DegradedModeReason::NotYetImplemented,
                    ));
                }
            };

            Ok(MuxCommandResult { changed })
        })
    }

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let runtime = Arc::clone(&self.runtime);
        Box::pin(async move { open_native_subscription(runtime, spec) })
    }
}

fn native_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        tiled_panes: true,
        split_resize: true,
        tab_create: true,
        tab_close: true,
        tab_focus: true,
        tab_rename: true,
        session_scoped_tab_refs: true,
        session_scoped_pane_refs: true,
        pane_split: true,
        pane_close: true,
        pane_focus: true,
        pane_input_write: true,
        pane_paste_write: true,
        rendered_viewport_stream: true,
        rendered_viewport_snapshot: true,
        layout_dump: true,
        layout_override: true,
        explicit_session_save: true,
        explicit_session_restore: true,
        advisory_metadata_subscriptions: true,
        ..BackendCapabilities::default()
    }
}
