use std::{
    borrow::Cow,
    collections::{HashSet, VecDeque},
    io::{Read as _, Write as _},
    sync::{Arc, Mutex},
    thread,
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use terminal_backend_api::{
    BackendError, BackendSessionSummary, CreateSessionSpec, MuxCommand, MuxCommandResult,
    NewTabSpec, OverrideLayoutSpec, ResizePaneSpec, SendInputSpec, SendPasteSpec, ShellLaunchSpec,
    SplitPaneSpec,
};
use terminal_domain::{PaneId, SessionId, SessionRoute, TabId};
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};
use terminal_projection::{ProjectionSource, ScreenDelta, ScreenSnapshot, TopologySnapshot};
use tokio::sync::watch;

use crate::{emulator::EmulatorBuffer, transcript::TranscriptBuffer};

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;
const SNAPSHOT_HISTORY_LIMIT: usize = 64;
const SPLIT_RATIO_SCALE: u16 = 10_000;
const DEFAULT_SPLIT_RATIO_BPS: u16 = SPLIT_RATIO_SCALE / 2;

pub(super) struct NativeSessionRuntime {
    session_id: SessionId,
    state: Mutex<NativeSessionState>,
    topology_tick: watch::Sender<u64>,
}

struct NativeSessionState {
    summary: BackendSessionSummary,
    launch: ShellLaunchSpec,
    tabs: Vec<NativeTabRuntime>,
    focused_tab: TabId,
    rows: u16,
    cols: u16,
}

struct NativeTabRuntime {
    tab_id: TabId,
    title: Option<String>,
    focused_pane: PaneId,
    root: NativePaneLayoutNode,
    panes: Vec<NativePaneRuntime>,
}

enum NativePaneLayoutNode {
    Leaf { pane_id: PaneId },
    Split(NativePaneLayoutSplit),
}

struct NativePaneLayoutSplit {
    direction: SplitDirection,
    ratio_bps: u16,
    first: Box<NativePaneLayoutNode>,
    second: Box<NativePaneLayoutNode>,
}

struct NativePaneRuntime {
    pane_id: PaneId,
    emulator: Arc<EmulatorBuffer>,
    _transcript: Arc<TranscriptBuffer>,
    projection: Mutex<NativeProjectionState>,
    geometry: Mutex<PaneGeometry>,
    surface_tick: watch::Sender<u64>,
    process: Mutex<NativePtyProcess>,
}

#[derive(Default)]
struct NativeProjectionState {
    history: VecDeque<ScreenSnapshot>,
}

#[derive(Clone, Copy)]
struct PaneGeometry {
    rows: u16,
    cols: u16,
}

#[derive(Default)]
struct LayoutResizeOutcome {
    changed: bool,
    row_applied: bool,
    col_applied: bool,
}

