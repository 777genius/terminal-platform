use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use terminal_backend_api::{BackendRawOutputEvent, BackendSessionSummary, ShellLaunchSpec};
use terminal_domain::{PaneId, TabId};
use tokio::sync::{broadcast, watch};

use crate::{emulator::EmulatorBuffer, transcript::TranscriptBuffer};

pub(super) struct NativeSessionState {
    pub(super) summary: BackendSessionSummary,
    pub(super) launch: ShellLaunchSpec,
    pub(super) tabs: Vec<NativeTabRuntime>,
    pub(super) focused_tab: TabId,
    pub(super) rows: u16,
    pub(super) cols: u16,
}

pub(super) struct NativeTabRuntime {
    pub(super) tab_id: TabId,
    pub(super) title: Option<String>,
    pub(super) focused_pane: PaneId,
    pub(super) root: NativePaneLayoutNode,
    pub(super) panes: Vec<NativePaneRuntime>,
}

pub(super) enum NativePaneLayoutNode {
    Leaf { pane_id: PaneId },
    Split(NativePaneLayoutSplit),
}

pub(super) struct NativePaneLayoutSplit {
    pub(super) direction: terminal_mux_domain::SplitDirection,
    pub(super) ratio_bps: u16,
    pub(super) first: Box<NativePaneLayoutNode>,
    pub(super) second: Box<NativePaneLayoutNode>,
}

pub(super) struct NativePaneRuntime {
    pub(super) pane_id: PaneId,
    pub(super) emulator: Arc<EmulatorBuffer>,
    pub(super) _transcript: Arc<TranscriptBuffer>,
    pub(super) raw_output_tick: broadcast::Sender<BackendRawOutputEvent>,
    pub(super) projection: Mutex<NativeProjectionState>,
    pub(super) geometry: Mutex<PaneGeometry>,
    pub(super) surface_tick: watch::Sender<u64>,
    pub(super) process: Mutex<NativePtyProcess>,
}

#[derive(Default)]
pub(super) struct NativeProjectionState {
    pub(super) history: VecDeque<terminal_projection::ScreenSnapshot>,
}

#[derive(Clone, Copy)]
pub(super) struct PaneGeometry {
    pub(super) rows: u16,
    pub(super) cols: u16,
}

#[derive(Default)]
pub(super) struct LayoutResizeOutcome {
    pub(super) changed: bool,
    pub(super) row_applied: bool,
    pub(super) col_applied: bool,
}

pub(super) struct NativePtyProcess {
    pub(super) master: Box<dyn portable_pty::MasterPty + Send>,
    pub(super) writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    pub(super) child: Box<dyn portable_pty::Child + Send + Sync>,
}
