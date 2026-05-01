use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
pub(super) fn fullscreen_tools_available() -> bool {
    let missing = ["vim", "less", "fzf"]
        .into_iter()
        .filter(|command| !command_on_path(command))
        .collect::<Vec<_>>();

    if missing.is_empty() {
        return true;
    }

    if std::env::var_os("CI").is_some() {
        panic!("fullscreen viewport smoke requires tools on PATH: {}", missing.join(", "));
    }

    eprintln!(
        "skipping fullscreen viewport smoke locally because tools are missing: {}",
        missing.join(", ")
    );
    false
}

#[cfg(any(unix, windows))]
pub(super) fn command_on_path(command: &str) -> bool {
    let has_separator = command.contains(std::path::MAIN_SEPARATOR) || command.contains('/');
    if has_separator {
        return std::path::Path::new(command).is_file();
    }

    let candidates = if cfg!(windows) {
        if command.contains('.') {
            vec![command.to_string()]
        } else {
            vec![
                format!("{command}.exe"),
                format!("{command}.cmd"),
                format!("{command}.bat"),
                command.to_string(),
            ]
        }
    } else {
        vec![command.to_string()]
    };

    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .any(|dir| candidates.iter().any(|candidate| dir.join(candidate).is_file()))
        })
        .unwrap_or(false)
}

#[cfg(any(unix, windows))]
pub(super) fn resolve_command_on_path(command: &str) -> Option<String> {
    let has_separator = command.contains(std::path::MAIN_SEPARATOR) || command.contains('/');
    if has_separator {
        return std::path::Path::new(command).is_file().then(|| command.to_string());
    }

    let candidates = if cfg!(windows) {
        if command.contains('.') {
            vec![command.to_string()]
        } else {
            vec![
                format!("{command}.exe"),
                format!("{command}.cmd"),
                format!("{command}.bat"),
                command.to_string(),
            ]
        }
    } else {
        vec![command.to_string()]
    };

    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            candidates.iter().find_map(|candidate| {
                let path = dir.join(candidate);
                path.is_file().then(|| path.display().to_string())
            })
        })
    })
}

#[cfg(any(unix, windows))]
pub(super) fn quoted_command_path(path: &std::path::Path) -> String {
    format!("\"{}\"", path.display())
}

#[cfg(any(unix, windows))]
pub(super) fn temp_fullscreen_paths(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let temp_name = format!(
        "terminal-platform-fullscreen-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );
    let viewport_file = std::env::temp_dir().join(format!("{temp_name}.txt"));
    let fzf_file = std::env::temp_dir().join(format!("{temp_name}-fzf.txt"));
    (viewport_file, fzf_file)
}

#[cfg(any(unix, windows))]
pub(super) async fn send_pane_input(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    data: String,
) {
    fixture
        .client
        .dispatch(
            session_id,
            MuxCommand::SendInput(SendInputSpec { pane_id, data, client_event_id: None }),
        )
        .await
        .expect("pane input should succeed");
}

#[cfg(any(unix, windows))]
pub(super) async fn wait_for_shell_marker(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    label: &str,
) {
    let marker = format!(
        "terminal-platform-shell-{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );

    let max_attempts = if cfg!(windows) { 240 } else { 120 };
    for attempt in 0..max_attempts {
        if attempt % 6 == 0 {
            send_pane_input(
                fixture,
                session_id,
                pane_id,
                submitted_input(&format!("echo {marker}")),
            )
            .await;
        }

        let screen = fixture
            .client
            .screen_snapshot(session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");
        if screen.surface.lines.iter().any(|line| line.text.contains(&marker)) {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }

    panic!("shell never echoed marker for {label}");
}

#[cfg(any(unix, windows))]
pub(super) fn fullscreen_fzf_command(path: &std::path::Path) -> String {
    let quoted = quoted_command_path(path);
    if cfg!(windows) {
        let fzf = resolve_command_on_path("fzf").unwrap_or_else(|| "fzf".to_string());
        // `cmd.exe` handles a bare executable path more reliably inside a pipe when the resolved
        // path has no spaces, while still keeping the quoted fallback for local paths that do.
        let fzf = if fzf.contains(' ') { format!("\"{fzf}\"") } else { fzf };
        format!("type {quoted} | {fzf}")
    } else {
        format!("fzf < {quoted}")
    }
}

#[cfg(any(unix, windows))]
pub(super) fn fullscreen_less_command(path: &std::path::Path, prefix: &str) -> String {
    let quoted = quoted_command_path(path);
    if cfg!(unix) && prefix == "zellij" {
        // Zellij `dump-screen` is not a truthful proof source for `less` in alternate screen mode
        // on hosted Linux, so keep the pager in the main buffer for imported-backend acceptance.
        format!("less -X +/{prefix}-less-gamma {quoted}")
    } else {
        format!("less +/{prefix}-less-gamma {quoted}")
    }
}

#[cfg(any(unix, windows))]
pub(super) async fn run_fullscreen_viewport_flow(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    prefix: &str,
    exercise_less: bool,
    exercise_fzf: bool,
) {
    let (viewport_file, fzf_file) = temp_fullscreen_paths(prefix);
    fs::write(
        &viewport_file,
        format!(
            "{prefix}-vim-alpha\n\
{prefix}-vim-beta\n\
{prefix}-less-gamma\n\
{prefix}-less-delta\n"
        ),
    )
    .expect("viewport fixture file should write");
    fs::write(&fzf_file, format!("{prefix}-fzf-alpha\n{prefix}-fzf-beta\n{prefix}-fzf-gamma\n"))
        .expect("fzf fixture file should write");

    send_pane_input(
        fixture,
        session_id,
        pane_id,
        submitted_input(&format!("vim {}", quoted_command_path(&viewport_file))),
    )
    .await;
    wait_for_screen_line(fixture, session_id, pane_id, &format!("{prefix}-vim-alpha")).await;

    send_pane_input(fixture, session_id, pane_id, submitted_input(":q!")).await;
    wait_for_shell_marker(fixture, session_id, pane_id, &format!("{prefix}-vim-exit")).await;

    if exercise_less {
        send_pane_input(
            fixture,
            session_id,
            pane_id,
            submitted_input(&fullscreen_less_command(&viewport_file, prefix)),
        )
        .await;
        wait_for_screen_line(fixture, session_id, pane_id, &format!("{prefix}-less-gamma")).await;

        send_pane_input(fixture, session_id, pane_id, "q".to_string()).await;
        wait_for_shell_marker(fixture, session_id, pane_id, &format!("{prefix}-less-exit")).await;
    }

    if exercise_fzf {
        send_pane_input(
            fixture,
            session_id,
            pane_id,
            submitted_input(&fullscreen_fzf_command(&fzf_file)),
        )
        .await;
        wait_for_screen_line(fixture, session_id, pane_id, &format!("{prefix}-fzf-beta")).await;

        send_pane_input(fixture, session_id, pane_id, submitted_input("beta")).await;
        wait_for_screen_line(fixture, session_id, pane_id, &format!("{prefix}-fzf-beta")).await;
        wait_for_shell_marker(fixture, session_id, pane_id, &format!("{prefix}-fzf-exit")).await;
    }

    let _ = fs::remove_file(viewport_file);
    let _ = fs::remove_file(fzf_file);
}