struct NativePtyProcess {
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl NativeSessionRuntime {
    pub(super) fn spawn(
        session_id: SessionId,
        route: SessionRoute,
        spec: CreateSessionSpec,
    ) -> Result<Self, BackendError> {
        let launch = resolve_launch_spec(spec.launch)?;
        let (topology_tick, _) = watch::channel(0_u64);
        let first_tab = spawn_tab(spec.title.clone(), &launch, DEFAULT_ROWS, DEFAULT_COLS)?;
        let summary = BackendSessionSummary { session_id, route, title: spec.title };

        Ok(Self {
            session_id,
            state: Mutex::new(NativeSessionState {
                summary,
                launch,
                focused_tab: first_tab.tab_id,
                tabs: vec![first_tab],
                rows: DEFAULT_ROWS,
                cols: DEFAULT_COLS,
            }),
            topology_tick,
        })
    }

    pub(super) fn summary(&self) -> Result<BackendSessionSummary, BackendError> {
        Ok(self.lock_state()?.summary.clone())
    }

    pub(super) fn topology_snapshot(&self) -> Result<TopologySnapshot, BackendError> {
        let state = self.lock_state()?;

        Ok(TopologySnapshot {
            session_id: self.session_id,
            backend_kind: terminal_domain::BackendKind::Native,
            focused_tab: Some(state.focused_tab),
            tabs: state
                .tabs
                .iter()
                .map(|tab| TabSnapshot {
                    tab_id: tab.tab_id,
                    title: tab.title.clone(),
                    root: tab.root.snapshot(),
                    focused_pane: Some(tab.focused_pane),
                })
                .collect(),
        })
    }

    pub(super) fn screen_snapshot(&self, pane_id: PaneId) -> Result<ScreenSnapshot, BackendError> {
        let state = self.lock_state()?;
        let (tab, pane) = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id).map(|pane| (tab, pane)))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        pane.render_snapshot(tab.title.clone().or_else(|| state.summary.title.clone()))
    }

    pub(super) fn screen_delta(
        &self,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let state = self.lock_state()?;
        let (tab, pane) = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id).map(|pane| (tab, pane)))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        pane.screen_delta(tab.title.clone().or_else(|| state.summary.title.clone()), from_sequence)
    }

    pub(super) fn dispatch(&self, command: MuxCommand) -> Result<MuxCommandResult, BackendError> {
        let mut state = self.lock_state()?;

        let (changed, surface_updates) = match command {
            MuxCommand::NewTab(spec) => (dispatch_new_tab(&mut state, spec)?, Vec::new()),
            MuxCommand::SplitPane(spec) => (dispatch_split_pane(&mut state, spec)?, Vec::new()),
            MuxCommand::FocusTab { tab_id } => {
                (dispatch_focus_tab(&mut state, tab_id)?, Vec::new())
            }
            MuxCommand::RenameTab { tab_id, title } => {
                dispatch_rename_tab(&mut state, tab_id, title)?
            }
            MuxCommand::FocusPane { pane_id } => {
                (dispatch_focus_pane(&mut state, pane_id)?, Vec::new())
            }
            MuxCommand::ClosePane { pane_id } => {
                (dispatch_close_pane(&mut state, pane_id)?, Vec::new())
            }
            MuxCommand::CloseTab { tab_id } => {
                (dispatch_close_tab(&mut state, tab_id)?, Vec::new())
            }
            MuxCommand::ResizePane(spec) => {
                let pane_id = spec.pane_id;
                let changed = dispatch_resize_pane(&mut state, spec)?;
                let surface_updates = if changed {
                    state
                        .tabs
                        .iter()
                        .find(|tab| tab.contains_pane(pane_id))
                        .map_or_else(Vec::new, NativeTabRuntime::pane_ids)
                } else {
                    Vec::new()
                };
                (changed, surface_updates)
            }
            MuxCommand::OverrideLayout(spec) => dispatch_override_layout(&mut state, spec)?,
            MuxCommand::SendInput(spec) => (dispatch_send_input(&state, spec)?, Vec::new()),
            MuxCommand::SendPaste(spec) => (dispatch_send_paste(&state, spec)?, Vec::new()),
            MuxCommand::Detach | MuxCommand::SaveSession => {
                return Err(BackendError::unsupported(
                    "native mux command is not wired in v1 start phase",
                    terminal_domain::DegradedModeReason::NotYetImplemented,
                ));
            }
        };

        if changed {
            bump_watch(&self.topology_tick);
        }
        for pane_id in surface_updates {
            if let Some(pane) = state.tabs.iter().find_map(|tab| tab.pane(pane_id)) {
                pane.mark_surface_dirty();
            }
        }
        Ok(MuxCommandResult { changed })
    }

    pub(super) fn subscribe_topology(&self) -> watch::Receiver<u64> {
        self.topology_tick.subscribe()
    }

    pub(super) fn subscribe_pane_surface(
        &self,
        pane_id: PaneId,
    ) -> Result<watch::Receiver<u64>, BackendError> {
        let state = self.lock_state()?;
        let pane = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        Ok(pane.surface_tick.subscribe())
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, NativeSessionState>, BackendError> {
        self.state.lock().map_err(|_| BackendError::internal("native session state lock poisoned"))
    }
}

impl Drop for NativePtyProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn resolve_launch_spec(spec: Option<ShellLaunchSpec>) -> Result<ShellLaunchSpec, BackendError> {
    match spec {
        Some(spec) if spec.program.trim().is_empty() => {
            Err(BackendError::invalid_input("shell launch program cannot be empty"))
        }
        Some(spec) => Ok(spec),
        None => Ok(default_launch_spec()),
    }
}

#[cfg(unix)]
fn default_launch_spec() -> ShellLaunchSpec {
    let program = std::env::var("SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string());

    ShellLaunchSpec::new(program)
}

#[cfg(windows)]
fn default_launch_spec() -> ShellLaunchSpec {
    let program = std::env::var("COMSPEC")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "cmd.exe".to_string());

    ShellLaunchSpec::new(program)
}

