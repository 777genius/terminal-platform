use std::{
    io::{Read as _, Write as _},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use terminal_backend_api::{
    BackendError, BackendRawOutputBytes, BackendRawOutputEvent, ShellLaunchSpec,
};
use terminal_domain::{PaneId, TabId};
use tokio::sync::{broadcast, watch};

use crate::{emulator::EmulatorBuffer, transcript::TranscriptBuffer};

use super::{
    model::{
        NativePaneLayoutNode, NativePaneRuntime, NativeProjectionState, NativePtyProcess,
        NativeTabRuntime, PaneGeometry,
    },
    signals::bump_watch,
};

impl Drop for NativePtyProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub(super) fn resolve_launch_spec(
    spec: Option<ShellLaunchSpec>,
) -> Result<ShellLaunchSpec, BackendError> {
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

pub(super) fn spawn_tab(
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

pub(super) fn spawn_pane(
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

fn spawn_reader_thread(
    pane_id: PaneId,
    mut reader: Box<dyn std::io::Read + Send>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    transcript: Arc<TranscriptBuffer>,
    emulator: Arc<EmulatorBuffer>,
    raw_output_sequence: Arc<AtomicU64>,
    raw_output_tick: broadcast::Sender<BackendRawOutputEvent>,
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
                    let sequence = raw_output_sequence.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ =
                        raw_output_tick.send(BackendRawOutputEvent::Bytes(BackendRawOutputBytes {
                            pane_id,
                            sequence,
                            payload: chunk[..read].to_vec(),
                        }));
                    bump_watch(&surface_tick);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });
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
