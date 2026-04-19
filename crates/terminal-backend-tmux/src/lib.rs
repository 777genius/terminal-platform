use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    process::Command,
    sync::Arc,
};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BackendSubscriptionEvent, BoxFuture,
    CreateSessionSpec, DiscoveredSession, MuxBackendPort, MuxCommand, MuxCommandResult,
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
    sync::{mpsc, oneshot},
    time::{self, Duration, MissedTickBehavior},
};
use uuid::Uuid;

const TMUX_ROUTE_NAMESPACE: &str = "tmux_target";
const TMUX_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Default)]
pub struct TmuxBackend {
    socket_name: Option<String>,
}

impl TmuxBackend {
    #[must_use]
    pub fn with_socket_name(socket_name: impl Into<String>) -> Self {
        Self { socket_name: Some(socket_name.into()) }
    }

    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Tmux
    }

    fn run(&self, target: Option<&TmuxTarget>, args: &[&str]) -> Result<String, BackendError> {
        let mut command = Command::new("tmux");
        if let Some(socket_name) =
            target.and_then(|target| target.socket_name.as_deref()).or(self.socket_name.as_deref())
        {
            command.arg("-L").arg(socket_name);
        }
        command.args(args);

        let output = command.output().map_err(|error| {
            BackendError::transport(format!("tmux command failed to spawn: {error}"))
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::transport(format!("tmux command failed: {}", stderr.trim())));
        }

        String::from_utf8(output.stdout)
            .map_err(|error| BackendError::internal(format!("tmux output is not utf8: {error}")))
    }

    fn run_owned(
        &self,
        target: Option<&TmuxTarget>,
        args: &[String],
    ) -> Result<String, BackendError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(target, &refs)
    }
}

impl MuxBackendPort for TmuxBackend {
    fn kind(&self) -> BackendKind {
        self.kind()
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async {
            Ok(BackendCapabilities {
                tiled_panes: true,
                tab_create: true,
                tab_close: true,
                tab_focus: true,
                tab_rename: true,
                session_scoped_tab_refs: true,
                session_scoped_pane_refs: true,
                pane_input_write: true,
                pane_paste_write: true,
                rendered_viewport_stream: true,
                rendered_viewport_snapshot: true,
                advisory_metadata_subscriptions: true,
                read_only_client_mode: true,
                ..BackendCapabilities::default()
            })
        })
    }

    fn discover_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async move {
            let output = self.run(
                None,
                &[
                    "list-sessions",
                    "-F",
                    "#{session_name}\t#{session_windows}\t#{session_attached}",
                ],
            )?;
            let mut sessions = Vec::new();
            for line in output.lines().filter(|line| !line.trim().is_empty()) {
                let mut fields = line.split('\t');
                let Some(session_name) = fields.next() else {
                    continue;
                };
                let target = TmuxTarget {
                    socket_name: self.socket_name.clone(),
                    session_name: session_name.to_string(),
                };
                sessions.push(DiscoveredSession {
                    route: target.route(),
                    title: Some(session_name.to_string()),
                });
            }

            Ok(sessions)
        })
    }

    fn create_session(
        &self,
        _spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "tmux sessions are imported, not created",
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
            if route.backend != BackendKind::Tmux {
                return Err(BackendError::invalid_input(
                    "tmux backend can only attach tmux routes",
                ));
            }
            let target = TmuxTarget::from_route(&route)?;
            backend.run(Some(&target), &["has-session", "-t", &target.session_name])?;
            let session_id = imported_session_id(&route)
                .ok_or_else(|| BackendError::invalid_input("tmux route is not importable"))?;

            Ok(Box::new(TmuxAttachedSession { backend: Arc::new(backend), session_id, target })
                as Box<dyn BackendSessionPort>)
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "tmux backend does not expose canonical sessions directly",
                DegradedModeReason::ImportedForeignSession,
            ))
        })
    }
}