fn spawn_tab(
    title: Option<String>,
    launch: &ShellLaunchSpec,
    rows: u16,
    cols: u16,
) -> Result<NativeTabRuntime, BackendError> {
    let pane = spawn_pane(launch, rows, cols)?;
    let pane_id = pane.pane_id;

    Ok(NativeTabRuntime {
        tab_id: TabId::new(),
        title,
        focused_pane: pane_id,
        root: NativePaneLayoutNode::Leaf { pane_id },
        panes: vec![pane],
    })
}

fn spawn_pane(
    launch: &ShellLaunchSpec,
    rows: u16,
    cols: u16,
) -> Result<NativePaneRuntime, BackendError> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|error| BackendError::transport(format!("failed to open native pty - {error}")))?;
    let command = build_command(launch);
    let child = pty_pair.slave.spawn_command(command).map_err(|error| {
        BackendError::transport(format!(
            "failed to spawn native shell `{}` - {error}",
            launch.program
        ))
    })?;
    let reader = pty_pair.master.try_clone_reader().map_err(|error| {
        BackendError::transport(format!("failed to clone pty reader - {error}"))
    })?;
    let writer = pty_pair
        .master
        .take_writer()
        .map_err(|error| BackendError::transport(format!("failed to take pty writer - {error}")))?;
    let writer = Arc::new(Mutex::new(writer));
    let emulator = Arc::new(EmulatorBuffer::new(rows, cols));
    let transcript = Arc::new(TranscriptBuffer::default());
    let pane_id = PaneId::new();
    let (surface_tick, _) = watch::channel(0_u64);

    spawn_reader_thread(
        reader,
        Arc::clone(&writer),
        Arc::clone(&transcript),
        Arc::clone(&emulator),
        surface_tick.clone(),
    );

    Ok(NativePaneRuntime {
        pane_id,
        emulator,
        _transcript: transcript,
        projection: Mutex::new(NativeProjectionState::default()),
        geometry: Mutex::new(PaneGeometry { rows, cols }),
        surface_tick,
        process: Mutex::new(NativePtyProcess { master: pty_pair.master, writer, child }),
    })
}

fn build_command(launch: &ShellLaunchSpec) -> CommandBuilder {
    let mut command = CommandBuilder::new(&launch.program);
    for arg in &launch.args {
        command.arg(arg);
    }
    if let Some(cwd) = &launch.cwd {
        command.cwd(cwd);
    }
    command
}

fn spawn_reader_thread(
    mut reader: Box<dyn std::io::Read + Send>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    transcript: Arc<TranscriptBuffer>,
    emulator: Arc<EmulatorBuffer>,
    surface_tick: watch::Sender<u64>,
) {
    thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => {
                    respond_to_cursor_inherit_query(&chunk[..read], &writer);
                    transcript.append(&chunk[..read]);
                    emulator.advance(&chunk[..read]);
                    bump_watch(&surface_tick);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });
}

impl NativePaneRuntime {
    fn mark_surface_dirty(&self) {
        bump_watch(&self.surface_tick);
    }

    fn render_snapshot(&self, title: Option<String>) -> Result<ScreenSnapshot, BackendError> {
        let geometry = self.geometry()?;
        let rows = geometry.rows;
        let cols = geometry.cols;
        let rendered = self.emulator.render(title.clone());
        let mut projection = self
            .projection
            .lock()
            .map_err(|_| BackendError::internal("native pane projection state lock poisoned"))?;

        if let Some(current) = projection.history.back()
            && current.rows == rows
            && current.cols == cols
            && current.source == ProjectionSource::NativeEmulator
            && current.surface == rendered.surface
        {
            return Ok(current.clone());
        }

        let sequence = projection.history.back().map_or(1, |snapshot| snapshot.sequence + 1);
        let snapshot = ScreenSnapshot {
            pane_id: self.pane_id,
            sequence,
            rows,
            cols,
            source: ProjectionSource::NativeEmulator,
            surface: rendered.surface,
        };

        projection.history.push_back(snapshot.clone());
        while projection.history.len() > SNAPSHOT_HISTORY_LIMIT {
            projection.history.pop_front();
        }

        Ok(snapshot)
    }

    fn screen_delta(
        &self,
        title: Option<String>,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let current = self.render_snapshot(title)?;
        if current.sequence == from_sequence {
            return Ok(ScreenDelta::unchanged_from(&current));
        }

        let projection = self
            .projection
            .lock()
            .map_err(|_| BackendError::internal("native pane projection state lock poisoned"))?;
        let previous =
            projection.history.iter().find(|snapshot| snapshot.sequence == from_sequence);

        Ok(match previous {
            Some(previous) => ScreenDelta::between(previous, &current),
            None => ScreenDelta::full_replace(from_sequence, &current),
        })
    }

