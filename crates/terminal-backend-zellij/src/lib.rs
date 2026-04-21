use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    process::{Command, Stdio},
    sync::{Arc, Mutex as StdMutex},
    thread,
    time::Instant,
};

use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BackendSubscriptionEvent, BoxFuture,
    CreateSessionSpec, DiscoveredSession, MuxBackendPort, MuxCommand, MuxCommandResult, NewTabSpec,
    SendInputSpec, SendPasteSpec, SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, ExternalSessionRef, PaneId, RouteAuthority, SessionId,
    SessionRoute, TabId, imported_session_id,
};
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};
use terminal_projection::{
    ProjectionSource, ScreenDelta, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command as TokioCommand,
    sync::{Mutex, mpsc, oneshot},
    time::{self, Duration, MissedTickBehavior},
};
use uuid::Uuid;

const ZELLIJ_ROUTE_NAMESPACE: &str = "zellij_session";
const ZELLIJ_POLL_INTERVAL: Duration = Duration::from_millis(100);
const ZELLIJ_TRANSIENT_RETRY_ATTEMPTS: usize = 2;
const ZELLIJ_ACTION_SETTLE_ATTEMPTS: usize = 600;
const ZELLIJ_ACTION_SETTLE_TIMEOUT: Duration = Duration::from_secs(15);
const ZELLIJ_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const ZELLIJ_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Default)]
pub struct ZellijBackend;

impl ZellijBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }

    fn run(&self, target: Option<&ZellijTarget>, args: &[&str]) -> Result<String, BackendError> {
        let mut last_error = None;
        for attempt in 0..ZELLIJ_TRANSIENT_RETRY_ATTEMPTS {
            let mut command = Command::new("zellij");
            if let Some(target) = target {
                command.arg("--session").arg(&target.session_name);
            }
            command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

            let mut child = command.spawn().map_err(|error| {
                BackendError::transport(format!("zellij command failed to spawn: {error}"))
            })?;
            let started = Instant::now();

            let output = loop {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        break child.wait_with_output().map_err(|error| {
                            BackendError::transport(format!(
                                "zellij command output collection failed: {error}"
                            ))
                        })?;
                    }
                    Ok(None) => {
                        if started.elapsed() >= ZELLIJ_COMMAND_TIMEOUT {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err(BackendError::transport(format!(
                                "zellij command timed out after {} ms: zellij {}",
                                ZELLIJ_COMMAND_TIMEOUT.as_millis(),
                                args.join(" ")
                            )));
                        }
                        thread::sleep(ZELLIJ_COMMAND_POLL_INTERVAL);
                    }
                    Err(error) => {
                        return Err(BackendError::transport(format!(
                            "zellij command wait failed: {error}"
                        )));
                    }
                }
            };
            if output.status.success() {
                return String::from_utf8(output.stdout).map_err(|error| {
                    BackendError::internal(format!("zellij output is not utf8: {error}"))
                });
            }

            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let error = BackendError::transport(format!("zellij command failed: {stderr}"));
            if attempt + 1 < ZELLIJ_TRANSIENT_RETRY_ATTEMPTS && is_transient_zellij_error(&stderr) {
                last_error = Some(error);
                thread::sleep(ZELLIJ_POLL_INTERVAL);
                continue;
            }
            return Err(error);
        }

        Err(last_error.unwrap_or_else(|| {
            BackendError::transport("zellij command never reached a stable result")
        }))
    }

    fn run_owned(
        &self,
        target: Option<&ZellijTarget>,
        args: &[String],
    ) -> Result<String, BackendError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(target, &refs)
    }

    fn spawn_subscribe(
        &self,
        target: &ZellijTarget,
        pane_ref: &str,
    ) -> Result<tokio::process::Child, BackendError> {
        let mut command = TokioCommand::new("zellij");
        command
            .arg("--session")
            .arg(&target.session_name)
            .arg("subscribe")
            .arg("--pane-id")
            .arg(pane_ref)
            .arg("--format")
            .arg("json")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        command.spawn().map_err(|error| {
            BackendError::transport(format!("zellij subscribe failed to spawn: {error}"))
        })
    }

    fn probe(&self) -> Result<ZellijProbe, BackendError> {
        let version_output = self.run(None, &["--version"])?;
        let root_help = self.run(None, &["--help"]).ok();
        let action_help = self.run(None, &["action", "--help"]).ok();

        Ok(ZellijProbe::parse(&version_output, root_help.as_deref(), action_help.as_deref()))
    }
}

impl MuxBackendPort for ZellijBackend {
    fn kind(&self) -> BackendKind {
        self.kind()
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async move {
            let probe = self.probe()?;
            Ok(match probe.surface {
                ZellijSurface::RichCli044Plus => BackendCapabilities {
                    tiled_panes: true,
                    tab_create: true,
                    tab_close: true,
                    tab_focus: true,
                    tab_rename: true,
                    session_scoped_tab_refs: true,
                    session_scoped_pane_refs: true,
                    pane_close: true,
                    pane_focus: true,
                    pane_input_write: true,
                    pane_paste_write: true,
                    rendered_viewport_stream: true,
                    rendered_viewport_snapshot: true,
                    plugin_panes: true,
                    advisory_metadata_subscriptions: true,
                    read_only_client_mode: true,
                    ..BackendCapabilities::default()
                },
                ZellijSurface::LegacyCli043 => BackendCapabilities {
                    read_only_client_mode: true,
                    ..BackendCapabilities::default()
                },
                ZellijSurface::Unknown => BackendCapabilities::default(),
            })
        })
    }

    fn discover_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async move {
            let output = match self.run(None, &["list-sessions", "--short", "--no-formatting"]) {
                Ok(output) => output,
                Err(error) if is_transient_zellij_backend_error(&error) => return Ok(Vec::new()),
                Err(error) => return Err(error),
            };
            let sessions = output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && *line != "No active zellij sessions found.")
                .map(|session_name| {
                    let route = SessionRoute {
                        backend: BackendKind::Zellij,
                        authority: RouteAuthority::ImportedForeign,
                        external: Some(ExternalSessionRef {
                            namespace: ZELLIJ_ROUTE_NAMESPACE.to_string(),
                            value: format!("session={session_name}"),
                        }),
                    };

                    DiscoveredSession { route, title: Some(session_name.to_string()) }
                })
                .collect();

            Ok(sessions)
        })
    }

    fn create_session(
        &self,
        _spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij sessions are imported, not created",
                DegradedModeReason::ImportedForeignSession,
            ))
        })
    }

    fn attach_session(
        &self,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        let backend = self.clone();
        Box::pin(async move {
            let target = ZellijTarget::from_route(&route)?;
            let probe = backend.probe()?;
            let sessions = backend.discover_sessions(BackendScope::CurrentUser).await?;
            if !sessions.iter().any(|session| session.route == route) {
                return Err(BackendError::not_found(format!(
                    "zellij session '{}' is not active",
                    target.session_name
                )));
            }

            match probe.surface {
                ZellijSurface::RichCli044Plus => {
                    let session_id = imported_session_id(&route).ok_or_else(|| {
                        BackendError::invalid_input("zellij route is not importable")
                    })?;
                    let attached = ZellijAttachedSession {
                        backend: Arc::new(backend),
                        session_id,
                        target,
                        io_lane: Arc::new(StdMutex::new(())),
                        command_lane: Arc::new(Mutex::new(())),
                    };
                    attached.snapshot()?;

                    Ok(Box::new(attached) as Box<dyn BackendSessionPort>)
                }
                ZellijSurface::LegacyCli043 => Err(BackendError::unsupported(
                    format!(
                        "zellij {} does not expose the list-panes/list-tabs/subscribe surface required for imported attach",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                )),
                ZellijSurface::Unknown => Err(BackendError::unsupported(
                    format!(
                        "zellij {} could not be matched to a supported control surface",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                )),
            }
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij backend does not expose canonical sessions directly",
                DegradedModeReason::ImportedForeignSession,
            ))
        })
    }
}

