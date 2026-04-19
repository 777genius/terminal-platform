use std::{
    collections::VecDeque,
    io::{Read as _, Write as _},
    sync::{Arc, Mutex},
    thread,
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use terminal_backend_api::{
    BackendError, BackendSessionSummary, CreateSessionSpec, MuxCommand, MuxCommandResult,
    NewTabSpec, ResizePaneSpec, SendInputSpec, SendPasteSpec, ShellLaunchSpec, SplitPaneSpec,
};
use terminal_domain::{PaneId, SessionId, SessionRoute, TabId};
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};
use terminal_projection::{ProjectionSource, ScreenDelta, ScreenSnapshot, TopologySnapshot};
use tokio::sync::watch;

use crate::{emulator::EmulatorBuffer, transcript::TranscriptBuffer};

const DEFAULT_ROWS: u16 = 24;
const DEFAULT_COLS: u16 = 80;
const SNAPSHOT_HISTORY_LIMIT: usize = 64;

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
    root: PaneTreeNode,
    panes: Vec<NativePaneRuntime>,
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

struct NativePtyProcess {
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn std::io::Write + Send>,
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
                    root: tab.root.clone(),
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
                let surface_updates = if changed { vec![pane_id] } else { Vec::new() };
                (changed, surface_updates)
            }
            MuxCommand::SendInput(spec) => (dispatch_send_input(&state, spec)?, Vec::new()),
            MuxCommand::SendPaste(spec) => (dispatch_send_paste(&state, spec)?, Vec::new()),
            MuxCommand::Detach | MuxCommand::SaveSession | MuxCommand::OverrideLayout(_) => {
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
        root: PaneTreeNode::Leaf { pane_id },
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
    let emulator = Arc::new(EmulatorBuffer::new(rows, cols));
    let transcript = Arc::new(TranscriptBuffer::default());
    let pane_id = PaneId::new();
    let (surface_tick, _) = watch::channel(0_u64);

    spawn_reader_thread(
        reader,
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
        self.panes.iter().map(|pane| pane.pane_id).collect()
    }

    fn contains_pane(&self, pane_id: PaneId) -> bool {
        self.panes.iter().any(|pane| pane.pane_id == pane_id)
    }
}

fn replace_leaf_with_split(
    node: &mut PaneTreeNode,
    target: PaneId,
    direction: SplitDirection,
    new_pane: PaneId,
) -> bool {
    match node {
        PaneTreeNode::Leaf { pane_id } if *pane_id == target => {
            *node = PaneTreeNode::Split(PaneSplit {
                direction,
                first: Box::new(PaneTreeNode::Leaf { pane_id: *pane_id }),
                second: Box::new(PaneTreeNode::Leaf { pane_id: new_pane }),
            });
            true
        }
        PaneTreeNode::Leaf { .. } => false,
        PaneTreeNode::Split(split) => {
            replace_leaf_with_split(&mut split.first, target, direction, new_pane)
                || replace_leaf_with_split(&mut split.second, target, direction, new_pane)
        }
    }
}

fn remove_leaf(node: &PaneTreeNode, target: PaneId) -> Option<PaneTreeNode> {
    match node {
        PaneTreeNode::Leaf { pane_id } => (*pane_id != target).then_some(node.clone()),
        PaneTreeNode::Split(split) => {
            match (remove_leaf(&split.first, target), remove_leaf(&split.second, target)) {
                (Some(first), Some(second)) => Some(PaneTreeNode::Split(PaneSplit {
                    direction: split.direction,
                    first: Box::new(first),
                    second: Box::new(second),
                })),
                (Some(node), None) | (None, Some(node)) => Some(node),
                (None, None) => None,
            }
        }
    }
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

fn reflow_tab_layout(tab: &NativeTabRuntime, rows: u16, cols: u16) -> Result<(), BackendError> {
    apply_pane_layout(&tab.root, tab, rows.max(1), cols.max(1))
}

fn apply_pane_layout(
    node: &PaneTreeNode,
    tab: &NativeTabRuntime,
    rows: u16,
    cols: u16,
) -> Result<(), BackendError> {
    match node {
        PaneTreeNode::Leaf { pane_id } => {
            let pane = tab.pane(*pane_id).ok_or_else(|| {
                BackendError::internal(format!(
                    "native pane tree references missing pane {pane_id:?}"
                ))
            })?;
            pane.resize(rows, cols)?;
            Ok(())
        }
        PaneTreeNode::Split(split) => match split.direction {
            SplitDirection::Vertical => {
                let (first_cols, second_cols) = partition_dimension(cols);
                apply_pane_layout(&split.first, tab, rows, first_cols)?;
                apply_pane_layout(&split.second, tab, rows, second_cols)
            }
            SplitDirection::Horizontal => {
                let (first_rows, second_rows) = partition_dimension(rows);
                apply_pane_layout(&split.first, tab, first_rows, cols)?;
                apply_pane_layout(&split.second, tab, second_rows, cols)
            }
        },
    }
}

fn partition_dimension(total: u16) -> (u16, u16) {
    if total <= 1 {
        return (1, 1);
    }

    let first = (total / 2).max(1);
    let second = total.saturating_sub(first).max(1);
    (first, second)
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
    if !replace_leaf_with_split(&mut tab.root, spec.pane_id, spec.direction, new_pane_id) {
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

    let Some(new_root) = remove_leaf(&tab.root, pane_id) else {
        return Err(BackendError::not_found(format!("unknown pane {pane_id:?}")));
    };
    tab.root = new_root;
    tab.panes.retain(|pane| pane.pane_id != pane_id);
    if tab.focused_pane == pane_id {
        tab.focused_pane = collect_pane_ids(&tab.root)
            .into_iter()
            .next()
            .ok_or_else(|| BackendError::internal("native tab root lost all panes"))?;
    }
    reflow_tab_layout(tab, state.rows, state.cols)?;

    Ok(true)
}

fn dispatch_resize_pane(
    state: &mut NativeSessionState,
    spec: ResizePaneSpec,
) -> Result<bool, BackendError> {
    let pane = state
        .tabs
        .iter()
        .find_map(|tab| tab.pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    let current = pane.geometry()?;
    if current.rows == spec.rows && current.cols == spec.cols {
        return Ok(false);
    }
    pane.resize(spec.rows, spec.cols)?;
    Ok(true)
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
    pane.write_all(spec.data.as_bytes())?;
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
    pane.write_all(spec.data.as_bytes())?;
    Ok(false)
}

impl NativePaneRuntime {
    fn write_all(&self, bytes: &[u8]) -> Result<(), BackendError> {
        let mut process = self
            .process
            .lock()
            .map_err(|_| BackendError::internal("native pane process lock poisoned"))?;
        process.writer.write_all(bytes).map_err(|error| {
            BackendError::transport(format!("failed to write to pty - {error}"))
        })?;
        process.writer.flush().map_err(|error| {
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

fn bump_watch(sender: &watch::Sender<u64>) {
    let next = sender.borrow().wrapping_add(1);
    let _ = sender.send(next);
}