#[derive(Clone)]
struct TmuxAttachedSession {
    backend: Arc<TmuxBackend>,
    session_id: SessionId,
    target: TmuxTarget,
}

impl BackendSessionPort for TmuxAttachedSession {
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
        Box::pin(async move { self.dispatch_inner(command) })
    }

    fn subscribe(
        &self,
        spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let session = self.clone();
        Box::pin(async move { session.open_subscription(spec) })
    }
}

impl TmuxAttachedSession {
    fn dispatch_inner(&self, command: MuxCommand) -> Result<MuxCommandResult, BackendError> {
        match command {
            MuxCommand::NewTab(spec) => self.new_tab(spec),
            MuxCommand::SendInput(spec) => self.send_input(spec),
            MuxCommand::SendPaste(spec) => self.send_paste(spec),
            MuxCommand::CloseTab { tab_id } => self.close_tab(tab_id),
            MuxCommand::FocusTab { tab_id } => self.focus_tab(tab_id),
            MuxCommand::RenameTab { tab_id, title } => self.rename_tab(tab_id, &title),
            MuxCommand::SplitPane(_)
            | MuxCommand::ClosePane { .. }
            | MuxCommand::FocusPane { .. }
            | MuxCommand::ResizePane(_)
            | MuxCommand::Detach
            | MuxCommand::SaveSession
            | MuxCommand::OverrideLayout(_) => Err(BackendError::unsupported(
                "tmux imported routes do not support this command in the current rollout phase",
                DegradedModeReason::UnsupportedByBackend,
            )),
        }
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
            let mut ticker = time::interval(TMUX_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    _ = ticker.tick() => {
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

            let mut last = initial;
            let mut ticker = time::interval(TMUX_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    _ = ticker.tick() => {
                        let current = match session.screen_snapshot_inner(pane_id) {
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
                }
            }
        });

        Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
    }

    fn rename_tab(&self, tab_id: TabId, title: &str) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend
            .run(Some(&self.target), &["rename-window", "-t", &tab_target.target, title])?;

        Ok(MuxCommandResult { changed: true })
    }

