use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BoxFuture, CreateSessionSpec, MuxBackendPort,
    MuxCommand, MuxCommandResult, NewTabSpec, SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, PaneId, RouteAuthority, SessionId, SessionRoute, TabId,
};
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
    tabs: Vec<NativeTabRecord>,
    focused_tab: TabId,
    rows: u16,
    cols: u16,
    sequence: u64,
}

#[derive(Debug, Clone)]
struct NativeTabRecord {
    tab_id: TabId,
    title: Option<String>,
    pane_id: PaneId,
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
            let tab = make_tab(spec.title.clone());
            let summary =
                BackendSessionSummary { session_id, route: route.clone(), title: spec.title };
            let record = NativeSessionRecord {
                summary: summary.clone(),
                focused_tab: tab.tab_id,
                tabs: vec![tab],
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

            Ok(build_topology_snapshot(self.session_id, &record))
        })
    }

    fn screen_snapshot(
        &self,
        pane_id: PaneId,
    ) -> BoxFuture<'_, Result<ScreenSnapshot, BackendError>> {
        Box::pin(async move {
            let record = self.record()?;
            let tab = record
                .tabs
                .iter()
                .find(|tab| tab.pane_id == pane_id)
                .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

            Ok(ScreenSnapshot {
                pane_id,
                sequence: record.sequence,
                rows: record.rows,
                cols: record.cols,
                source: ProjectionSource::NativeEmulator,
                surface: ScreenSurface {
                    title: tab.title.clone().or_else(|| record.summary.title.clone()),
                    cursor: Some(ScreenCursor { row: 0, col: 0 }),
                    lines: vec![
                        ScreenLine {
                            text: tab
                                .title
                                .clone()
                                .or_else(|| record.summary.title.clone())
                                .unwrap_or_else(|| "native-session".to_string()),
                        },
                        ScreenLine { text: format!("pane {:?} in stub native session", pane_id) },
                    ],
                },
            })
        })
    }

    fn dispatch(
        &self,
        command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        Box::pin(async move {
            let mut sessions = self
                .sessions
                .write()
                .expect("native attached session write lock should not be poisoned");
            let record = sessions
                .get_mut(&self.session_id)
                .ok_or_else(|| BackendError::not_found("native session disappeared"))?;

            let changed = match command {
                MuxCommand::NewTab(spec) => dispatch_new_tab(record, spec),
                MuxCommand::FocusTab { tab_id } => dispatch_focus_tab(record, tab_id)?,
                MuxCommand::RenameTab { tab_id, title } => {
                    dispatch_rename_tab(record, tab_id, title)?
                }
                MuxCommand::FocusPane { pane_id } => dispatch_focus_pane(record, pane_id)?,
                MuxCommand::CloseTab { tab_id } => dispatch_close_tab(record, tab_id)?,
                MuxCommand::ClosePane { .. }
                | MuxCommand::SplitPane(_)
                | MuxCommand::ResizePane(_)
                | MuxCommand::SendInput(_)
                | MuxCommand::SendPaste(_)
                | MuxCommand::Detach
                | MuxCommand::SaveSession
                | MuxCommand::OverrideLayout(_) => {
                    return Err(BackendError::unsupported(
                        "native mux command is not wired in v1 start phase",
                        DegradedModeReason::NotYetImplemented,
                    ));
                }
            };

            if changed {
                record.sequence += 1;
            }

            Ok(MuxCommandResult { changed })
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

fn make_tab(title: Option<String>) -> NativeTabRecord {
    NativeTabRecord { tab_id: TabId::new(), title, pane_id: PaneId::new() }
}

fn build_topology_snapshot(
    session_id: SessionId,
    record: &NativeSessionRecord,
) -> TopologySnapshot {
    TopologySnapshot {
        session_id,
        backend_kind: BackendKind::Native,
        tabs: record
            .tabs
            .iter()
            .map(|tab| TabSnapshot {
                tab_id: tab.tab_id,
                title: tab.title.clone(),
                root: PaneTreeNode::Leaf { pane_id: tab.pane_id },
                focused_pane: Some(tab.pane_id),
            })
            .collect(),
        focused_tab: Some(record.focused_tab),
    }
}

fn dispatch_new_tab(record: &mut NativeSessionRecord, spec: NewTabSpec) -> bool {
    let tab = make_tab(spec.title);
    record.focused_tab = tab.tab_id;
    record.tabs.push(tab);
    true
}

fn dispatch_focus_tab(
    record: &mut NativeSessionRecord,
    tab_id: TabId,
) -> Result<bool, BackendError> {
    if !record.tabs.iter().any(|tab| tab.tab_id == tab_id) {
        return Err(BackendError::not_found(format!("unknown tab {tab_id:?}")));
    }

    if record.focused_tab == tab_id {
        return Ok(false);
    }

    record.focused_tab = tab_id;
    Ok(true)
}

fn dispatch_rename_tab(
    record: &mut NativeSessionRecord,
    tab_id: TabId,
    title: String,
) -> Result<bool, BackendError> {
    let tab = record
        .tabs
        .iter_mut()
        .find(|tab| tab.tab_id == tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {tab_id:?}")))?;

    if tab.title.as_deref() == Some(title.as_str()) {
        return Ok(false);
    }

    tab.title = Some(title);
    Ok(true)
}

fn dispatch_focus_pane(
    record: &mut NativeSessionRecord,
    pane_id: PaneId,
) -> Result<bool, BackendError> {
    let tab = record
        .tabs
        .iter()
        .find(|tab| tab.pane_id == pane_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

    if record.focused_tab == tab.tab_id {
        return Ok(false);
    }

    record.focused_tab = tab.tab_id;
    Ok(true)
}

fn dispatch_close_tab(
    record: &mut NativeSessionRecord,
    tab_id: TabId,
) -> Result<bool, BackendError> {
    if record.tabs.len() == 1 {
        return Err(BackendError::unsupported(
            "cannot close the last tab in v1 start phase",
            DegradedModeReason::NotYetImplemented,
        ));
    }

    let index = record
        .tabs
        .iter()
        .position(|tab| tab.tab_id == tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {tab_id:?}")))?;

    record.tabs.remove(index);

    if record.focused_tab == tab_id {
        let next_index = index.saturating_sub(1).min(record.tabs.len() - 1);
        record.focused_tab = record.tabs[next_index].tab_id;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::{
        BackendScope, CreateSessionSpec, MuxBackendPort, MuxCommand, NewTabSpec,
    };
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

    #[tokio::test(flavor = "multi_thread")]
    async fn mutates_topology_through_dispatch() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec { title: Some("shell".to_string()) })
            .await
            .expect("create_session should succeed");
        let session = backend
            .attach_session(binding.route.clone())
            .await
            .expect("attach_session should succeed");
        let before = session.topology_snapshot().await.expect("topology snapshot should succeed");

        let created = session
            .dispatch(MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }))
            .await
            .expect("new tab dispatch should succeed");
        let after_new_tab =
            session.topology_snapshot().await.expect("topology snapshot should succeed");
        let new_tab_id = after_new_tab.focused_tab.expect("focused tab should exist");
        let renamed = session
            .dispatch(MuxCommand::RenameTab { tab_id: new_tab_id, title: "console".to_string() })
            .await
            .expect("rename tab dispatch should succeed");
        let after_rename =
            session.topology_snapshot().await.expect("topology snapshot should succeed");

        assert!(created.changed);
        assert!(renamed.changed);
        assert_eq!(before.tabs.len(), 1);
        assert_eq!(after_new_tab.tabs.len(), 2);
        assert_eq!(
            after_rename
                .tabs
                .iter()
                .find(|tab| tab.tab_id == new_tab_id)
                .and_then(|tab| tab.title.as_deref()),
            Some("console")
        );
    }
}