    fn geometry(&self) -> Result<PaneGeometry, BackendError> {
        self.geometry
            .lock()
            .map(|geometry| *geometry)
            .map_err(|_| BackendError::internal("native pane geometry lock poisoned"))
    }
}

impl NativeTabRuntime {
    fn pane(&self, pane_id: PaneId) -> Option<&NativePaneRuntime> {
        self.panes.iter().find(|pane| pane.pane_id == pane_id)
    }

    fn pane_ids(&self) -> Vec<PaneId> {
        self.root.pane_ids()
    }

    fn contains_pane(&self, pane_id: PaneId) -> bool {
        self.root.contains_pane(pane_id)
    }

    fn first_pane_id(&self) -> Option<PaneId> {
        self.root.first_pane_id()
    }
}

impl NativePaneLayoutNode {
    fn from_snapshot(root: PaneTreeNode) -> Self {
        match root {
            PaneTreeNode::Leaf { pane_id } => Self::Leaf { pane_id },
            PaneTreeNode::Split(split) => Self::Split(NativePaneLayoutSplit {
                direction: split.direction,
                ratio_bps: DEFAULT_SPLIT_RATIO_BPS,
                first: Box::new(Self::from_snapshot(*split.first)),
                second: Box::new(Self::from_snapshot(*split.second)),
            }),
        }
    }

    fn snapshot(&self) -> PaneTreeNode {
        match self {
            Self::Leaf { pane_id } => PaneTreeNode::Leaf { pane_id: *pane_id },
            Self::Split(split) => PaneTreeNode::Split(PaneSplit {
                direction: split.direction,
                first: Box::new(split.first.snapshot()),
                second: Box::new(split.second.snapshot()),
            }),
        }
    }

    fn contains_pane(&self, target: PaneId) -> bool {
        match self {
            Self::Leaf { pane_id } => *pane_id == target,
            Self::Split(split) => {
                split.first.contains_pane(target) || split.second.contains_pane(target)
            }
        }
    }

    fn pane_ids(&self) -> Vec<PaneId> {
        let mut pane_ids = Vec::new();
        self.collect_pane_ids(&mut pane_ids);
        pane_ids
    }

