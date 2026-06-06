use std::sync::{Arc, Mutex, atomic::AtomicU64};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use terminal_backend_api::{BackendError, ShellLaunchSpec};
use terminal_domain::{PaneId, TabId};
use tokio::sync::{broadcast, watch};

use crate::{emulator::EmulatorBuffer, transcript::TranscriptBuffer};

use super::{
    super::model::{
        NativePaneLayoutNode, NativePaneRuntime, NativeProjectionState, NativePtyProcess,
        NativeTabRuntime, PaneGeometry,
    },
    reader::spawn_reader_thread,
};

pub(in crate::engine) fn spawn_tab(
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

pub(in crate::engine) fn spawn_pane(
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
    let raw_output_sequence = Arc::new(AtomicU64::new(0));
    let (raw_output_tick, _) = broadcast::channel(1024);
    let (surface_tick, _) = watch::channel(0_u64);

    spawn_reader_thread(
        pane_id,
        reader,
        Arc::clone(&writer),
        Arc::clone(&transcript),
        Arc::clone(&emulator),
        Arc::clone(&raw_output_sequence),
        raw_output_tick.clone(),
        surface_tick.clone(),
    );

    Ok(NativePaneRuntime {
        pane_id,
        emulator,
        _transcript: transcript,
        raw_output_tick,
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