#[derive(Clone)]
struct ZellijAttachedSession {
    backend: Arc<ZellijBackend>,
    session_id: SessionId,
    target: ZellijTarget,
    io_lane: Arc<StdMutex<()>>,
    command_lane: Arc<Mutex<()>>,
}

impl BackendSessionPort for ZellijAttachedSession {
    fn topology_snapshot(&self) -> BoxFuture<'_, Result<TopologySnapshot, BackendError>> {
        Box::pin(async move { Ok(self.snapshot()?.topology) })
    }

    fn screen_snapshot(
        &self,
        pane_id: PaneId,
    ) -> BoxFuture<'_, Result<ScreenSnapshot, BackendError>> {
        Box::pin(async move { self.screen_snapshot_inner(pane_id) })
    }

    fn screen_delta(
        &self,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> BoxFuture<'_, Result<ScreenDelta, BackendError>> {
        Box::pin(async move {
            let current = self.screen_snapshot_inner(pane_id)?;
            if current.sequence == from_sequence {
                Ok(ScreenDelta::unchanged_from(&current))
            } else {
                Ok(ScreenDelta::full_replace(from_sequence, &current))
            }
        })
    }

    fn dispatch(
        &self,
        command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        let session = self.clone();
        Box::pin(async move { session.dispatch_inner(command).await })
    }

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let session = self.clone();
        Box::pin(async move { session.open_subscription(spec) })
    }
}

impl ZellijAttachedSession {
    async fn dispatch_inner(&self, command: MuxCommand) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let actions = self.dispatch_actions(&snapshot, command)?;
        if actions.is_empty() {
            return Ok(MuxCommandResult { changed: false });
        }

        let _permit = self.command_lane.lock().await;
        let mut settled_snapshot = snapshot.clone();
        for action in actions {
            let _io_permit = self.io_lane.lock().expect("zellij io lane should not be poisoned");
            self.backend.run_owned(Some(&self.target), &action.args())?;
            drop(_io_permit);
            if action.requires_settle() {
                settled_snapshot = self.wait_for_action_settle(&settled_snapshot, &action).await?;
            }
        }