    fn path_has_direction(&self, target: PaneId, direction: SplitDirection) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split(split) => {
                let first_contains = split.first.contains_pane(target);
                let second_contains = split.second.contains_pane(target);
                if !first_contains && !second_contains {
                    return false;
                }

                if split.direction == direction {
                    true
                } else if first_contains {
                    split.first.path_has_direction(target, direction)
                } else {
                    split.second.path_has_direction(target, direction)
                }
            }
        }
    }

    fn first_pane_id(&self) -> Option<PaneId> {
        match self {
            Self::Leaf { pane_id } => Some(*pane_id),
            Self::Split(split) => {
                split.first.first_pane_id().or_else(|| split.second.first_pane_id())
            }
        }
    }

    fn split_leaf(&mut self, target: PaneId, direction: SplitDirection, new_pane: PaneId) -> bool {
        match self {
            Self::Leaf { pane_id } if *pane_id == target => {
                let current_pane = *pane_id;
                *self = Self::Split(NativePaneLayoutSplit {
                    direction,
                    ratio_bps: DEFAULT_SPLIT_RATIO_BPS,
                    first: Box::new(Self::Leaf { pane_id: current_pane }),
                    second: Box::new(Self::Leaf { pane_id: new_pane }),
                });
                true
            }
            Self::Leaf { .. } => false,
            Self::Split(split) => {
                split.first.split_leaf(target, direction, new_pane)
                    || split.second.split_leaf(target, direction, new_pane)
            }
        }
    }

    fn remove_leaf(&self, target: PaneId) -> Option<Self> {
        match self {
            Self::Leaf { pane_id } => {
                (*pane_id != target).then_some(Self::Leaf { pane_id: *pane_id })
            }
            Self::Split(split) => {
                match (split.first.remove_leaf(target), split.second.remove_leaf(target)) {
                    (Some(first), Some(second)) => Some(Self::Split(NativePaneLayoutSplit {
                        direction: split.direction,
                        ratio_bps: split.ratio_bps,
                        first: Box::new(first),
                        second: Box::new(second),
                    })),
                    (Some(node), None) | (None, Some(node)) => Some(node),
                    (None, None) => None,
                }
            }
        }
    }

    fn resize_target(
        &mut self,
        target: PaneId,
        desired: PaneGeometry,
        rows: u16,
        cols: u16,
    ) -> LayoutResizeOutcome {
        self.resize_target_with_policy(target, desired, rows, cols, true, true)
    }

    fn resize_target_with_policy(
        &mut self,
        target: PaneId,
        desired: PaneGeometry,
        rows: u16,
        cols: u16,
        allow_row_resize: bool,
        allow_col_resize: bool,
    ) -> LayoutResizeOutcome {
        match self {
            Self::Leaf { .. } => LayoutResizeOutcome::default(),
            Self::Split(split) => {
                let mut outcome = LayoutResizeOutcome::default();
                let first_contains = split.first.contains_pane(target);
                let second_contains = split.second.contains_pane(target);
                if !first_contains && !second_contains {
                    return outcome;
                }

                match split.direction {
                    SplitDirection::Vertical if allow_col_resize && cols > 1 => {
                        let desired_first_cols =
                            target_to_first_span(cols, desired.cols, first_contains);
                        let new_ratio = span_to_ratio_bps(desired_first_cols, cols);
                        if split.ratio_bps != new_ratio {
                            split.ratio_bps = new_ratio;
                            outcome.changed = true;
                        }
                        outcome.col_applied = true;
                    }
                    SplitDirection::Horizontal if allow_row_resize && rows > 1 => {
                        let desired_first_rows =
                            target_to_first_span(rows, desired.rows, first_contains);
                        let new_ratio = span_to_ratio_bps(desired_first_rows, rows);
                        if split.ratio_bps != new_ratio {
                            split.ratio_bps = new_ratio;
                            outcome.changed = true;
                        }
                        outcome.row_applied = true;
                    }
                    _ => {}
                }

                let ((first_rows, first_cols), (second_rows, second_cols)) =
                    split.partition(rows, cols);
                let child_allow_row =
                    allow_row_resize && split.direction != SplitDirection::Horizontal;
                let child_allow_col =
                    allow_col_resize && split.direction != SplitDirection::Vertical;
                let nested = if first_contains {
                    split.first.resize_target_with_policy(
                        target,
                        desired,
                        first_rows,
                        first_cols,
                        child_allow_row,
                        child_allow_col,
                    )
                } else {
                    split.second.resize_target_with_policy(
                        target,
                        desired,
                        second_rows,
                        second_cols,
                        child_allow_row,
                        child_allow_col,
                    )
                };
                outcome.merge(nested);
                outcome
            }
        }
    }

    fn collect_pane_ids(&self, pane_ids: &mut Vec<PaneId>) {
        match self {
            Self::Leaf { pane_id } => pane_ids.push(*pane_id),
            Self::Split(split) => {
                split.first.collect_pane_ids(pane_ids);
                split.second.collect_pane_ids(pane_ids);
            }
        }
    }
}

impl NativePaneLayoutSplit {
    fn partition(&self, rows: u16, cols: u16) -> ((u16, u16), (u16, u16)) {
        match self.direction {
            SplitDirection::Vertical => {
                let (first_cols, second_cols) = partition_dimension_by_ratio(cols, self.ratio_bps);
                ((rows, first_cols), (rows, second_cols))
            }
            SplitDirection::Horizontal => {
                let (first_rows, second_rows) = partition_dimension_by_ratio(rows, self.ratio_bps);
                ((first_rows, cols), (second_rows, cols))
            }
        }
    }
}

impl LayoutResizeOutcome {
    fn merge(&mut self, nested: Self) {
        self.changed |= nested.changed;
        self.row_applied |= nested.row_applied;
        self.col_applied |= nested.col_applied;
    }
}

fn target_to_first_span(total: u16, desired_target: u16, target_is_first: bool) -> u16 {
    if total <= 1 {
        return 1;
    }

    let clamped_target = desired_target.clamp(1, total.saturating_sub(1));
    if target_is_first {
        clamped_target
    } else {
        total.saturating_sub(clamped_target).clamp(1, total.saturating_sub(1))
    }
}

