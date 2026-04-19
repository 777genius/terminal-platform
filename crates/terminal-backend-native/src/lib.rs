use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BoxFuture, CreateSessionSpec, MuxBackendPort,
    MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{BackendKind, DegradedModeReason, RouteAuthority, SessionId, SessionRoute};
use terminal_domain::{PaneId, TabId};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
use terminal_projection::{
    ProjectionSource, ScreenCursor, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
};

#[derive(Default)]
pub struct NativeBackend {
    sessions: Arc<RwLock<HashMap<SessionId, NativeSessionRecord>>>,
}

#[derive(Debug, Clone)]
struct NativeSessionRecord {
    summary: BackendSessionSummary,
    tab_id: TabId,
    pane_id: PaneId,
    rows: u16,
    cols: u16,
    sequence: u64,
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
        Box::pin(async {
            Ok(BackendCapabilities {
                tiled_panes: true,
                session_scoped_tab_refs: true,
                session_scoped_pane_refs: true,
                ..BackendCapabilities::default()
            })
        })
    }

    fn create_session(
        &self,
        spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async move {
            let session_id = SessionId::new();
            let route = SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            };
            let summary =
                BackendSessionSummary { session_id, route: route.clone(), title: spec.title };
            let record = NativeSessionRecord {
                summary: summary.clone(),
                tab_id: TabId::new(),
                pane_id: PaneId::new(),
                rows: 24,
                cols: 80,
                sequence: 0,
            };

            let mut sessions =
                self.sessions.write().expect("native backend write lock should not be poisoned");
            sessions.insert(session_id, record);

            Ok(BackendSessionBinding { session_id, route })
        })
    }

    fn attach_session(
        &self,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async move {
            if route.backend != BackendKind::Native {
                return Err(BackendError::invalid_input(
                    "native backend can only attach native routes",
                ));
            }

            let session_id = {
                let sessions =
                    self.sessions.read().expect("native backend read lock should not be poisoned");
                sessions
                    .keys()
                    .copied()
                    .find(|session_id| {
                        sessions
                            .get(session_id)
                            .map(|record| record.summary.route == route)
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| BackendError::not_found("native route is not registered"))?
            };

            Ok(Box::new(NativeAttachedSession { session_id, sessions: Arc::clone(&self.sessions) })
                as Box<dyn BackendSessionPort>)
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async move {
            let sessions =
                self.sessions.read().expect("native backend read lock should not be poisoned");

            Ok(sessions.values().map(|record| record.summary.clone()).collect())
        })
    }
}

struct NativeAttachedSession {
    session_id: SessionId,
    sessions: Arc<RwLock<HashMap<SessionId, NativeSessionRecord>>>,
}

impl BackendSessionPort for NativeAttachedSession {
    fn topology_snapshot(&self) -> BoxFuture<'_, Result<TopologySnapshot, BackendError>> {
        Box::pin(async move {
            let record = self.record()?;

            Ok(TopologySnapshot {
                session_id: self.session_id,
                backend_kind: BackendKind::Native,
                tabs: vec![TabSnapshot {
                    tab_id: record.tab_id,
                    title: record.summary.title.clone(),
                    root: PaneTreeNode::Leaf { pane_id: record.pane_id },
                    focused_pane: Some(record.pane_id),
                }],
                focused_tab: Some(record.tab_id),
            })
        })
    }

    fn screen_snapshot(
        &self,
        pane_id: PaneId,
    ) -> BoxFuture<'_, Result<ScreenSnapshot, BackendError>> {
        Box::pin(async move {
            let record = self.record()?;

            if pane_id != record.pane_id {
                return Err(BackendError::not_found(format!("unknown pane {pane_id:?}")));
            }

            Ok(ScreenSnapshot {
                pane_id,
                sequence: record.sequence,
                rows: record.rows,
                cols: record.cols,
                source: ProjectionSource::NativeEmulator,
                surface: ScreenSurface {
                    title: record.summary.title.clone(),
                    cursor: Some(ScreenCursor { row: 0, col: 0 }),
                    lines: vec![
                        ScreenLine {
                            text: record
                                .summary
                                .title
                                .clone()
                                .unwrap_or_else(|| "native-session".to_string()),
                        },
                        ScreenLine { text: "native pty/emulator not wired yet".to_string() },
                    ],
                },
            })
        })
    }

    fn dispatch(
        &self,
        _command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "native mux commands are not wired in v1 start phase",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }

    fn subscribe(
        &self,
        _spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "native subscriptions are not wired in v1 start phase",
                DegradedModeReason::NotYetImplemented,
            ))
        })
    }
}

impl NativeAttachedSession {
    fn record(&self) -> Result<NativeSessionRecord, BackendError> {
        let sessions =
            self.sessions.read().expect("native attached session read lock should not be poisoned");

        sessions
            .get(&self.session_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found("native session disappeared"))
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::{BackendScope, CreateSessionSpec, MuxBackendPort};
    use terminal_projection::ProjectionSource;

    use super::NativeBackend;

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_and_lists_empty_sessions() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec { title: Some("shell".to_string()) })
            .await
            .expect("create_session should succeed");
        let sessions = backend
            .list_sessions(BackendScope::CurrentUser)
            .await
            .expect("list_sessions should succeed");

        assert_eq!(binding.route.backend, terminal_domain::BackendKind::Native);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, binding.session_id);
        assert_eq!(sessions[0].title.as_deref(), Some("shell"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn attaches_and_exposes_stub_topology_and_screen() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec { title: Some("shell".to_string()) })
            .await
            .expect("create_session should succeed");
        let session = backend
            .attach_session(binding.route.clone())
            .await
            .expect("attach_session should succeed");
        let topology = session.topology_snapshot().await.expect("topology snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let screen =
            session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");

        assert_eq!(topology.session_id, binding.session_id);
        assert_eq!(topology.backend_kind, terminal_domain::BackendKind::Native);
        assert_eq!(topology.tabs.len(), 1);
        assert_eq!(screen.pane_id, pane_id);
        assert_eq!(screen.source, ProjectionSource::NativeEmulator);
        assert_eq!(screen.surface.lines.len(), 2);
    }
}
