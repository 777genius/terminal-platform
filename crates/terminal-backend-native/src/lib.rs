mod emulator;
mod runtime;
mod transcript;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BackendSubscriptionEvent, BoxFuture,
    CreateSessionSpec, DiscoveredSession, MuxBackendPort, SubscriptionSpec,
};
use terminal_domain::{BackendKind, DegradedModeReason, RouteAuthority, SessionId, SessionRoute};
use tokio::sync::{mpsc, oneshot};

use runtime::NativeSessionRuntime;

#[derive(Default)]
pub struct NativeBackend {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<NativeSessionRuntime>>>>,
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
                tab_create: true,
                tab_rename: true,
                session_scoped_tab_refs: true,
                session_scoped_pane_refs: true,
                pane_input_write: true,
                pane_paste_write: true,
                rendered_viewport_stream: true,
                rendered_viewport_snapshot: true,
                advisory_metadata_subscriptions: true,
                ..BackendCapabilities::default()
            })
        })
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
            let route = SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            };
            let runtime = Arc::new(NativeSessionRuntime::spawn(session_id, route.clone(), spec)?);

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
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async move {
            if route.backend != BackendKind::Native {
                return Err(BackendError::invalid_input(
                    "native backend can only attach native routes",
                ));
            }

            let runtime = {
                let sessions = self
                    .sessions
                    .read()
                    .map_err(|_| BackendError::internal("native backend read lock poisoned"))?;
                let mut matched = None;
                for runtime in sessions.values() {
                    if runtime.summary()?.route == route {
                        matched = Some(Arc::clone(runtime));
                        break;
                    }
                }
                matched.ok_or_else(|| BackendError::not_found("native route is not registered"))?
            };

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

struct NativeAttachedSession {
    runtime: Arc<NativeSessionRuntime>,
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
        command: terminal_backend_api::MuxCommand,
    ) -> BoxFuture<'_, Result<terminal_backend_api::MuxCommandResult, BackendError>> {
        Box::pin(async move { self.runtime.dispatch(command) })
    }

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let runtime = Arc::clone(&self.runtime);
        Box::pin(async move { open_native_subscription(runtime, spec) })
    }
}

fn open_native_subscription(
    runtime: Arc<NativeSessionRuntime>,
    spec: SubscriptionSpec,
) -> Result<BackendSubscription, BackendError> {
    match spec {
        SubscriptionSpec::SessionTopology => open_topology_subscription(runtime),
        SubscriptionSpec::PaneSurface { pane_id } => {
            open_pane_surface_subscription(runtime, pane_id)
        }
    }
}

