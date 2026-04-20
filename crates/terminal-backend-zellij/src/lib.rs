use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    process::Command,
    sync::Arc,
};

use serde::Deserialize;
use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BackendSubscriptionEvent, BoxFuture,
    CreateSessionSpec, DiscoveredSession, MuxBackendPort, MuxCommand, MuxCommandResult,
    SubscriptionSpec,
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
    sync::{mpsc, oneshot},
    time::{self, Duration, MissedTickBehavior},
};
use uuid::Uuid;

const ZELLIJ_ROUTE_NAMESPACE: &str = "zellij_session";
const ZELLIJ_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Default)]
pub struct ZellijBackend;

impl ZellijBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }

    fn run(&self, target: Option<&ZellijTarget>, args: &[&str]) -> Result<String, BackendError> {
        let mut command = Command::new("zellij");
        if let Some(target) = target {
            command.arg("--session").arg(&target.session_name);
        }
        command.args(args);

        let output = command.output().map_err(|error| {
            BackendError::transport(format!("zellij command failed to spawn: {error}"))
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::transport(format!(
                "zellij command failed: {}",
                stderr.trim()
            )));
        }

        String::from_utf8(output.stdout)
            .map_err(|error| BackendError::internal(format!("zellij output is not utf8: {error}")))
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
                    session_scoped_tab_refs: true,
                    session_scoped_pane_refs: true,
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
            let output = self.run(None, &["list-sessions", "--short", "--no-formatting"])?;
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
                    let attached =
                        ZellijAttachedSession { backend: Arc::new(backend), session_id, target };
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
        _command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij rich imported routes are currently read-only in the v1 rollout phase",
                DegradedModeReason::UnsupportedByBackend,
            ))
        })
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
        let tabs = parse_tabs_json(
            &self.backend.run(Some(&self.target), &["action", "list-tabs", "--json"])?,
        )?;
        let panes = parse_panes_json(
            &self.backend.run(Some(&self.target), &["action", "list-panes", "--json"])?,
        )?;

        build_session_snapshot(self.session_id, &self.target, &tabs, &panes)
    }

    fn pane_target(&self, pane_id: PaneId) -> Result<ZellijPaneTarget, BackendError> {
        self.snapshot()?
            .pane_targets
            .remove(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij pane {pane_id:?}")))
    }

    fn screen_snapshot_inner(&self, pane_id: PaneId) -> Result<ScreenSnapshot, BackendError> {
        let pane_target = self.pane_target(pane_id)?;
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
}

struct ZellijSessionSnapshot {
    topology: TopologySnapshot,
    pane_targets: HashMap<PaneId, ZellijPaneTarget>,
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
    title: Option<String>,
    rows: u16,
    cols: u16,
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
    serde_json::from_str(output)
        .map_err(|error| BackendError::internal(format!("invalid zellij list-tabs json: {error}")))
}

fn parse_panes_json(output: &str) -> Result<Vec<ZellijPaneRow>, BackendError> {
    serde_json::from_str(output)
        .map_err(|error| BackendError::internal(format!("invalid zellij list-panes json: {error}")))
}

fn build_session_snapshot(
    session_id: SessionId,
    target: &ZellijTarget,
    tabs: &[ZellijTabRow],
    panes: &[ZellijPaneRow],
) -> Result<ZellijSessionSnapshot, BackendError> {
    let mut tabs = tabs.to_vec();
    tabs.sort_by_key(|tab| tab.position);

    let mut pane_targets = HashMap::new();
    let mut topology_tabs = Vec::new();
    let mut focused_tab = None;
    let focused_tab_from_pane = panes.iter().find(|pane| pane.is_focused).map(|pane| pane.tab_id);

    for tab in tabs {
        let mut tab_panes: Vec<ZellijPaneRow> = panes
            .iter()
            .filter(|pane| pane.tab_id == tab.tab_id && !pane.is_floating)
            .cloned()
            .collect();
        if tab_panes.is_empty() {
            continue;
        }

        tab_panes.sort_by_key(|pane| (pane.pane_y, pane.pane_x, pane.id));
        let tab_id = deterministic_tab_id(target, tab.tab_id, tab.position);
        let pane_ids: Vec<PaneId> = tab_panes
            .iter()
            .map(|pane| {
                let pane_id = deterministic_pane_id(target, tab.tab_id, &pane.backend_ref());
                pane_targets.insert(
                    pane_id,
                    ZellijPaneTarget {
                        backend_ref: pane.backend_ref(),
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

        if focused_tab.is_none() && (tab.active || focused_tab_from_pane == Some(tab.tab_id)) {
            focused_tab = Some(tab_id);
        }

        topology_tabs.push((
            tab.position,
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

#[cfg(test)]
mod tests {
    use terminal_domain::{RouteAuthority, SessionRoute};

    use super::{
        ZELLIJ_ROUTE_NAMESPACE, ZellijPaneRow, ZellijProbe, ZellijSurface, ZellijTabRow,
        ZellijTarget, build_session_snapshot, parse_panes_json, parse_semver_triplet,
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
        assert_eq!(snapshot.pane_targets.len(), 3);
        assert!(snapshot.pane_targets.values().any(|pane| pane.backend_ref == "plugin_2"));
    }

    fn collect_pane_ids(root: &terminal_mux_domain::PaneTreeNode) -> Vec<terminal_domain::PaneId> {
        let mut pane_ids = Vec::new();
        collect_pane_ids_inner(root, &mut pane_ids);
        pane_ids
    }

    fn collect_pane_ids_inner(
        root: &terminal_mux_domain::PaneTreeNode,
        pane_ids: &mut Vec<terminal_domain::PaneId>,
    ) {
        match root {
            terminal_mux_domain::PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
            terminal_mux_domain::PaneTreeNode::Split(split) => {
                collect_pane_ids_inner(&split.first, pane_ids);
                collect_pane_ids_inner(&split.second, pane_ids);
            }
        }
    }
}
