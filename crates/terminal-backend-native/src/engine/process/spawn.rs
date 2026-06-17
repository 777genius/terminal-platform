use std::{
    ffi::OsStr,
    sync::{Arc, Mutex, atomic::AtomicU64},
};

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
    reader::{ReaderThreadParts, spawn_reader_thread},
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

    spawn_reader_thread(ReaderThreadParts {
        pane_id,
        reader,
        writer: Arc::clone(&writer),
        transcript: Arc::clone(&transcript),
        emulator: Arc::clone(&emulator),
        raw_output_sequence: Arc::clone(&raw_output_sequence),
        raw_output_tick: raw_output_tick.clone(),
        surface_tick: surface_tick.clone(),
    });

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
    apply_terminal_capability_environment(&mut command);
    command
}

fn apply_terminal_capability_environment(command: &mut CommandBuilder) {
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("TERM_PROGRAM", "terminal-platform");
    command.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    command.env("TERMINAL_PLATFORM", "1");
    command.env("TERM_FEATURES", crate::TERMINAL_FEATURE_REPORT);
    if should_advertise_cli_color_hint_from_env() {
        command.env("CLICOLOR", "1");
    }
}

fn should_advertise_cli_color_hint_from_env() -> bool {
    should_advertise_cli_color_hint(
        std::env::var_os("NO_COLOR").as_deref(),
        std::env::var_os("CLICOLOR").as_deref(),
    )
}

fn should_advertise_cli_color_hint(no_color: Option<&OsStr>, clicolor: Option<&OsStr>) -> bool {
    let no_color_set = no_color.is_some_and(|value| !value.to_string_lossy().is_empty());
    !no_color_set && clicolor.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_advertises_rich_terminal_capabilities_to_child_processes() {
        let command = build_command(&ShellLaunchSpec::new("sh").with_args(["-lc", "true"]));

        assert_eq!(command.get_env("TERM"), Some(OsStr::new("xterm-256color")));
        assert_eq!(command.get_env("COLORTERM"), Some(OsStr::new("truecolor")));
        assert_eq!(command.get_env("TERM_PROGRAM"), Some(OsStr::new("terminal-platform")));
        assert_eq!(
            command.get_env("TERM_PROGRAM_VERSION"),
            Some(OsStr::new(env!("CARGO_PKG_VERSION")))
        );
        assert_eq!(command.get_env("TERMINAL_PLATFORM"), Some(OsStr::new("1")));
        assert_eq!(
            command.get_env("TERM_FEATURES"),
            Some(OsStr::new(crate::TERMINAL_FEATURE_REPORT))
        );
    }

    #[test]
    fn build_command_does_not_force_cli_color_policy() {
        let command = build_command(&ShellLaunchSpec::new("sh"));
        let overridden_keys =
            command.iter_extra_env_as_str().map(|(key, _)| key).collect::<Vec<_>>();

        assert!(!overridden_keys.contains(&"FORCE_COLOR"));
        assert!(!overridden_keys.contains(&"CLICOLOR_FORCE"));
        assert!(!overridden_keys.contains(&"NO_COLOR"));
    }

    #[test]
    fn cli_color_hint_is_non_forcing_and_respects_explicit_user_policy() {
        assert!(should_advertise_cli_color_hint(None, None));
        assert!(should_advertise_cli_color_hint(Some(OsStr::new("")), None));
        assert!(!should_advertise_cli_color_hint(Some(OsStr::new("1")), None));
        assert!(!should_advertise_cli_color_hint(None, Some(OsStr::new("0"))));
    }
}
