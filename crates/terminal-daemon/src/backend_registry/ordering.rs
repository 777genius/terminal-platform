use terminal_domain::BackendKind;

pub(super) fn sort_backends(mut backends: Vec<BackendKind>) -> Vec<BackendKind> {
    backends.sort_by_key(|kind| match kind {
        BackendKind::Native => 0,
        BackendKind::Tmux => 1,
        BackendKind::Zellij => 2,
    });
    backends
}
