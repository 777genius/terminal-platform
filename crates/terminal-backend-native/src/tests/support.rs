use std::time::Duration;

use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::PaneId;
use terminal_mux_domain::PaneTreeNode;
use tokio::time::sleep;

pub(super) fn cat_launch_spec() -> ShellLaunchSpec {
    #[cfg(unix)]
    {
        ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
    }

    #[cfg(windows)]
    {
        let program = std::env::var("COMSPEC")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "cmd.exe".to_string());

        ShellLaunchSpec::new(program).with_args(["/D", "/Q", "/K", "echo ready"])
    }
}

pub(super) fn echo_input(text: &str) -> String {
    #[cfg(unix)]
    {
        format!("{text}\r")
    }

    #[cfg(windows)]
    {
        format!("echo {text}\r")
    }
}

pub(super) async fn wait_for_screen_line(
    session: &dyn terminal_backend_api::BackendSessionPort,
    pane_id: PaneId,
    needle: &str,
) {
    let mut last_lines = Vec::new();
    for _ in 0..120 {
        let screen =
            session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
        if screen.surface.lines.iter().any(|line| line.text.contains(needle)) {
            return;
        }
        last_lines = screen.surface.lines.iter().map(|line| line.text.clone()).take(12).collect();
        sleep(Duration::from_millis(50)).await;
    }

    panic!("screen never contained expected text: {needle}; last lines: {last_lines:?}");
}

pub(super) fn collect_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
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