fn span_to_ratio_bps(first_span: u16, total: u16) -> u16 {
    if total <= 1 {
        return DEFAULT_SPLIT_RATIO_BPS;
    }

    let clamped_first = first_span.clamp(1, total.saturating_sub(1));
    let ratio = ((u32::from(clamped_first) * u32::from(SPLIT_RATIO_SCALE))
        + (u32::from(total) / 2))
        / u32::from(total);
    ratio
        .clamp(1, u32::from(SPLIT_RATIO_SCALE.saturating_sub(1)))
        .try_into()
        .unwrap_or(DEFAULT_SPLIT_RATIO_BPS)
}

fn partition_dimension_by_ratio(total: u16, ratio_bps: u16) -> (u16, u16) {
    if total <= 1 {
        return (1, 1);
    }

    let ratio = ratio_bps.clamp(1, SPLIT_RATIO_SCALE.saturating_sub(1));
    let mut first = ((u32::from(total) * u32::from(ratio)) + (u32::from(SPLIT_RATIO_SCALE) / 2))
        / u32::from(SPLIT_RATIO_SCALE);
    first = first.clamp(1, u32::from(total.saturating_sub(1)));
    let first: u16 = first.try_into().unwrap_or(1);
    let second = total.saturating_sub(first).max(1);
    (first, second)
}

fn reflow_tab_layout(tab: &NativeTabRuntime, rows: u16, cols: u16) -> Result<(), BackendError> {
    apply_pane_layout(&tab.root, tab, rows.max(1), cols.max(1))
}

fn apply_pane_layout(
    node: &NativePaneLayoutNode,
    tab: &NativeTabRuntime,
    rows: u16,
    cols: u16,
) -> Result<(), BackendError> {
    match node {
        NativePaneLayoutNode::Leaf { pane_id } => {
            let pane = tab.pane(*pane_id).ok_or_else(|| {
                BackendError::internal(format!(
                    "native pane tree references missing pane {pane_id:?}"
                ))
            })?;
            pane.resize(rows, cols)?;
            Ok(())
        }
        NativePaneLayoutNode::Split(split) => {
            let ((first_rows, first_cols), (second_rows, second_cols)) =
                split.partition(rows, cols);
            apply_pane_layout(&split.first, tab, first_rows, first_cols)?;
            apply_pane_layout(&split.second, tab, second_rows, second_cols)
        }
    }
}

fn dispatch_new_tab(
    state: &mut NativeSessionState,
    spec: NewTabSpec,
) -> Result<bool, BackendError> {
    let tab = spawn_tab(spec.title.clone(), &state.launch, state.rows, state.cols)?;
    state.focused_tab = tab.tab_id;
    state.tabs.push(tab);
    Ok(true)
}

fn dispatch_split_pane(
    state: &mut NativeSessionState,
    spec: SplitPaneSpec,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;

    let pane = spawn_pane(&state.launch, state.rows, state.cols)?;
    let new_pane_id = pane.pane_id;
    if !tab.root.split_leaf(spec.pane_id, spec.direction, new_pane_id) {
        return Err(BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)));
    }

    tab.focused_pane = new_pane_id;
    tab.panes.push(pane);
    state.focused_tab = tab.tab_id;
    reflow_tab_layout(tab, state.rows, state.cols)?;
    Ok(true)
}

fn dispatch_focus_tab(state: &mut NativeSessionState, tab_id: TabId) -> Result<bool, BackendError> {
    if !state.tabs.iter().any(|tab| tab.tab_id == tab_id) {
        return Err(BackendError::not_found(format!("unknown tab {tab_id:?}")));
    }

    if state.focused_tab == tab_id {
        return Ok(false);
    }

    state.focused_tab = tab_id;
    Ok(true)
}

fn dispatch_rename_tab(
    state: &mut NativeSessionState,
    tab_id: TabId,
    title: String,
) -> Result<(bool, Vec<PaneId>), BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.tab_id == tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {tab_id:?}")))?;

    if tab.title.as_deref() == Some(title.as_str()) {
        return Ok((false, Vec::new()));
    }

    tab.title = Some(title);
    Ok((true, tab.pane_ids()))
}

fn dispatch_focus_pane(
    state: &mut NativeSessionState,
    pane_id: PaneId,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

    if state.focused_tab == tab.tab_id && tab.focused_pane == pane_id {
        return Ok(false);
    }

    state.focused_tab = tab.tab_id;
    tab.focused_pane = pane_id;
    Ok(true)
}

