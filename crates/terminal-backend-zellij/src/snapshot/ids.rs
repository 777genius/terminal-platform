use terminal_domain::{PaneId, TabId};
use uuid::Uuid;

use crate::target::ZellijTarget;

pub(super) fn deterministic_tab_id(
    target: &ZellijTarget,
    backend_tab_id: u32,
    position: u32,
) -> TabId {
    deterministic_uuid(
        &format!(
            "terminal-platform/zellij/tab/{}/{}/{}",
            target.session_name, backend_tab_id, position
        ),
        TabId::from,
    )
}

pub(super) fn deterministic_pane_id(
    target: &ZellijTarget,
    backend_tab_id: u32,
    backend_ref: &str,
) -> PaneId {
    deterministic_uuid(
        &format!(
            "terminal-platform/zellij/pane/{}/{}/{}",
            target.session_name, backend_tab_id, backend_ref
        ),
        PaneId::from,
    )
}

fn deterministic_uuid<T>(fingerprint: &str, construct: fn(Uuid) -> T) -> T {
    construct(Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes()))
}