fn open_topology_subscription(
    runtime: Arc<NativeSessionRuntime>,
) -> Result<BackendSubscription, BackendError> {
    let subscription_id = terminal_domain::SubscriptionId::new();
    let initial = runtime.topology_snapshot()?;
    let mut topology_tick = runtime.subscribe_topology();
    let (events_tx, events_rx) = mpsc::channel(32);
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        if events_tx.send(BackendSubscriptionEvent::TopologySnapshot(initial)).await.is_err() {
            return;
        }

        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                changed = topology_tick.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let snapshot = match runtime.topology_snapshot() {
                        Ok(snapshot) => snapshot,
                        Err(_) => break,
                    };
                    if events_tx.send(BackendSubscriptionEvent::TopologySnapshot(snapshot)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
}

fn open_pane_surface_subscription(
    runtime: Arc<NativeSessionRuntime>,
    pane_id: terminal_domain::PaneId,
) -> Result<BackendSubscription, BackendError> {
    let subscription_id = terminal_domain::SubscriptionId::new();
    let initial = runtime.screen_snapshot(pane_id)?;
    let mut last_sequence = initial.sequence;
    let mut surface_tick = runtime.subscribe_pane_surface(pane_id)?;
    let (events_tx, events_rx) = mpsc::channel(32);
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        if events_tx
            .send(BackendSubscriptionEvent::ScreenDelta(
                terminal_projection::ScreenDelta::full_replace(0, &initial),
            ))
            .await
            .is_err()
        {
            return;
        }

        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                changed = surface_tick.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let delta = match runtime.screen_delta(pane_id, last_sequence) {
                        Ok(delta) => delta,
                        Err(_) => break,
                    };
                    if delta.to_sequence == last_sequence {
                        continue;
                    }
                    last_sequence = delta.to_sequence;
                    if events_tx.send(BackendSubscriptionEvent::ScreenDelta(delta)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use terminal_backend_api::BackendSubscriptionEvent;
    use terminal_backend_api::{
        BackendScope, CreateSessionSpec, MuxBackendPort, MuxCommand, NewTabSpec, SendInputSpec,
        ShellLaunchSpec, SubscriptionSpec,
    };
    use terminal_domain::BackendKind;
    use terminal_projection::ProjectionSource;

    use super::NativeBackend;

    #[tokio::test]
    async fn creates_and_lists_empty_sessions() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec {
                title: Some("shell".to_string()),
                ..CreateSessionSpec::default()
            })
            .await
            .expect("native session should be created");
        let sessions = backend
            .list_sessions(BackendScope::CurrentUser)
            .await
            .expect("list_sessions should succeed");

        assert_eq!(backend.kind(), BackendKind::Native);
        assert_eq!(binding.route.backend, BackendKind::Native);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, binding.session_id);
        assert_eq!(sessions[0].title.as_deref(), Some("shell"));
    }

    #[tokio::test]
    async fn attaches_and_exposes_topology_and_screen() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec {
                title: Some("shell".to_string()),
                ..CreateSessionSpec::default()
            })
            .await
            .expect("native session should be created");
        let session = backend
            .attach_session(binding.route.clone())
            .await
            .expect("attach_session should succeed");
        let topology = session.topology_snapshot().await.expect("topology should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("tab should expose a focused pane");
        let screen =
            session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
        let delta = session
            .screen_delta(pane_id, screen.sequence)
            .await
            .expect("screen delta should succeed");

        assert_eq!(topology.session_id, binding.session_id);
        assert_eq!(topology.tabs.len(), 1);
        assert_eq!(screen.pane_id, pane_id);
        assert_eq!(screen.source, ProjectionSource::NativeEmulator);
        assert!(!screen.surface.lines.is_empty());
        assert_eq!(delta.pane_id, pane_id);
        assert_eq!(delta.from_sequence, screen.sequence);
        assert_eq!(delta.to_sequence, screen.sequence);
        assert_eq!(delta.source, ProjectionSource::NativeEmulator);
        assert_eq!(delta.rows, screen.rows);
        assert_eq!(delta.cols, screen.cols);
        assert!(delta.patch.is_none());
        assert!(delta.full_replace.is_none());
    }

    #[tokio::test]
    async fn mutates_topology_through_dispatch() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec {
                title: Some("shell".to_string()),
                ..CreateSessionSpec::default()
            })
            .await
            .expect("native session should be created");
        let session =
            backend.attach_session(binding.route).await.expect("attach_session should succeed");
        let before = session.topology_snapshot().await.expect("topology should succeed");
        let focused_pane = before.tabs[0].focused_pane.expect("focused pane should exist");

        let new_tab = session
            .dispatch(MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }))
            .await
            .expect("new tab should succeed");
        let focus_same_pane = session
            .dispatch(MuxCommand::FocusPane { pane_id: focused_pane })
            .await
            .expect("focus pane should succeed");
        let after = session.topology_snapshot().await.expect("topology should succeed");

        assert!(new_tab.changed);
        assert!(focus_same_pane.changed);
        assert_eq!(after.tabs.len(), 2);
        assert_eq!(after.focused_tab, Some(before.tabs[0].tab_id));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn writes_input_into_live_pty_backed_session() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec {
                title: Some("shell".to_string()),
                launch: Some(cat_launch_spec()),
            })
            .await
            .expect("native session should be created");
        let session =
            backend.attach_session(binding.route).await.expect("attach_session should succeed");
        let topology = session.topology_snapshot().await.expect("topology should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

        wait_for_screen_line(&*session, pane_id, "ready").await;
        let before =
            session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
        let result = session
            .dispatch(MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: "hello from backend test\r".to_string(),
            }))
            .await
            .expect("send input should succeed");

        assert!(!result.changed);
        wait_for_screen_line(&*session, pane_id, "hello from backend test").await;
        let delta = session
            .screen_delta(pane_id, before.sequence)
            .await
            .expect("screen delta should succeed");
        let patch = delta.patch.expect("delta patch should exist");

        assert_eq!(delta.pane_id, pane_id);
        assert_eq!(delta.from_sequence, before.sequence);
        assert!(delta.to_sequence > before.sequence);
        assert!(
            patch
                .line_updates
                .iter()
                .any(|line| line.line.text.contains("hello from backend test"))
        );
        assert!(delta.full_replace.is_none());
    }

    #[tokio::test]
    async fn emits_screen_delta_for_tab_title_changes() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec {
                title: Some("shell".to_string()),
                ..CreateSessionSpec::default()
            })
            .await
            .expect("native session should be created");
        let session =
            backend.attach_session(binding.route).await.expect("attach_session should succeed");
        let topology = session.topology_snapshot().await.expect("topology should succeed");
        let tab_id = topology.tabs[0].tab_id;
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let before =
            session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");

        let result = session
            .dispatch(MuxCommand::RenameTab { tab_id, title: "renamed".to_string() })
            .await
            .expect("rename tab should succeed");
        let delta = session
            .screen_delta(pane_id, before.sequence)
            .await
            .expect("screen delta should succeed");
        let patch = delta.patch.expect("delta patch should exist");

        assert!(result.changed);
        assert!(delta.to_sequence > before.sequence);
        assert!(patch.title_changed);
        assert_eq!(patch.title.as_deref(), Some("renamed"));
        assert!(delta.full_replace.is_none());
    }

    #[tokio::test]
    async fn streams_initial_topology_and_new_tab_updates() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec {
                title: Some("shell".to_string()),
                ..CreateSessionSpec::default()
            })
            .await
            .expect("native session should be created");
        let session =
            backend.attach_session(binding.route).await.expect("attach_session should succeed");
        let mut subscription = session
            .subscribe(SubscriptionSpec::SessionTopology)
            .await
            .expect("topology subscription should open");

        let initial = subscription.events.recv().await.expect("initial event should arrive");
        let initial = match initial {
            BackendSubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
            other => panic!("unexpected initial event: {other:?}"),
        };
        let result = session
            .dispatch(MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }))
            .await
            .expect("new tab should succeed");
        let updated = subscription.events.recv().await.expect("topology update should arrive");
        let updated = match updated {
            BackendSubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
            other => panic!("unexpected topology event: {other:?}"),
        };

        assert_eq!(initial.tabs.len(), 1);
        assert!(result.changed);
        assert_eq!(updated.tabs.len(), 2);
    }

    #[tokio::test]
    async fn streams_initial_surface_and_title_patch_updates() {
        let backend = NativeBackend::default();
        let binding = backend
            .create_session(CreateSessionSpec {
                title: Some("shell".to_string()),
                ..CreateSessionSpec::default()
            })
            .await
            .expect("native session should be created");
        let session =
            backend.attach_session(binding.route).await.expect("attach_session should succeed");
        let topology = session.topology_snapshot().await.expect("topology should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let tab_id = topology.tabs[0].tab_id;
        let mut subscription = session
            .subscribe(SubscriptionSpec::PaneSurface { pane_id })
            .await
            .expect("pane subscription should open");

        let initial = subscription.events.recv().await.expect("initial event should arrive");
        let initial = match initial {
            BackendSubscriptionEvent::ScreenDelta(delta) => delta,
            other => panic!("unexpected initial event: {other:?}"),
        };
        let result = session
            .dispatch(MuxCommand::RenameTab { tab_id, title: "renamed".to_string() })
            .await
            .expect("rename tab should succeed");
        let updated = subscription.events.recv().await.expect("surface update should arrive");
        let updated = match updated {
            BackendSubscriptionEvent::ScreenDelta(delta) => delta,
            other => panic!("unexpected surface event: {other:?}"),
        };
        let patch = updated.patch.expect("delta patch should exist");

        assert!(initial.full_replace.is_some());
        assert!(result.changed);
        assert!(updated.to_sequence > updated.from_sequence);
        assert!(patch.title_changed);
        assert_eq!(patch.title.as_deref(), Some("renamed"));
    }

    #[cfg(unix)]
    fn cat_launch_spec() -> ShellLaunchSpec {
        ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
    }

    #[cfg(unix)]
    async fn wait_for_screen_line(
        session: &dyn terminal_backend_api::BackendSessionPort,
        pane_id: terminal_domain::PaneId,
        needle: &str,
    ) {
        for _ in 0..40 {
            let screen =
                session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
            if screen.surface.lines.iter().any(|line| line.text.contains(needle)) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }

        panic!("screen never contained expected text: {needle}");
    }
}