fn dispatch_close_tab(state: &mut NativeSessionState, tab_id: TabId) -> Result<bool, BackendError> {
    if state.tabs.len() == 1 {
        return Err(BackendError::invalid_input("native session must keep at least one tab"));
    }

    let index = state
        .tabs
        .iter()
        .position(|tab| tab.tab_id == tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {tab_id:?}")))?;
    let removed_focused = state.focused_tab == tab_id;
    state.tabs.remove(index);

    if removed_focused {
        let replacement_index = index.min(state.tabs.len().saturating_sub(1));
        state.focused_tab = state.tabs[replacement_index].tab_id;
    }

    Ok(true)
}

fn dispatch_close_pane(
    state: &mut NativeSessionState,
    pane_id: PaneId,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;
    if tab.panes.len() <= 1 {
        return Err(BackendError::invalid_input("native tab must keep at least one pane"));
    }

    let Some(new_root) = tab.root.remove_leaf(pane_id) else {
        return Err(BackendError::not_found(format!("unknown pane {pane_id:?}")));
    };
    tab.root = new_root;
    tab.panes.retain(|pane| pane.pane_id != pane_id);
    if tab.focused_pane == pane_id {
        tab.focused_pane = tab
            .first_pane_id()
            .ok_or_else(|| BackendError::internal("native tab root lost all panes"))?;
    }
    reflow_tab_layout(tab, state.rows, state.cols)?;

    Ok(true)
}

fn dispatch_resize_pane(
    state: &mut NativeSessionState,
    spec: ResizePaneSpec,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    let pane = tab.pane(spec.pane_id).ok_or_else(|| {
        BackendError::internal(format!("native tab lost pane {:?}", spec.pane_id))
    })?;
    let current = pane.geometry()?;
    if current.rows == spec.rows && current.cols == spec.cols {
        return Ok(false);
    }

    if tab.panes.len() == 1 {
        pane.resize(spec.rows, spec.cols)?;
        return Ok(true);
    }

    let desired = PaneGeometry { rows: spec.rows.max(1), cols: spec.cols.max(1) };
    if desired.rows != current.rows
        && !tab.root.path_has_direction(spec.pane_id, SplitDirection::Horizontal)
    {
        return Err(BackendError::unsupported(
            "native pane resize cannot independently change rows in current layout",
            terminal_domain::DegradedModeReason::ResizeAuthorityExternal,
        ));
    }
    if desired.cols != current.cols
        && !tab.root.path_has_direction(spec.pane_id, SplitDirection::Vertical)
    {
        return Err(BackendError::unsupported(
            "native pane resize cannot independently change cols in current layout",
            terminal_domain::DegradedModeReason::ResizeAuthorityExternal,
        ));
    }
    let outcome = tab.root.resize_target(spec.pane_id, desired, state.rows, state.cols);
    if !outcome.changed {
        return Ok(false);
    }

    reflow_tab_layout(tab, state.rows, state.cols)?;
    Ok(true)
}

fn dispatch_override_layout(
    state: &mut NativeSessionState,
    spec: OverrideLayoutSpec,
) -> Result<(bool, Vec<PaneId>), BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.tab_id == spec.tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {:?}", spec.tab_id)))?;
    let current_snapshot = tab.root.snapshot();
    if current_snapshot == spec.root {
        return Ok((false, Vec::new()));
    }

    validate_layout_override(tab, &spec.root)?;
    tab.root = NativePaneLayoutNode::from_snapshot(spec.root);
    if !tab.contains_pane(tab.focused_pane) {
        tab.focused_pane = tab
            .first_pane_id()
            .ok_or_else(|| BackendError::internal("native layout override lost all panes"))?;
    }
    reflow_tab_layout(tab, state.rows, state.cols)?;

    Ok((true, tab.pane_ids()))
}

fn validate_layout_override(
    tab: &NativeTabRuntime,
    root: &PaneTreeNode,
) -> Result<(), BackendError> {
    let current_panes: HashSet<_> = tab.pane_ids().into_iter().collect();
    let requested_panes = collect_snapshot_pane_ids(root);
    let requested_unique: HashSet<_> = requested_panes.iter().copied().collect();

    if requested_panes.len() != requested_unique.len() {
        return Err(BackendError::invalid_input("layout override contains duplicate pane ids"));
    }
    if current_panes != requested_unique {
        return Err(BackendError::invalid_input(
            "layout override must preserve the exact pane set for the target tab",
        ));
    }

    Ok(())
}

fn collect_snapshot_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_snapshot_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

fn collect_snapshot_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_snapshot_pane_ids_inner(&split.first, pane_ids);
            collect_snapshot_pane_ids_inner(&split.second, pane_ids);
        }
    }
}