    fn focus_tab(&self, tab_id: TabId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend.run(Some(&self.target), &["select-window", "-t", &tab_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    fn new_tab(
        &self,
        spec: terminal_backend_api::NewTabSpec,
    ) -> Result<MuxCommandResult, BackendError> {
        let mut args = vec![
            "new-window".to_string(),
            "-P".to_string(),
            "-F".to_string(),
            "#{window_id}".to_string(),
        ];
        args.push("-t".to_string());
        args.push(self.target.session_name.clone());
        if let Some(title) = spec.title {
            args.push("-n".to_string());
            args.push(title);
        }
        self.backend.run_owned(Some(&self.target), &args)?;

        Ok(MuxCommandResult { changed: true })
    }

    fn close_tab(&self, tab_id: TabId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        if snapshot.topology.tabs.len() <= 1 {
            return Err(BackendError::unsupported(
                "tmux imported routes refuse to close the last tab because it would terminate the foreign session",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux tab {tab_id:?}")))?;
        self.backend.run(Some(&self.target), &["kill-window", "-t", &tab_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    fn send_input(&self, spec: SendInputSpec) -> Result<MuxCommandResult, BackendError> {
        self.send_text_to_pane(spec.pane_id, &spec.data)
    }

    fn send_paste(&self, spec: SendPasteSpec) -> Result<MuxCommandResult, BackendError> {
        self.send_text_to_pane(spec.pane_id, &spec.data)
    }

    fn send_text_to_pane(
        &self,
        pane_id: PaneId,
        data: &str,
    ) -> Result<MuxCommandResult, BackendError> {
        if data.is_empty() {
            return Ok(MuxCommandResult { changed: false });
        }

        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        self.send_tmux_text(&pane_target.target, data)?;

        Ok(MuxCommandResult { changed: true })
    }

    fn send_tmux_text(&self, pane_target: &str, data: &str) -> Result<(), BackendError> {
        let mut literal = String::new();
        for ch in data.chars() {
            match ch {
                '\r' | '\n' => {
                    self.flush_tmux_literal(pane_target, &mut literal)?;
                    self.backend
                        .run(Some(&self.target), &["send-keys", "-t", pane_target, "Enter"])?;
                }
                '\t' => {
                    self.flush_tmux_literal(pane_target, &mut literal)?;
                    self.backend
                        .run(Some(&self.target), &["send-keys", "-t", pane_target, "Tab"])?;
                }
                c if c.is_control() => {
                    return Err(BackendError::unsupported(
                        format!("tmux input path does not support control character {:?}", c),
                        DegradedModeReason::UnsupportedByBackend,
                    ));
                }
                c => literal.push(c),
            }
        }

        self.flush_tmux_literal(pane_target, &mut literal)
    }

    fn flush_tmux_literal(
        &self,
        pane_target: &str,
        literal: &mut String,
    ) -> Result<(), BackendError> {
        if literal.is_empty() {
            return Ok(());
        }

        let args = vec![
            "send-keys".to_string(),
            "-t".to_string(),
            pane_target.to_string(),
            "-l".to_string(),
            literal.clone(),
        ];
        self.backend.run_owned(Some(&self.target), &args)?;
        literal.clear();

        Ok(())
    }

    fn snapshot(&self) -> Result<TmuxSessionSnapshot, BackendError> {
        let windows_output = self.backend.run(
            Some(&self.target),
            &[
                "list-windows",
                "-t",
                &self.target.session_name,
                "-F",
                "#{window_index}\t#{window_id}\t#{window_name}\t#{window_active}\t#{window_layout}",
            ],
        )?;
        let panes_output = self.backend.run(
            Some(&self.target),
            &[
                "list-panes",
                "-t",
                &self.target.session_name,
                "-F",
                "#{window_id}\t#{pane_id}\t#{pane_index}\t#{pane_title}\t#{pane_active}\t#{pane_width}\t#{pane_height}",
            ],
        )?;

        let mut panes_by_window: HashMap<String, Vec<TmuxPaneRow>> = HashMap::new();
        for line in panes_output.lines().filter(|line| !line.trim().is_empty()) {
            let row = TmuxPaneRow::parse(line)?;
            panes_by_window.entry(row.window_id.clone()).or_default().push(row);
        }

        let mut focused_tab = None;
        let mut tabs = Vec::new();
        let mut pane_targets = HashMap::new();
        let mut tab_targets = HashMap::new();
        for line in windows_output.lines().filter(|line| !line.trim().is_empty()) {
            let window = TmuxWindowRow::parse(line)?;
            let mut panes = panes_by_window.remove(&window.window_id).unwrap_or_default();
            panes.sort_by_key(|pane| pane.pane_index);
            let pane_ids: HashMap<u32, PaneId> = panes
                .iter()
                .map(|pane| {
                    (
                        pane.pane_index,
                        deterministic_pane_id(&self.target, &window.window_id, &pane.pane_id),
                    )
                })
                .collect();

            for pane in &panes {
                let canonical_pane_id =
                    deterministic_pane_id(&self.target, &window.window_id, &pane.pane_id);
                pane_targets.insert(
                    canonical_pane_id,
                    TmuxPaneTarget {
                        target: pane.pane_id.clone(),
                        title: non_empty(&pane.pane_title),
                        rows: pane.pane_height,
                        cols: pane.pane_width,
                    },
                );
            }

            let tab_id = deterministic_tab_id(&self.target, &window.window_id);
            tab_targets.insert(tab_id, TmuxTabTarget { target: window.window_id.clone() });
            let focused_pane = panes
                .iter()
                .find(|pane| pane.pane_active)
                .map(|pane| deterministic_pane_id(&self.target, &window.window_id, &pane.pane_id));
            if window.window_active {
                focused_tab = Some(tab_id);
            }
            tabs.push((
                window.window_index,
                TabSnapshot {
                    tab_id,
                    title: non_empty(&window.window_name),
                    root: parse_tmux_layout(&window.window_layout, &pane_ids).unwrap_or_else(
                        || fallback_tree(panes.iter().map(|pane| pane_ids[&pane.pane_index])),
                    ),
                    focused_pane,
                },
            ));
        }
        tabs.sort_by_key(|(window_index, _)| *window_index);

        Ok(TmuxSessionSnapshot {
            topology: TopologySnapshot {
                session_id: self.session_id,
                backend_kind: BackendKind::Tmux,
                tabs: tabs.into_iter().map(|(_, tab)| tab).collect(),
                focused_tab,
            },
            pane_targets,
            tab_targets,
        })
    }

    fn screen_snapshot_inner(&self, pane_id: PaneId) -> Result<ScreenSnapshot, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        let output = self
            .backend
            .run(Some(&self.target), &["capture-pane", "-p", "-J", "-t", &pane_target.target])?;
        let lines: Vec<ScreenLine> =
            output.lines().map(|line| ScreenLine { text: line.to_string() }).collect();
        let surface = ScreenSurface { title: pane_target.title.clone(), cursor: None, lines };
        let sequence = screen_sequence(
            pane_id,
            pane_target.rows,
            pane_target.cols,
            surface.title.as_deref(),
            &surface.lines,
        );

        Ok(ScreenSnapshot {
            pane_id,
            sequence,
            rows: pane_target.rows,
            cols: pane_target.cols,
            source: ProjectionSource::TmuxCapturePane,
            surface,
        })
    }
}

struct TmuxSessionSnapshot {
    topology: TopologySnapshot,
    pane_targets: HashMap<PaneId, TmuxPaneTarget>,
    tab_targets: HashMap<TabId, TmuxTabTarget>,
}

#[derive(Debug, Clone)]
struct TmuxTarget {
    socket_name: Option<String>,
    session_name: String,
}

impl TmuxTarget {
    fn from_route(route: &SessionRoute) -> Result<Self, BackendError> {
        if route.authority != RouteAuthority::ImportedForeign {
            return Err(BackendError::invalid_input("tmux route must be imported_foreign"));
        }
        let external = route.external.as_ref().ok_or_else(|| {
            BackendError::invalid_input("tmux route is missing external reference")
        })?;
        if external.namespace != TMUX_ROUTE_NAMESPACE {
            return Err(BackendError::invalid_input("tmux route namespace is invalid"));
        }
        let mut socket_name = None;
        let mut session_name = None;
        for part in external.value.split(';') {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            match key {
                "socket" if !value.is_empty() => socket_name = Some(value.to_string()),
                "session" if !value.is_empty() => session_name = Some(value.to_string()),
                _ => {}
            }
        }
        let session_name = session_name
            .ok_or_else(|| BackendError::invalid_input("tmux route is missing session"))?;

        Ok(Self { socket_name, session_name })
    }

    fn route(&self) -> SessionRoute {
        let mut value = String::new();
        if let Some(socket_name) = &self.socket_name {
            value.push_str("socket=");
            value.push_str(socket_name);
            value.push(';');
        }
        value.push_str("session=");
        value.push_str(&self.session_name);

        SessionRoute {
            backend: BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(ExternalSessionRef {
                namespace: TMUX_ROUTE_NAMESPACE.to_string(),
                value,
            }),
        }
    }
}

#[derive(Debug, Clone)]
struct TmuxWindowRow {
    window_index: u32,
    window_id: String,
    window_name: String,
    window_active: bool,
    window_layout: String,
}

impl TmuxWindowRow {
    fn parse(line: &str) -> Result<Self, BackendError> {
        let mut fields = line.split('\t');
        let window_index = parse_u32(next_field(&mut fields, "window_index")?, "window_index")?;
        let window_id = next_field(&mut fields, "window_id")?.to_string();
        let window_name = next_field(&mut fields, "window_name")?.to_string();
        let window_active = parse_bool(next_field(&mut fields, "window_active")?);
        let window_layout = next_field(&mut fields, "window_layout")?.to_string();

        Ok(Self { window_index, window_id, window_name, window_active, window_layout })
    }
}

#[derive(Debug, Clone)]
struct TmuxPaneRow {
    window_id: String,
    pane_id: String,
    pane_index: u32,
    pane_title: String,
    pane_active: bool,
    pane_width: u16,
    pane_height: u16,
}

impl TmuxPaneRow {
    fn parse(line: &str) -> Result<Self, BackendError> {
        let mut fields = line.split('\t');
        let window_id = next_field(&mut fields, "window_id")?.to_string();
        let pane_id = next_field(&mut fields, "pane_id")?.to_string();
        let pane_index = parse_u32(next_field(&mut fields, "pane_index")?, "pane_index")?;
        let pane_title = next_field(&mut fields, "pane_title")?.to_string();
        let pane_active = parse_bool(next_field(&mut fields, "pane_active")?);
        let pane_width = parse_u16(next_field(&mut fields, "pane_width")?, "pane_width")?;
        let pane_height = parse_u16(next_field(&mut fields, "pane_height")?, "pane_height")?;

        Ok(Self {
            window_id,
            pane_id,
            pane_index,
            pane_title,
            pane_active,
            pane_width,
            pane_height,
        })
    }
}

#[derive(Debug, Clone)]
struct TmuxPaneTarget {
    target: String,
    title: Option<String>,
    rows: u16,
    cols: u16,
}

#[derive(Debug, Clone)]
struct TmuxTabTarget {
    target: String,
}

fn deterministic_tab_id(target: &TmuxTarget, window_id: &str) -> TabId {
    deterministic_uuid(
        &format!(
            "terminal-platform/tmux/tab/{:?}/{}/{}",
            target.socket_name, target.session_name, window_id
        ),
        TabId::from,
    )
}

fn deterministic_pane_id(target: &TmuxTarget, window_id: &str, pane_id: &str) -> PaneId {
    deterministic_uuid(
        &format!(
            "terminal-platform/tmux/pane/{:?}/{}/{}/{}",
            target.socket_name, target.session_name, window_id, pane_id
        ),
        PaneId::from,
    )
}

fn deterministic_uuid<T>(fingerprint: &str, construct: fn(Uuid) -> T) -> T {
    construct(Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes()))
}

fn parse_tmux_layout(input: &str, pane_ids: &HashMap<u32, PaneId>) -> Option<PaneTreeNode> {
    let mut parser = LayoutParser::new(input)?;
    parser.parse_node(pane_ids)
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

struct LayoutParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> LayoutParser<'a> {
    fn new(input: &'a str) -> Option<Self> {
        let checksum_end = input.find(',')?;
        Some(Self { input: input.as_bytes(), pos: checksum_end + 1 })
    }

    fn parse_node(&mut self, pane_ids: &HashMap<u32, PaneId>) -> Option<PaneTreeNode> {
        self.parse_number()?;
        self.expect(b'x')?;
        self.parse_number()?;
        self.expect(b',')?;
        self.parse_number()?;
        self.expect(b',')?;
        self.parse_number()?;

        match self.peek()? {
            b',' => {
                self.pos += 1;
                let pane_index = self.parse_number()? as u32;
                pane_ids.get(&pane_index).copied().map(|pane_id| PaneTreeNode::Leaf { pane_id })
            }
            b'{' | b'[' => {
                let open = self.next()?;
                let close = if open == b'{' { b'}' } else { b']' };
                let direction = if open == b'{' {
                    SplitDirection::Vertical
                } else {
                    SplitDirection::Horizontal
                };
                let mut node = self.parse_node(pane_ids)?;
                while let Some(byte) = self.peek() {
                    if byte == close {
                        self.pos += 1;
                        break;
                    }
                    self.expect(b',')?;
                    let next = self.parse_node(pane_ids)?;
                    node = PaneTreeNode::Split(PaneSplit {
                        direction,
                        first: Box::new(node),
                        second: Box::new(next),
                    });
                }
                Some(node)
            }
            _ => None,
        }
    }

    fn parse_number(&mut self) -> Option<usize> {
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            self.pos += 1;
        }
        (self.pos > start)
            .then(|| std::str::from_utf8(&self.input[start..self.pos]).ok()?.parse().ok())
            .flatten()
    }

    fn expect(&mut self, expected: u8) -> Option<()> {
        (self.next()? == expected).then_some(())
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
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

fn next_field<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<&'a str, BackendError> {
    fields.next().ok_or_else(|| BackendError::internal(format!("missing tmux field {name}")))
}

fn parse_bool(value: &str) -> bool {
    value == "1"
}

fn parse_u32(value: &str, name: &str) -> Result<u32, BackendError> {
    value.parse().map_err(|error| BackendError::internal(format!("invalid {name}: {error}")))
}

fn parse_u16(value: &str, name: &str) -> Result<u16, BackendError> {
    value.parse().map_err(|error| BackendError::internal(format!("invalid {name}: {error}")))
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use terminal_domain::{RouteAuthority, SessionRoute};
    use terminal_mux_domain::PaneTreeNode;

    use super::{TMUX_ROUTE_NAMESPACE, TmuxTarget, fallback_tree, parse_tmux_layout};

    #[test]
    fn roundtrips_tmux_route_target() {
        let target = TmuxTarget {
            socket_name: Some("test-socket".to_string()),
            session_name: "workspace".to_string(),
        };

        let route = target.route();
        let decoded = TmuxTarget::from_route(&route).expect("route should decode");

        assert_eq!(route.backend, terminal_domain::BackendKind::Tmux);
        assert_eq!(route.authority, RouteAuthority::ImportedForeign);
        assert_eq!(decoded.socket_name.as_deref(), Some("test-socket"));
        assert_eq!(decoded.session_name, "workspace");
    }

    #[test]
    fn rejects_invalid_tmux_route_namespace() {
        let route = SessionRoute {
            backend: terminal_domain::BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "other".to_string(),
                value: "session=workspace".to_string(),
            }),
        };

        let error = TmuxTarget::from_route(&route).expect_err("route should fail");
        assert_eq!(error.kind, terminal_backend_api::BackendErrorKind::InvalidInput);
    }

    #[test]
    fn parses_nested_tmux_layout() {
        let pane_ids = BTreeMap::from([
            (0_u32, terminal_domain::PaneId::new()),
            (1_u32, terminal_domain::PaneId::new()),
            (2_u32, terminal_domain::PaneId::new()),
        ]);
        let root = parse_tmux_layout(
            "bb62,159x48,0,0{79x48,0,0,0,79x23,80,0[79x11,80,0,1,79x11,80,12,2]}",
            &pane_ids.into_iter().collect(),
        )
        .expect("layout should parse");

        match root {
            PaneTreeNode::Split(_) => {}
            other => panic!("unexpected layout root: {other:?}"),
        }
    }

    #[test]
    fn builds_fallback_tree_for_multiple_panes() {
        let pane_a = terminal_domain::PaneId::new();
        let pane_b = terminal_domain::PaneId::new();
        let pane_c = terminal_domain::PaneId::new();
        let root = fallback_tree([pane_a, pane_b, pane_c].into_iter());

        match root {
            PaneTreeNode::Split(_) => {}
            other => panic!("unexpected fallback root: {other:?}"),
        }
    }

    #[test]
    fn exported_namespace_stays_stable() {
        assert_eq!(TMUX_ROUTE_NAMESPACE, "tmux_target");
    }
}
