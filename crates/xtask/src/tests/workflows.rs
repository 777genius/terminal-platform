use crate::verify_v1_workflows;

use super::shared_data::{
    VALID_CI_WORKFLOW, VALID_RELEASE_PLZ_WORKFLOW, VALID_RELEASE_READINESS_WORKFLOW,
};

#[test]
fn verify_v1_workflows_accepts_expected_support_matrix() {
    if let Err(error) = verify_v1_workflows(
        VALID_CI_WORKFLOW,
        VALID_RELEASE_READINESS_WORKFLOW,
        VALID_RELEASE_PLZ_WORKFLOW,
    ) {
        panic!("expected workflow readiness to validate - {error}");
    }
}

#[test]
fn verify_v1_workflows_rejects_tmux_in_windows_job() {
    let invalid_ci = VALID_CI_WORKFLOW.replace("zellij --version\n", "zellij --version\ntmux -V\n");
    let error = match verify_v1_workflows(
        &invalid_ci,
        VALID_RELEASE_READINESS_WORKFLOW,
        VALID_RELEASE_PLZ_WORKFLOW,
    ) {
        Ok(()) => panic!("expected Windows tmux marker to fail"),
        Err(error) => error,
    };

    assert!(error.contains("windows-v1 job must not include tmux"), "got: {error}");
}

#[test]
fn verify_v1_workflows_rejects_missing_fuzz_target() {
    let invalid_ci = VALID_CI_WORKFLOW.replace("zellij_surface", "zellij_missing");
    let error = match verify_v1_workflows(
        &invalid_ci,
        VALID_RELEASE_READINESS_WORKFLOW,
        VALID_RELEASE_PLZ_WORKFLOW,
    ) {
        Ok(()) => panic!("expected missing fuzz target to fail"),
        Err(error) => error,
    };

    assert!(error.contains("zellij_surface"), "got: {error}");
}