fn dispatch_send_input(
    state: &NativeSessionState,
    spec: SendInputSpec,
) -> Result<bool, BackendError> {
    let pane = state
        .tabs
        .iter()
        .find_map(|tab| tab.pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    pane.write_text(&spec.data)?;
    Ok(false)
}

fn dispatch_send_paste(
    state: &NativeSessionState,
    spec: SendPasteSpec,
) -> Result<bool, BackendError> {
    let pane = state
        .tabs
        .iter()
        .find_map(|tab| tab.pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    pane.write_text(&spec.data)?;
    Ok(false)
}

impl NativePaneRuntime {
    fn write_text(&self, text: &str) -> Result<(), BackendError> {
        let normalized = normalize_pty_input(text);
        self.write_all(normalized.as_bytes())
    }

    fn write_all(&self, bytes: &[u8]) -> Result<(), BackendError> {
        let process = self
            .process
            .lock()
            .map_err(|_| BackendError::internal("native pane process lock poisoned"))?;
        let mut writer = process
            .writer
            .lock()
            .map_err(|_| BackendError::internal("native pane writer lock poisoned"))?;
        writer.write_all(bytes).map_err(|error| {
            BackendError::transport(format!("failed to write to pty - {error}"))
        })?;
        writer.flush().map_err(|error| {
            BackendError::transport(format!("failed to flush pty writer - {error}"))
        })?;
        Ok(())
    }

    fn resize(&self, rows: u16, cols: u16) -> Result<(), BackendError> {
        let process = self
            .process
            .lock()
            .map_err(|_| BackendError::internal("native pane process lock poisoned"))?;
        process
            .master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|error| BackendError::transport(format!("failed to resize pty - {error}")))?;
        drop(process);
        let mut geometry = self
            .geometry
            .lock()
            .map_err(|_| BackendError::internal("native pane geometry lock poisoned"))?;
        geometry.rows = rows;
        geometry.cols = cols;
        drop(geometry);
        self.emulator.resize(rows, cols);
        self.mark_surface_dirty();
        Ok(())
    }
}

fn respond_to_cursor_inherit_query(
    chunk: &[u8],
    writer: &Arc<Mutex<Box<dyn std::io::Write + Send>>>,
) {
    #[cfg(windows)]
    {
        // CreatePseudoConsole warns that inheriting the cursor can deadlock unless the host
        // answers the cursor-position query received on the output pipe. v1 now pins the
        // vendored portable-pty path to dwFlags = 0, but keep this safeguard so unexpected
        // ConPTY hosts or future vendor drift do not wedge the pipe.
        if chunk.windows(4).any(|window| window == b"\x1b[6n")
            && let Ok(mut writer) = writer.lock()
        {
            let _ = writer.write_all(b"\x1b[1;1R");
            let _ = writer.flush();
        }
    }

    #[cfg(not(windows))]
    let _ = (chunk, writer);
}

fn normalize_pty_input(text: &str) -> Cow<'_, str> {
    #[cfg(windows)]
    {
        normalize_windows_pty_input(text)
    }

    #[cfg(not(windows))]
    {
        Cow::Borrowed(text)
    }
}

#[cfg(any(test, windows))]
fn normalize_windows_pty_input(text: &str) -> Cow<'_, str> {
    if !text.bytes().any(|byte| matches!(byte, b'\r' | b'\n')) {
        return Cow::Borrowed(text);
    }

    let mut normalized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                normalized.push('\r');
            }
            '\n' => normalized.push('\r'),
            _ => normalized.push(ch),
        }
    }

    Cow::Owned(normalized)
}

fn bump_watch(sender: &watch::Sender<u64>) {
    let next = sender.borrow().wrapping_add(1);
    let _ = sender.send(next);
}

#[cfg(test)]
mod tests {
    use super::normalize_windows_pty_input;

    #[test]
    fn normalize_windows_pty_input_preserves_plain_text() {
        assert_eq!(normalize_windows_pty_input("plain text").as_ref(), "plain text");
    }

    #[test]
    fn normalize_windows_pty_input_collapses_newline_variants_to_carriage_return() {
        assert_eq!(normalize_windows_pty_input("alpha\r\nbeta").as_ref(), "alpha\rbeta");
        assert_eq!(normalize_windows_pty_input("alpha\nbeta").as_ref(), "alpha\rbeta");
        assert_eq!(normalize_windows_pty_input("alpha\rbeta").as_ref(), "alpha\rbeta");
    }
}
