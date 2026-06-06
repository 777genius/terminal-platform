use terminal_backend_api::BackendError;
use terminal_domain::PaneId;

pub(super) fn new_pane_id_after_split(
    before_panes: &[PaneId],
    after_panes: &[PaneId],
) -> Result<PaneId, BackendError> {
    after_panes.iter().copied().find(|pane_id| !before_panes.contains(pane_id)).ok_or_else(|| {
        BackendError::internal("restored native split did not produce a new pane id")
    })
}
