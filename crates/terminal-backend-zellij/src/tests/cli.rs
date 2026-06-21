use terminal_backend_api::BackendError;

use crate::cli::is_transient_zellij_backend_error;

#[test]
fn treats_incomplete_list_panes_rows_as_transient() {
    let error = BackendError::internal("invalid zellij list-panes json: missing field `id`");

    assert!(is_transient_zellij_backend_error(&error));
}