        Ok(MuxCommandResult { changed: true })
    }

    fn open_subscription(
        &self,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        match spec {
            SubscriptionSpec::SessionTopology => self.open_topology_subscription(),
            SubscriptionSpec::PaneSurface { pane_id } => {
                self.open_pane_surface_subscription(pane_id)
            }
        }
    }

    fn open_topology_subscription(&self) -> Result<BackendSubscription, BackendError> {
        let subscription_id = terminal_domain::SubscriptionId::new();
        let session = self.clone();
        let initial = session.snapshot()?.topology;
        let (events_tx, events_rx) = mpsc::channel(32);
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            if events_tx
                .send(BackendSubscriptionEvent::TopologySnapshot(initial.clone()))
                .await
                .is_err()
            {
                return;
            }

            let mut last = initial;
            let mut ticker = time::interval(ZELLIJ_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    _ = ticker.tick() => {
                        let Ok(command_idle) = session.command_lane.try_lock() else {
                            continue;
                        };
                        drop(command_idle);
                        let current = match session.snapshot() {
                            Ok(snapshot) => snapshot.topology,
                            Err(_) => break,
                        };
                        if current == last {
                            continue;
                        }
                        last = current.clone();
                        if events_tx.send(BackendSubscriptionEvent::TopologySnapshot(current)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
    }

    fn open_pane_surface_subscription(
        &self,
        pane_id: PaneId,
    ) -> Result<BackendSubscription, BackendError> {
        let subscription_id = terminal_domain::SubscriptionId::new();
        let session = self.clone();
        let pane_target = session.pane_target(pane_id)?;
        let initial = session.screen_snapshot_inner(pane_id)?;
        let (events_tx, events_rx) = mpsc::channel(32);
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            if events_tx
                .send(BackendSubscriptionEvent::ScreenDelta(ScreenDelta::full_replace(0, &initial)))
                .await
                .is_err()
            {
                return;
            }

            let mut child =
                match session.backend.spawn_subscribe(&session.target, &pane_target.backend_ref) {
                    Ok(child) => child,
                    Err(_) => return,
                };
            let Some(stdout) = child.stdout.take() else {
                let _ = child.start_kill();
                let _ = child.wait().await;
                return;
            };
            let mut lines = BufReader::new(stdout).lines();
            let mut last = initial;

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    next_line = lines.next_line() => {
                        match next_line {
                            Ok(Some(line)) => {
                                if line.trim().is_empty() {
                                    continue;
                                }
                                let event = match serde_json::from_str::<ZellijSubscribeEvent>(&line) {
                                    Ok(event) => event,
                                    Err(_) => break,
                                };
                                match event {
                                    ZellijSubscribeEvent::PaneUpdate { pane_id: updated_pane_ref, viewport, is_initial, .. } => {
                                        if updated_pane_ref != pane_target.backend_ref || is_initial {
                                            continue;
                                        }
                                        let current = match session.screen_snapshot_from_viewport(
                                            pane_id,
                                            viewport,
                                            ProjectionSource::ZellijViewportSubscribe,
                                        ) {
                                            Ok(snapshot) => snapshot,
                                            Err(_) => break,
                                        };
                                        if current.sequence == last.sequence {
                                            continue;
                                        }
                                        let delta = ScreenDelta::between(&last, &current);
                                        last = current;
                                        if events_tx.send(BackendSubscriptionEvent::ScreenDelta(delta)).await.is_err() {
                                            break;
                                        }
                                    }
                                    ZellijSubscribeEvent::PaneClosed { pane_id: closed_pane_ref } => {
                                        if closed_pane_ref == pane_target.backend_ref {
                                            break;
                                        }
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
            }

            let _ = child.start_kill();
            let _ = child.wait().await;
        });

        Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
    }

    fn snapshot(&self) -> Result<ZellijSessionSnapshot, BackendError> {
        let mut last_error = None;
        for attempt in 0..ZELLIJ_TRANSIENT_RETRY_ATTEMPTS {
            let snapshot_outputs: Result<(String, String), BackendError> = (|| {
                let _io_permit =
                    self.io_lane.lock().expect("zellij io lane should not be poisoned");
                let tabs_output =
                    self.backend.run(Some(&self.target), &["action", "list-tabs", "--json"])?;
                let panes_output =
                    self.backend.run(Some(&self.target), &["action", "list-panes", "--json"])?;
                Ok((tabs_output, panes_output))
            })();
            let (tabs_output, panes_output) = match snapshot_outputs {
                Ok(outputs) => outputs,
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
                Err(error) => return Err(error),
            };

            if tabs_output.trim().is_empty() || panes_output.trim().is_empty() {
                last_error = Some(BackendError::internal(
                    "zellij snapshot commands returned empty output while the session was still settling",
                ));
                if attempt + 1 < ZELLIJ_TRANSIENT_RETRY_ATTEMPTS {
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
            }

            let tabs = match parse_tabs_json(&tabs_output) {
                Ok(tabs) => tabs,
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
                Err(error) => return Err(error),
            };
            let panes = match parse_panes_json(&panes_output) {
                Ok(panes) => panes,
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                    thread::sleep(ZELLIJ_POLL_INTERVAL);
                    continue;
                }
                Err(error) => return Err(error),
            };

            return build_session_snapshot(self.session_id, &self.target, &tabs, &panes);
        }

        Err(last_error.unwrap_or_else(|| {
            BackendError::transport("zellij snapshot never stabilized after retries")
        }))
    }

    fn pane_target(&self, pane_id: PaneId) -> Result<ZellijPaneTarget, BackendError> {
        self.snapshot()?
            .pane_targets
            .get(&pane_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij pane {pane_id:?}")))
    }

    fn dispatch_actions(
        &self,
        snapshot: &ZellijSessionSnapshot,
        command: MuxCommand,
    ) -> Result<Vec<ZellijAction>, BackendError> {
        match command {
            MuxCommand::NewTab(spec) => Ok(self.new_tab_actions(spec)),
            MuxCommand::SendInput(spec) => self.send_input_actions(snapshot, spec),
            MuxCommand::SendPaste(spec) => self.send_paste_actions(snapshot, spec),
            MuxCommand::FocusPane { pane_id } => {
                Ok(vec![self.focus_pane_action(snapshot, pane_id)?])
            }
            MuxCommand::ClosePane { pane_id } => {
                Ok(vec![self.close_pane_action(snapshot, pane_id)?])
            }
            MuxCommand::FocusTab { tab_id } => Ok(vec![self.focus_tab_action(snapshot, tab_id)?]),
            MuxCommand::CloseTab { tab_id } => Ok(vec![self.close_tab_action(snapshot, tab_id)?]),
            MuxCommand::RenameTab { tab_id, title } => {
                Ok(vec![self.rename_tab_action(snapshot, tab_id, &title)?])
            }
            MuxCommand::SplitPane(_)
            | MuxCommand::ResizePane(_)
            | MuxCommand::Detach
            | MuxCommand::SaveSession
            | MuxCommand::OverrideLayout(_) => Err(BackendError::unsupported(
                "zellij imported routes do not support this command in the current rollout phase",
                DegradedModeReason::UnsupportedByBackend,
            )),
        }
    }

    fn new_tab_actions(&self, spec: NewTabSpec) -> Vec<ZellijAction> {
        vec![ZellijAction::NewTab { title: spec.title }]
    }

    fn focus_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
    ) -> Result<ZellijAction, BackendError> {
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;
        Ok(ZellijAction::FocusTab { backend_tab_id: tab_target.backend_tab_id })
    }

    fn close_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
    ) -> Result<ZellijAction, BackendError> {
        if snapshot.topology.tabs.len() <= 1 {
            return Err(BackendError::unsupported(
                "zellij imported routes refuse to close the last tab because it would terminate the foreign session",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;

        Ok(ZellijAction::CloseTab { backend_tab_id: tab_target.backend_tab_id })
    }

    fn rename_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
        title: &str,
    ) -> Result<ZellijAction, BackendError> {
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;
        Ok(ZellijAction::RenameTab {
            backend_tab_id: tab_target.backend_tab_id,
            title: title.to_string(),
        })
    }

    fn focus_pane_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        pane_id: PaneId,
    ) -> Result<ZellijAction, BackendError> {
        let pane_target =
            snapshot.pane_targets.get(&pane_id).cloned().ok_or_else(|| {
                BackendError::not_found(format!("unknown zellij pane {pane_id:?}"))
            })?;
        Ok(ZellijAction::FocusPane { pane_ref: pane_target.backend_ref })
    }

    fn close_pane_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        pane_id: PaneId,
    ) -> Result<ZellijAction, BackendError> {
        let pane_target =
            snapshot.pane_targets.get(&pane_id).cloned().ok_or_else(|| {
                BackendError::not_found(format!("unknown zellij pane {pane_id:?}"))
            })?;
        let tab = snapshot
            .topology
            .tabs
            .iter()
            .find(|tab| tab_contains_pane(tab, pane_id))
            .ok_or_else(|| {
                BackendError::not_found(format!("zellij pane {pane_id:?} is not bound to a tab"))
            })?;
        if collect_pane_ids(&tab.root).len() <= 1 {
            return Err(BackendError::unsupported(
                "zellij imported routes refuse to close the last pane in a tab because it would collapse tab lifecycle into tab closure semantics",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        Ok(ZellijAction::ClosePane { pane_ref: pane_target.backend_ref })
    }

    fn send_input_actions(
        &self,
        snapshot: &ZellijSessionSnapshot,
        spec: SendInputSpec,
    ) -> Result<Vec<ZellijAction>, BackendError> {
        if spec.data.is_empty() {
            return Ok(Vec::new());
        }

        let pane_target = snapshot.pane_targets.get(&spec.pane_id).cloned().ok_or_else(|| {
            BackendError::not_found(format!("unknown zellij pane {:?}", spec.pane_id))
        })?;
        if pane_target.kind != ZellijPaneKind::Terminal {
            return Err(BackendError::unsupported(
                "zellij input writes target terminal panes only",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        let mut actions = Vec::new();
        let mut literal = String::new();
        for ch in spec.data.chars() {
            match ch {
                '\r' | '\n' => {
                    flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);
                    actions.push(ZellijAction::SendKeys {
                        pane_ref: pane_target.backend_ref.clone(),
                        keys: vec!["Enter".to_string()],
                    });
                }
                '\t' => {
                    flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);
                    actions.push(ZellijAction::SendKeys {
                        pane_ref: pane_target.backend_ref.clone(),
                        keys: vec!["Tab".to_string()],
                    });
                }
                c if c.is_control() => {
                    return Err(BackendError::unsupported(
                        format!("zellij input path does not support control character {:?}", c),
                        DegradedModeReason::UnsupportedByBackend,
                    ));
                }
                c => literal.push(c),
            }
        }
        flush_zellij_literal(&pane_target.backend_ref, &mut literal, &mut actions);

        Ok(actions)
    }

    fn send_paste_actions(
        &self,
        snapshot: &ZellijSessionSnapshot,
        spec: SendPasteSpec,
    ) -> Result<Vec<ZellijAction>, BackendError> {
        if spec.data.is_empty() {
            return Ok(Vec::new());
        }

        let pane_target = snapshot.pane_targets.get(&spec.pane_id).cloned().ok_or_else(|| {
            BackendError::not_found(format!("unknown zellij pane {:?}", spec.pane_id))
        })?;
        if pane_target.kind != ZellijPaneKind::Terminal {
            return Err(BackendError::unsupported(
                "zellij paste writes target terminal panes only",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        Ok(vec![ZellijAction::Paste { pane_ref: pane_target.backend_ref, text: spec.data }])
    }

    fn screen_snapshot_inner(&self, pane_id: PaneId) -> Result<ScreenSnapshot, BackendError> {
        let pane_target = self.pane_target(pane_id)?;
        let _io_permit = self.io_lane.lock().expect("zellij io lane should not be poisoned");
        let output = self.backend.run_owned(
            Some(&self.target),
            &[
                "action".to_string(),
                "dump-screen".to_string(),
                "--pane-id".to_string(),
                pane_target.backend_ref.clone(),
            ],
        )?;

        Ok(screen_snapshot_from_lines(
            pane_id,
            &pane_target,
            screen_lines_from_output(&output),
            ProjectionSource::ZellijDumpSnapshot,
        ))
    }

    fn screen_snapshot_from_viewport(
        &self,
        pane_id: PaneId,
        viewport: Vec<String>,
        source: ProjectionSource,
    ) -> Result<ScreenSnapshot, BackendError> {
        let pane_target = self.pane_target(pane_id)?;
        let lines = viewport.into_iter().map(|text| ScreenLine { text }).collect();

        Ok(screen_snapshot_from_lines(pane_id, &pane_target, lines, source))
    }

    async fn wait_for_action_settle(
        &self,
        previous: &ZellijSessionSnapshot,
        action: &ZellijAction,
    ) -> Result<ZellijSessionSnapshot, BackendError> {
        let mut last_error = None;
        let started = Instant::now();
        for _ in 0..ZELLIJ_ACTION_SETTLE_ATTEMPTS {
            match self.snapshot() {
                Ok(snapshot) if action.settled(previous, &snapshot) => return Ok(snapshot),
                Ok(_) => {}
                Err(error) if is_transient_zellij_backend_error(&error) => {
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            }
            if started.elapsed() >= ZELLIJ_ACTION_SETTLE_TIMEOUT {
                break;
            }
            time::sleep(ZELLIJ_POLL_INTERVAL).await;
        }

        Err(last_error.unwrap_or_else(|| {
            BackendError::transport(format!(
                "zellij action did not settle within {} ms",
                ZELLIJ_ACTION_SETTLE_TIMEOUT.as_millis()
            ))
        }))
    }
}

#[derive(Clone)]
struct ZellijSessionSnapshot {
    topology: TopologySnapshot,
    tab_targets: HashMap<TabId, ZellijTabTarget>,
    pane_targets: HashMap<PaneId, ZellijPaneTarget>,
}

impl ZellijSessionSnapshot {
    fn focused_backend_tab_id(&self) -> Option<u32> {
        self.topology
            .focused_tab
            .and_then(|tab_id| self.tab_targets.get(&tab_id))
            .map(|tab| tab.backend_tab_id)
    }

    fn tab_exists(&self, backend_tab_id: u32) -> bool {
        self.tab_targets.values().any(|tab| tab.backend_tab_id == backend_tab_id)
    }

    fn tab_title(&self, backend_tab_id: u32) -> Option<&str> {
        self.tab_targets
            .values()
            .find(|tab| tab.backend_tab_id == backend_tab_id)
            .and_then(|tab| tab.title.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZellijTarget {
    session_name: String,
}

impl ZellijTarget {
    fn from_route(route: &SessionRoute) -> Result<Self, BackendError> {
        if route.authority != RouteAuthority::ImportedForeign {
            return Err(BackendError::invalid_input("zellij route must be imported_foreign"));
        }
        let external = route.external.as_ref().ok_or_else(|| {
            BackendError::invalid_input("zellij route is missing external reference")
        })?;
        if external.namespace != ZELLIJ_ROUTE_NAMESPACE {
            return Err(BackendError::invalid_input("zellij route namespace is invalid"));
        }
        let session_name = external
            .value
            .strip_prefix("session=")
            .ok_or_else(|| BackendError::invalid_input("zellij route is missing session"))?;

        Ok(Self { session_name: session_name.to_string() })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZellijProbe {
    version: String,
    surface: ZellijSurface,
}

impl ZellijProbe {
    fn parse(version_output: &str, root_help: Option<&str>, action_help: Option<&str>) -> Self {
        let version = version_output.trim().to_string();
        let parsed = version.split_whitespace().find_map(parse_semver_triplet).unwrap_or((0, 0, 0));
        let surface = classify_surface(parsed, root_help, action_help);

        Self { version, surface }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZellijSurface {
    LegacyCli043,
    RichCli044Plus,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct ZellijTabRow {
    tab_id: u32,
    #[serde(default)]
    position: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    active: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct ZellijPaneRow {
    id: u32,
    tab_id: u32,
    #[serde(default)]
    title: String,
    #[serde(default)]
    is_plugin: bool,
    #[serde(default)]
    is_focused: bool,
    #[serde(default)]
    is_floating: bool,
    #[serde(default)]
    pane_x: u16,
    #[serde(default)]
    pane_y: u16,
    pane_rows: u16,
    pane_columns: u16,
}

impl ZellijPaneRow {
    fn backend_ref(&self) -> String {
        if self.is_plugin { format!("plugin_{}", self.id) } else { format!("terminal_{}", self.id) }
    }
}

#[derive(Debug, Clone)]
struct ZellijPaneTarget {
    backend_ref: String,
    kind: ZellijPaneKind,
    title: Option<String>,
    rows: u16,
    cols: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZellijPaneKind {
    Terminal,
    Plugin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZellijTabTarget {
    backend_tab_id: u32,
    position: u32,
    title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ZellijAction {
    NewTab { title: Option<String> },
    FocusTab { backend_tab_id: u32 },
    CloseTab { backend_tab_id: u32 },
    RenameTab { backend_tab_id: u32, title: String },
    FocusPane { pane_ref: String },
    ClosePane { pane_ref: String },
    WriteChars { pane_ref: String, chars: String },
    Paste { pane_ref: String, text: String },
    SendKeys { pane_ref: String, keys: Vec<String> },
}

impl ZellijAction {
    fn requires_settle(&self) -> bool {
        matches!(
            self,
            Self::NewTab { .. }
                | Self::FocusTab { .. }
                | Self::CloseTab { .. }
                | Self::RenameTab { .. }
                | Self::FocusPane { .. }
                | Self::ClosePane { .. }
        )
    }

    fn settled(&self, previous: &ZellijSessionSnapshot, current: &ZellijSessionSnapshot) -> bool {
        match self {
            Self::NewTab { title } => {
                current.topology.tabs.len() > previous.topology.tabs.len()
                    && title.as_ref().is_none_or(|title| {
                        current
                            .topology
                            .tabs
                            .iter()
                            .any(|tab| tab.title.as_deref() == Some(title.as_str()))
                    })
            }
            Self::FocusTab { backend_tab_id } => {
                current.focused_backend_tab_id() == Some(*backend_tab_id)
            }
            Self::CloseTab { backend_tab_id } => !current.tab_exists(*backend_tab_id),
            Self::RenameTab { backend_tab_id, title } => {
                current.tab_title(*backend_tab_id) == Some(title.as_str())
            }
            Self::FocusPane { pane_ref } => current
                .topology
                .tabs
                .iter()
                .find(|tab| Some(tab.tab_id) == current.topology.focused_tab)
                .and_then(|tab| tab.focused_pane)
                .and_then(|pane_id| current.pane_targets.get(&pane_id))
                .map(|pane| pane.backend_ref == *pane_ref)
                .unwrap_or(false),
            Self::ClosePane { pane_ref } => {
                !current.pane_targets.values().any(|pane| pane.backend_ref == *pane_ref)
            }
            Self::WriteChars { .. } | Self::Paste { .. } | Self::SendKeys { .. } => true,
        }
    }

    fn args(&self) -> Vec<String> {
        match self {
            Self::NewTab { title } => {
                let mut args = vec!["action".to_string(), "new-tab".to_string()];
                if let Some(title) = title {
                    args.push("--name".to_string());
                    args.push(title.clone());
                }
                args
            }
            Self::FocusTab { backend_tab_id } => vec![
                "action".to_string(),
                "go-to-tab-by-id".to_string(),
                backend_tab_id.to_string(),
            ],
            Self::CloseTab { backend_tab_id } => vec![
                "action".to_string(),
                "close-tab".to_string(),
                "--tab-id".to_string(),
                backend_tab_id.to_string(),
            ],
            Self::RenameTab { backend_tab_id, title } => vec![
                "action".to_string(),
                "rename-tab".to_string(),
                "--tab-id".to_string(),
                backend_tab_id.to_string(),
                title.clone(),
            ],
            Self::FocusPane { pane_ref } => {
                vec!["action".to_string(), "focus-pane-id".to_string(), pane_ref.clone()]
            }
            Self::ClosePane { pane_ref } => vec![
                "action".to_string(),
                "close-pane".to_string(),
                "--pane-id".to_string(),
                pane_ref.clone(),
            ],
            Self::WriteChars { pane_ref, chars } => vec![
                "action".to_string(),
                "write-chars".to_string(),
                "--pane-id".to_string(),
                pane_ref.clone(),
                chars.clone(),
            ],
            Self::Paste { pane_ref, text } => vec![
                "action".to_string(),
                "paste".to_string(),
                "--pane-id".to_string(),
                pane_ref.clone(),
                text.clone(),
            ],
            Self::SendKeys { pane_ref, keys } => {
                let mut args = vec![
                    "action".to_string(),
                    "send-keys".to_string(),
                    "--pane-id".to_string(),
                    pane_ref.clone(),
                ];
                args.extend(keys.clone());
                args
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ZellijSubscribeEvent {
    PaneUpdate {
        pane_id: String,
        viewport: Vec<String>,
        #[serde(default)]
        _scrollback: Option<Vec<String>>,
        #[serde(default)]
        is_initial: bool,
    },
    PaneClosed {
        pane_id: String,
    },
}

fn parse_tabs_json(output: &str) -> Result<Vec<ZellijTabRow>, BackendError> {
    parse_json_array(output, "list-tabs")
}

fn parse_panes_json(output: &str) -> Result<Vec<ZellijPaneRow>, BackendError> {
    parse_json_array(output, "list-panes")
}

fn parse_json_array<T>(output: &str, command: &str) -> Result<Vec<T>, BackendError>
where
    T: DeserializeOwned,
{
    let payload: Value = serde_json::from_str(output).map_err(|error| {
        BackendError::internal(format!("invalid zellij {command} json: {error}"))
    })?;
    match payload {
        Value::Array(items) => serde_json::from_value(Value::Array(items)).map_err(|error| {
            BackendError::internal(format!("invalid zellij {command} json: {error}"))
        }),
        other => Err(BackendError::internal(format!(
            "unexpected zellij {command} payload while the session was settling: {}",
            summarize_payload(&other)
        ))),
    }
}

fn summarize_payload(payload: &Value) -> String {
    let rendered = payload.to_string();
    if rendered.chars().count() > 160 {
        format!("{}...", rendered.chars().take(160).collect::<String>())
    } else {
        rendered
    }
}

fn is_transient_zellij_error(message: &str) -> bool {
    message.contains("No active zellij sessions found")
        || message.contains("There is no active session")
        || message.contains("Session '") && message.contains("' not found")
}

fn is_transient_zellij_backend_error(error: &BackendError) -> bool {
    is_transient_zellij_error(&error.message)
        || error.message.contains("invalid zellij list-tabs json: EOF while parsing a value")
        || error.message.contains("invalid zellij list-tabs json: expected value")
        || error.message.contains("invalid zellij list-panes json: EOF while parsing a value")
        || error.message.contains("invalid zellij list-panes json: expected value")
        || error
            .message
            .contains("unexpected zellij list-tabs payload while the session was settling")
        || error
            .message
            .contains("unexpected zellij list-panes payload while the session was settling")
        || error.message.contains(
            "zellij snapshot commands returned empty output while the session was still settling",
        )
        || error.message.contains("exposed no importable panes")
}

fn build_session_snapshot(
    session_id: SessionId,
    target: &ZellijTarget,
    tabs: &[ZellijTabRow],
    panes: &[ZellijPaneRow],
) -> Result<ZellijSessionSnapshot, BackendError> {
    let mut tabs = tabs.to_vec();
    tabs.sort_by_key(|tab| if tab.position == 0 { tab.tab_id } else { tab.position });

    let mut tab_targets = HashMap::new();
    let mut pane_targets = HashMap::new();
    let mut topology_tabs = Vec::new();
    let mut focused_tab = None;
    let focused_tab_from_pane = panes.iter().find(|pane| pane.is_focused).map(|pane| pane.tab_id);

    for (ordinal, tab) in tabs.into_iter().enumerate() {
        let position = if tab.position == 0 { ordinal as u32 + 1 } else { tab.position };
        let mut tab_panes: Vec<ZellijPaneRow> = panes
            .iter()
            .filter(|pane| pane.tab_id == tab.tab_id && !pane.is_floating)
            .cloned()
            .collect();
        if tab_panes.is_empty() {
            continue;
        }

        tab_panes.sort_by_key(|pane| (pane.pane_y, pane.pane_x, pane.id));
        let tab_id = deterministic_tab_id(target, tab.tab_id, position);
        let pane_ids: Vec<PaneId> = tab_panes
            .iter()
            .map(|pane| {
                let pane_id = deterministic_pane_id(target, tab.tab_id, &pane.backend_ref());
                pane_targets.insert(
                    pane_id,
                    ZellijPaneTarget {
                        backend_ref: pane.backend_ref(),
                        kind: if pane.is_plugin {
                            ZellijPaneKind::Plugin
                        } else {
                            ZellijPaneKind::Terminal
                        },
                        title: non_empty(&pane.title),
                        rows: pane.pane_rows,
                        cols: pane.pane_columns,
                    },
                );
                pane_id
            })
            .collect();
        let focused_pane = tab_panes
            .iter()
            .find(|pane| pane.is_focused)
            .map(|pane| deterministic_pane_id(target, tab.tab_id, &pane.backend_ref()))
            .or_else(|| pane_ids.first().copied());

        tab_targets.insert(
            tab_id,
            ZellijTabTarget { backend_tab_id: tab.tab_id, position, title: non_empty(&tab.name) },
        );

        if focused_tab.is_none() && (tab.active || focused_tab_from_pane == Some(tab.tab_id)) {
            focused_tab = Some(tab_id);
        }

        topology_tabs.push((
            position,
            TabSnapshot {
                tab_id,
                title: non_empty(&tab.name),
                root: fallback_tree(pane_ids.into_iter()),
                focused_pane,
            },
        ));
    }

    topology_tabs.sort_by_key(|(position, _)| *position);
    let tabs: Vec<TabSnapshot> = topology_tabs.into_iter().map(|(_, tab)| tab).collect();
    if tabs.is_empty() {
        return Err(BackendError::not_found(format!(
            "zellij session '{}' exposed no importable panes",
            target.session_name
        )));
    }
    let focused_tab = focused_tab.or_else(|| tabs.first().map(|tab| tab.tab_id));

    Ok(ZellijSessionSnapshot {
        topology: TopologySnapshot {
            session_id,
            backend_kind: BackendKind::Zellij,
            tabs,
            focused_tab,
        },
        tab_targets,
        pane_targets,
    })
}

fn classify_surface(
    parsed_version: (u64, u64, u64),
    root_help: Option<&str>,
    action_help: Option<&str>,
) -> ZellijSurface {
    if let (Some(root_help), Some(action_help)) = (root_help, action_help) {
        let has_subscribe = help_contains_subcommand(root_help, "subscribe");
        let has_list_panes = help_contains_subcommand(action_help, "list-panes");
        let has_list_tabs = help_contains_subcommand(action_help, "list-tabs");
        if has_subscribe && has_list_panes && has_list_tabs {
            return ZellijSurface::RichCli044Plus;
        }

        let has_query_tab_names = help_contains_subcommand(action_help, "query-tab-names");
        let has_dump_layout = help_contains_subcommand(action_help, "dump-layout");
        if has_query_tab_names || has_dump_layout {
            return ZellijSurface::LegacyCli043;
        }
    }

    if parsed_version >= (0, 44, 0) {
        ZellijSurface::RichCli044Plus
    } else if parsed_version >= (0, 43, 0) {
        ZellijSurface::LegacyCli043
    } else {
        ZellijSurface::Unknown
    }
}

fn help_contains_subcommand(help: &str, subcommand: &str) -> bool {
    help.lines().map(str::trim_start).any(|line| line.starts_with(subcommand))
}

fn parse_semver_triplet(token: &str) -> Option<(u64, u64, u64)> {
    let stripped = token.trim().trim_start_matches('v');
    let mut parts = stripped.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;

    Some((major, minor, patch))
}

fn deterministic_tab_id(target: &ZellijTarget, backend_tab_id: u32, position: u32) -> TabId {
    deterministic_uuid(
        &format!(
            "terminal-platform/zellij/tab/{}/{}/{}",
            target.session_name, backend_tab_id, position
        ),
        TabId::from,
    )
}

fn deterministic_pane_id(target: &ZellijTarget, backend_tab_id: u32, backend_ref: &str) -> PaneId {
    deterministic_uuid(
        &format!(
            "terminal-platform/zellij/pane/{}/{}/{}",
            target.session_name, backend_tab_id, backend_ref
        ),
        PaneId::from,
    )
}

fn deterministic_uuid<T>(fingerprint: &str, construct: fn(Uuid) -> T) -> T {
    construct(Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes()))
}

fn fallback_tree(mut pane_ids: impl Iterator<Item = PaneId>) -> PaneTreeNode {
    let first = pane_ids
        .next()
        .map(|pane_id| PaneTreeNode::Leaf { pane_id })
        .unwrap_or_else(|| PaneTreeNode::Leaf { pane_id: PaneId::new() });

    pane_ids.fold(first, |node, pane_id| {
        PaneTreeNode::Split(PaneSplit {
            direction: SplitDirection::Vertical,
            first: Box::new(node),
            second: Box::new(PaneTreeNode::Leaf { pane_id }),
        })
    })
}

fn screen_lines_from_output(output: &str) -> Vec<ScreenLine> {
    output.lines().map(|line| ScreenLine { text: line.to_string() }).collect()
}

fn screen_snapshot_from_lines(
    pane_id: PaneId,
    pane_target: &ZellijPaneTarget,
    lines: Vec<ScreenLine>,
    source: ProjectionSource,
) -> ScreenSnapshot {
    let surface = ScreenSurface { title: pane_target.title.clone(), cursor: None, lines };
    let sequence = screen_sequence(
        pane_id,
        pane_target.rows,
        pane_target.cols,
        surface.title.as_deref(),
        &surface.lines,
    );

    ScreenSnapshot {
        pane_id,
        sequence,
        rows: pane_target.rows,
        cols: pane_target.cols,
        source,
        surface,
    }
}

fn screen_sequence(
    pane_id: PaneId,
    rows: u16,
    cols: u16,
    title: Option<&str>,
    lines: &[ScreenLine],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    pane_id.hash(&mut hasher);
    rows.hash(&mut hasher);
    cols.hash(&mut hasher);
    title.hash(&mut hasher);
    for line in lines {
        line.text.hash(&mut hasher);
    }
    hasher.finish()
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn flush_zellij_literal(pane_ref: &str, literal: &mut String, actions: &mut Vec<ZellijAction>) {
    if literal.is_empty() {
        return;
    }

    actions
        .push(ZellijAction::WriteChars { pane_ref: pane_ref.to_string(), chars: literal.clone() });
    literal.clear();
}

fn tab_contains_pane(tab: &TabSnapshot, pane_id: PaneId) -> bool {
    collect_pane_ids(&tab.root).into_iter().any(|candidate| candidate == pane_id)
}

fn collect_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

fn collect_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_inner(&split.first, pane_ids);
            collect_pane_ids_inner(&split.second, pane_ids);
        }
    }
}

#[doc(hidden)]
pub mod __fuzz {
    use terminal_backend_api::BackendError;

    #[must_use]
    pub fn probe_surface_code(
        version_output: &str,
        root_help: Option<&str>,
        action_help: Option<&str>,
    ) -> u8 {
        match super::ZellijProbe::parse(version_output, root_help, action_help).surface {
            super::ZellijSurface::LegacyCli043 => 1,
            super::ZellijSurface::RichCli044Plus => 2,
            super::ZellijSurface::Unknown => 0,
        }
    }

    pub fn parse_tabs_json_len(output: &str) -> Result<usize, BackendError> {
        Ok(super::parse_tabs_json(output)?.len())
    }

    pub fn parse_panes_json_len(output: &str) -> Result<usize, BackendError> {
        Ok(super::parse_panes_json(output)?.len())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex as StdMutex};

    use terminal_backend_api::{
        BackendErrorKind, MuxCommand, NewTabSpec, SendInputSpec, SendPasteSpec,
    };
    use terminal_domain::{DegradedModeReason, RouteAuthority, SessionRoute};
    use tokio::sync::Mutex;

    use super::{
        ZELLIJ_ROUTE_NAMESPACE, ZellijAction, ZellijAttachedSession, ZellijBackend, ZellijPaneKind,
        ZellijPaneRow, ZellijProbe, ZellijSurface, ZellijTabRow, ZellijTarget,
        build_session_snapshot, collect_pane_ids, parse_panes_json, parse_semver_triplet,
        parse_tabs_json,
    };

    #[test]
    fn parses_legacy_surface_from_cli_help() {
        let probe = ZellijProbe::parse(
            "zellij 0.43.1",
            Some("SUBCOMMANDS:\n    action\n    attach\n"),
            Some("SUBCOMMANDS:\n    dump-layout\n    query-tab-names\n"),
        );

        assert_eq!(probe.surface, ZellijSurface::LegacyCli043);
    }

    #[test]
    fn parses_rich_surface_from_cli_help() {
        let probe = ZellijProbe::parse(
            "zellij 0.44.1",
            Some("SUBCOMMANDS:\n    action\n    subscribe\n"),
            Some("SUBCOMMANDS:\n    list-panes\n    list-tabs\n"),
        );

        assert_eq!(probe.surface, ZellijSurface::RichCli044Plus);
    }

    #[test]
    fn falls_back_to_version_when_help_is_missing() {
        let probe = ZellijProbe::parse("zellij 0.43.1", None, None);

        assert_eq!(probe.surface, ZellijSurface::LegacyCli043);
    }

    #[test]
    fn parses_semver_triplet() {
        assert_eq!(parse_semver_triplet("0.43.1"), Some((0, 43, 1)));
        assert_eq!(parse_semver_triplet("v0.44.0"), Some((0, 44, 0)));
    }

    #[test]
    fn roundtrips_zellij_route_target() {
        let route = SessionRoute {
            backend: terminal_domain::BackendKind::Zellij,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: ZELLIJ_ROUTE_NAMESPACE.to_string(),
                value: "session=workspace".to_string(),
            }),
        };

        let target = ZellijTarget::from_route(&route).expect("route should decode");
        assert_eq!(target.session_name, "workspace");
    }

    #[test]
    fn rejects_invalid_zellij_route_namespace() {
        let route = SessionRoute {
            backend: terminal_domain::BackendKind::Zellij,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "other".to_string(),
                value: "session=workspace".to_string(),
            }),
        };

        let error = ZellijTarget::from_route(&route).expect_err("route should fail");
        assert_eq!(error.kind, terminal_backend_api::BackendErrorKind::InvalidInput);
    }

    #[test]
    fn parses_rich_tab_rows_from_json() {
        let tabs = parse_tabs_json(
            r#"
            [
              { "tab_id": 1, "position": 0, "name": "shell", "active": true },
              { "tab_id": 2, "position": 1, "name": "logs", "active": false }
            ]
            "#,
        )
        .expect("tab rows should decode");

        assert_eq!(
            tabs,
            vec![
                ZellijTabRow { tab_id: 1, position: 0, name: "shell".to_string(), active: true },
                ZellijTabRow { tab_id: 2, position: 1, name: "logs".to_string(), active: false },
            ]
        );
    }

    #[test]
    fn parses_rich_pane_rows_from_json() {
        let panes = parse_panes_json(
            r#"
            [
              {
                "id": 1,
                "tab_id": 1,
                "title": "shell",
                "is_plugin": false,
                "is_focused": true,
                "is_floating": false,
                "pane_x": 0,
                "pane_y": 0,
                "pane_rows": 24,
                "pane_columns": 80
              },
              {
                "id": 2,
                "tab_id": 1,
                "title": "status",
                "is_plugin": true,
                "is_focused": false,
                "is_floating": false,
                "pane_x": 81,
                "pane_y": 0,
                "pane_rows": 24,
                "pane_columns": 40
              }
            ]
            "#,
        )
        .expect("pane rows should decode");

        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].backend_ref(), "terminal_1");
        assert_eq!(panes[1].backend_ref(), "plugin_2");
    }

    #[test]
    fn builds_session_snapshot_from_rich_cli_rows() {
        let target = ZellijTarget { session_name: "workspace".to_string() };
        let session_id = terminal_domain::SessionId::new();
        let tabs = vec![
            ZellijTabRow { tab_id: 1, position: 0, name: "shell".to_string(), active: true },
            ZellijTabRow { tab_id: 2, position: 1, name: "logs".to_string(), active: false },
        ];
        let panes = vec![
            ZellijPaneRow {
                id: 1,
                tab_id: 1,
                title: "shell".to_string(),
                is_plugin: false,
                is_focused: true,
                is_floating: false,
                pane_x: 0,
                pane_y: 0,
                pane_rows: 24,
                pane_columns: 80,
            },
            ZellijPaneRow {
                id: 2,
                tab_id: 1,
                title: "status".to_string(),
                is_plugin: true,
                is_focused: false,
                is_floating: false,
                pane_x: 81,
                pane_y: 0,
                pane_rows: 24,
                pane_columns: 40,
            },
            ZellijPaneRow {
                id: 3,
                tab_id: 2,
                title: "logs".to_string(),
                is_plugin: false,
                is_focused: false,
                is_floating: false,
                pane_x: 0,
                pane_y: 0,
                pane_rows: 24,
                pane_columns: 100,
            },
        ];

        let snapshot = build_session_snapshot(session_id, &target, &tabs, &panes)
            .expect("snapshot should build");

        assert_eq!(snapshot.topology.backend_kind, terminal_domain::BackendKind::Zellij);
        assert_eq!(snapshot.topology.tabs.len(), 2);
        assert_eq!(snapshot.topology.focused_tab, Some(snapshot.topology.tabs[0].tab_id));
        assert_eq!(
            snapshot.topology.tabs[0].focused_pane,
            Some(collect_pane_ids(&snapshot.topology.tabs[0].root)[0])
        );
        assert_eq!(snapshot.tab_targets.len(), 2);
        assert_eq!(snapshot.pane_targets.len(), 3);
        assert!(snapshot.pane_targets.values().any(|pane| pane.backend_ref == "plugin_2"));
        assert!(snapshot.pane_targets.values().any(|pane| pane.kind == ZellijPaneKind::Plugin));
    }

    #[test]
    fn builds_targeted_dispatch_actions_for_rich_surface() {
        let (attached, snapshot, first_tab, second_tab, terminal_pane, _plugin_pane) =
            sample_attached_session();

        assert_eq!(
            attached
                .dispatch_actions(
                    &snapshot,
                    MuxCommand::NewTab(NewTabSpec { title: Some("debug".to_string()) }),
                )
                .expect("new-tab should map"),
            vec![ZellijAction::NewTab { title: Some("debug".to_string()) }]
        );
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::FocusTab { tab_id: second_tab })
                .expect("focus-tab should map"),
            vec![ZellijAction::FocusTab { backend_tab_id: 2 }]
        );
        assert_eq!(
            attached
                .dispatch_actions(
                    &snapshot,
                    MuxCommand::RenameTab { tab_id: second_tab, title: "renamed".to_string() },
                )
                .expect("rename-tab should map"),
            vec![ZellijAction::RenameTab { backend_tab_id: 2, title: "renamed".to_string() }]
        );
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::CloseTab { tab_id: second_tab })
                .expect("close-tab should map"),
            vec![ZellijAction::CloseTab { backend_tab_id: 2 }]
        );
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::FocusPane { pane_id: terminal_pane })
                .expect("focus-pane should map"),
            vec![ZellijAction::FocusPane { pane_ref: "terminal_1".to_string() }]
        );
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::ClosePane { pane_id: terminal_pane })
                .expect("close-pane should map"),
            vec![ZellijAction::ClosePane { pane_ref: "terminal_1".to_string() }]
        );
        assert_ne!(first_tab, second_tab);
    }

    #[test]
    fn splits_terminal_input_into_ordered_rich_actions() {
        let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
            sample_attached_session();

        let actions = attached
            .dispatch_actions(
                &snapshot,
                MuxCommand::SendInput(SendInputSpec {
                    pane_id: terminal_pane,
                    data: "echo\tok\r".to_string(),
                }),
            )
            .expect("send-input should map");

        assert_eq!(
            actions,
            vec![
                ZellijAction::WriteChars {
                    pane_ref: "terminal_1".to_string(),
                    chars: "echo".to_string(),
                },
                ZellijAction::SendKeys {
                    pane_ref: "terminal_1".to_string(),
                    keys: vec!["Tab".to_string()],
                },
                ZellijAction::WriteChars {
                    pane_ref: "terminal_1".to_string(),
                    chars: "ok".to_string(),
                },
                ZellijAction::SendKeys {
                    pane_ref: "terminal_1".to_string(),
                    keys: vec!["Enter".to_string()],
                },
            ]
        );
    }

    #[test]
    fn maps_paste_to_target_terminal_pane() {
        let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
            sample_attached_session();

        let actions = attached
            .dispatch_actions(
                &snapshot,
                MuxCommand::SendPaste(SendPasteSpec {
                    pane_id: terminal_pane,
                    data: "hello\nworld".to_string(),
                }),
            )
            .expect("send-paste should map");

        assert_eq!(
            actions,
            vec![ZellijAction::Paste {
                pane_ref: "terminal_1".to_string(),
                text: "hello\nworld".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_plugin_input_writes() {
        let (attached, snapshot, _first_tab, _second_tab, _terminal_pane, plugin_pane) =
            sample_attached_session();

        let error = attached
            .dispatch_actions(
                &snapshot,
                MuxCommand::SendInput(SendInputSpec {
                    pane_id: plugin_pane,
                    data: "hello".to_string(),
                }),
            )
            .expect_err("plugin input should fail");

        assert_eq!(error.kind, BackendErrorKind::Unsupported);
        assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
    }

    #[test]
    fn rejects_closing_last_foreign_tab() {
        let (attached, snapshot, first_tab, _pane) = single_tab_attached_session();

        let error = attached
            .dispatch_actions(&snapshot, MuxCommand::CloseTab { tab_id: first_tab })
            .expect_err("closing the last tab should fail");

        assert_eq!(error.kind, BackendErrorKind::Unsupported);
        assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
    }

    #[test]
    fn rejects_closing_last_pane_in_tab() {
        let (attached, snapshot, _first_tab, pane_id) = single_tab_attached_session();

        let error = attached
            .dispatch_actions(&snapshot, MuxCommand::ClosePane { pane_id })
            .expect_err("closing the last pane should fail");

        assert_eq!(error.kind, BackendErrorKind::Unsupported);
        assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
    }

    fn sample_attached_session() -> (
        ZellijAttachedSession,
        super::ZellijSessionSnapshot,
        terminal_domain::TabId,
        terminal_domain::TabId,
        terminal_domain::PaneId,
        terminal_domain::PaneId,
    ) {
        let target = ZellijTarget { session_name: "workspace".to_string() };
        let session_id = terminal_domain::SessionId::new();
        let tabs = vec![
            ZellijTabRow { tab_id: 1, position: 0, name: "shell".to_string(), active: true },
            ZellijTabRow { tab_id: 2, position: 1, name: "logs".to_string(), active: false },
        ];
        let panes = vec![
            ZellijPaneRow {
                id: 1,
                tab_id: 1,
                title: "shell".to_string(),
                is_plugin: false,
                is_focused: true,
                is_floating: false,
                pane_x: 0,
                pane_y: 0,
                pane_rows: 24,
                pane_columns: 80,
            },
            ZellijPaneRow {
                id: 2,
                tab_id: 1,
                title: "status".to_string(),
                is_plugin: true,
                is_focused: false,
                is_floating: false,
                pane_x: 81,
                pane_y: 0,
                pane_rows: 24,
                pane_columns: 40,
            },
            ZellijPaneRow {
                id: 3,
                tab_id: 2,
                title: "logs".to_string(),
                is_plugin: false,
                is_focused: false,
                is_floating: false,
                pane_x: 0,
                pane_y: 0,
                pane_rows: 24,
                pane_columns: 100,
            },
        ];

        let snapshot = build_session_snapshot(session_id, &target, &tabs, &panes)
            .expect("snapshot should build");
        let first_tab = snapshot.topology.tabs[0].tab_id;
        let second_tab = snapshot.topology.tabs[1].tab_id;
        let tab_one_panes = collect_pane_ids(&snapshot.topology.tabs[0].root);
        let attached = ZellijAttachedSession {
            backend: Arc::new(ZellijBackend),
            session_id,
            target,
            io_lane: Arc::new(StdMutex::new(())),
            command_lane: Arc::new(Mutex::new(())),
        };

        (attached, snapshot, first_tab, second_tab, tab_one_panes[0], tab_one_panes[1])
    }

    fn single_tab_attached_session() -> (
        ZellijAttachedSession,
        super::ZellijSessionSnapshot,
        terminal_domain::TabId,
        terminal_domain::PaneId,
    ) {
        let target = ZellijTarget { session_name: "workspace".to_string() };
        let session_id = terminal_domain::SessionId::new();
        let tabs =
            vec![ZellijTabRow { tab_id: 1, position: 0, name: "shell".to_string(), active: true }];
        let panes = vec![ZellijPaneRow {
            id: 1,
            tab_id: 1,
            title: "shell".to_string(),
            is_plugin: false,
            is_focused: true,
            is_floating: false,
            pane_x: 0,
            pane_y: 0,
            pane_rows: 24,
            pane_columns: 80,
        }];

        let snapshot = build_session_snapshot(session_id, &target, &tabs, &panes)
            .expect("snapshot should build");
        let tab_id = snapshot.topology.tabs[0].tab_id;
        let pane_id = collect_pane_ids(&snapshot.topology.tabs[0].root)[0];
        let attached = ZellijAttachedSession {
            backend: Arc::new(ZellijBackend),
            session_id,
            target,
            io_lane: Arc::new(StdMutex::new(())),
            command_lane: Arc::new(Mutex::new(())),
        };

        (attached, snapshot, tab_id, pane_id)
    }
}
