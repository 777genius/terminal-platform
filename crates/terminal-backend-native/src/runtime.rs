use std::{
    collections::VecDeque,
    io::{Read as _, Write as _},
    sync::{Arc, Mutex},
    thread,
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use terminal_backend_api::{
    BackendError, BackendSessionSummary, CreateSessionSpec, MuxCommand, MuxCommandResult,
    NewTabSpec, ResizePaneSpec, SendInputSpec, SendPasteSpec, ShellLaunchSpec,
};
use terminal_domain::{PaneId, SessionId, SessionRoute, TabId};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
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
    pane: NativePaneRuntime,
}

struct NativePaneRuntime {
    pane_id: PaneId,
    emulator: Arc<EmulatorBuffer>,
    _transcript: Arc<TranscriptBuffer>,
    projection: Mutex<NativeProjectionState>,
    surface_tick: watch::Sender<u64>,
    process: Mutex<NativePtyProcess>,
}

#[derive(Default)]
struct NativeProjectionState {
    history: VecDeque<ScreenSnapshot>,
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
                    root: PaneTreeNode::Leaf { pane_id: tab.pane.pane_id },
                    focused_pane: Some(tab.pane.pane_id),
                })
                .collect(),
        })
    }

    pub(super) fn screen_snapshot(&self, pane_id: PaneId) -> Result<ScreenSnapshot, BackendError> {
        let state = self.lock_state()?;
        let tab = state
            .tabs
            .iter()
            .find(|tab| tab.pane.pane_id == pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        tab.pane.render_snapshot(
            tab.title.clone().or_else(|| state.summary.title.clone()),
            state.rows,
            state.cols,
        )
    }

    pub(super) fn screen_delta(
        &self,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let state = self.lock_state()?;
        let tab = state
            .tabs
            .iter()
            .find(|tab| tab.pane.pane_id == pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        tab.pane.screen_delta(
            tab.title.clone().or_else(|| state.summary.title.clone()),
            state.rows,
            state.cols,
            from_sequence,
        )
    }

    pub(super) fn dispatch(&self, command: MuxCommand) -> Result<MuxCommandResult, BackendError> {
        let mut state = self.lock_state()?;

        let (changed, surface_updates) = match command {
            MuxCommand::NewTab(spec) => (dispatch_new_tab(&mut state, spec)?, Vec::new()),
            MuxCommand::FocusTab { tab_id } => {
                (dispatch_focus_tab(&mut state, tab_id)?, Vec::new())
            }
            MuxCommand::RenameTab { tab_id, title } => {
                dispatch_rename_tab(&mut state, tab_id, title)?
            }
            MuxCommand::FocusPane { pane_id } => {
                (dispatch_focus_pane(&mut state, pane_id)?, Vec::new())
            }
            MuxCommand::CloseTab { tab_id } => {
                (dispatch_close_tab(&mut state, tab_id)?, Vec::new())
            }
            MuxCommand::ResizePane(spec) => {
                let changed = dispatch_resize_pane(&mut state, spec)?;
                let surface_updates = if changed {
                    state.tabs.iter().map(|tab| tab.pane.pane_id).collect()
                } else {
                    Vec::new()
                };
                (changed, surface_updates)
            }
            MuxCommand::SendInput(spec) => (dispatch_send_input(&state, spec)?, Vec::new()),
            MuxCommand::SendPaste(spec) => (dispatch_send_paste(&state, spec)?, Vec::new()),
            MuxCommand::ClosePane { .. }
            | MuxCommand::SplitPane(_)
            | MuxCommand::Detach
            | MuxCommand::SaveSession
            | MuxCommand::OverrideLayout(_) => {
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
            if let Some(tab) = state.tabs.iter().find(|tab| tab.pane.pane_id == pane_id) {
                tab.pane.mark_surface_dirty();
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
        let tab = state
            .tabs
            .iter()
            .find(|tab| tab.pane.pane_id == pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        Ok(tab.pane.surface_tick.subscribe())
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

    Ok(NativeTabRuntime {
        tab_id: TabId::new(),
        title,
        pane: NativePaneRuntime {
            pane_id,
            emulator,
            _transcript: transcript,
            projection: Mutex::new(NativeProjectionState::default()),
            surface_tick,
            process: Mutex::new(NativePtyProcess { master: pty_pair.master, writer, child }),
        },
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

    fn render_snapshot(
        &self,
        title: Option<String>,
        rows: u16,
        cols: u16,
    ) -> Result<ScreenSnapshot, BackendError> {
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
        rows: u16,
        cols: u16,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let current = self.render_snapshot(title, rows, cols)?;
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
    Ok((true, vec![tab.pane.pane_id]))
}

fn dispatch_focus_pane(
    state: &mut NativeSessionState,
    pane_id: PaneId,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter()
        .find(|tab| tab.pane.pane_id == pane_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

    if state.focused_tab == tab.tab_id {
        return Ok(false);
    }

    state.focused_tab = tab.tab_id;
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

fn dispatch_resize_pane(
    state: &mut NativeSessionState,
    spec: ResizePaneSpec,
) -> Result<bool, BackendError> {
    if !state.tabs.iter().any(|tab| tab.pane.pane_id == spec.pane_id) {
        return Err(BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)));
    }

    if state.rows == spec.rows && state.cols == spec.cols {
        return Ok(false);
    }

    for tab in &state.tabs {
        tab.pane.resize(spec.rows, spec.cols)?;
    }
    state.rows = spec.rows;
    state.cols = spec.cols;
    Ok(true)
}

fn dispatch_send_input(
    state: &NativeSessionState,
    spec: SendInputSpec,
) -> Result<bool, BackendError> {
    let pane = state
        .tabs
        .iter()
        .find(|tab| tab.pane.pane_id == spec.pane_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    pane.pane.write_all(spec.data.as_bytes())?;
    Ok(false)
}

fn dispatch_send_paste(
    state: &NativeSessionState,
    spec: SendPasteSpec,
) -> Result<bool, BackendError> {
    let pane = state
        .tabs
        .iter()
        .find(|tab| tab.pane.pane_id == spec.pane_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    pane.pane.write_all(spec.data.as_bytes())?;
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
        self.emulator.resize(rows, cols);
        self.mark_surface_dirty();
        Ok(())
    }
}

fn bump_watch(sender: &watch::Sender<u64>) {
    let next = sender.borrow().wrapping_add(1);
    let _ = sender.send(next);
}
