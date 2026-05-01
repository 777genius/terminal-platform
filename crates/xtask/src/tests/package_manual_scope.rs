use crate::{
    verify_manual_qa_scope, verify_node_package_scripts, verify_windows_zellij_package_smoke,
};

#[test]
fn verify_node_package_scripts_accepts_expected_guardrails() {
    if let Err(error) = verify_node_package_scripts(
        VALID_NODE_PACKAGE_STAGE_SCRIPT,
        VALID_NODE_PACKAGE_BUILD_SCRIPT,
        VALID_NODE_PACKAGE_PACK_SCRIPT,
        VALID_NODE_PACKAGE_VERIFY_SCRIPT,
    ) {
        panic!("expected Node package script guardrails to validate - {error}");
    }
}

#[test]
fn verify_node_package_scripts_rejects_missing_stage_output_guard() {
    let invalid_stage_script = VALID_NODE_PACKAGE_STAGE_SCRIPT
        .replace("assertSafeOutputDir(outDir)", "/* unsafe output accepted */");
    let error = match verify_node_package_scripts(
        &invalid_stage_script,
        VALID_NODE_PACKAGE_BUILD_SCRIPT,
        VALID_NODE_PACKAGE_PACK_SCRIPT,
        VALID_NODE_PACKAGE_VERIFY_SCRIPT,
    ) {
        Ok(()) => panic!("expected missing stage output guard to fail"),
        Err(error) => error,
    };

    assert!(error.contains("assertSafeOutputDir(outDir)"), "got: {error}");
}

#[test]
fn verify_manual_qa_scope_accepts_expected_markers() {
    if let Err(error) = verify_manual_qa_scope(
        VALID_ELECTRON_CHECKLIST,
        VALID_NATIVE_CHECKLIST,
        VALID_TMUX_CHECKLIST,
        VALID_ZELLIJ_CHECKLIST,
        VALID_WINDOWS_NATIVE_ZELLIJ_CHECKLIST,
    ) {
        panic!("expected manual QA scope to validate - {error}");
    }
}

#[test]
fn verify_manual_qa_scope_rejects_missing_windows_resize_churn() {
    let invalid_windows_checklist =
        VALID_WINDOWS_NATIVE_ZELLIJ_CHECKLIST.replace("resize churn", "size changes");
    let error = match verify_manual_qa_scope(
        VALID_ELECTRON_CHECKLIST,
        VALID_NATIVE_CHECKLIST,
        VALID_TMUX_CHECKLIST,
        VALID_ZELLIJ_CHECKLIST,
        &invalid_windows_checklist,
    ) {
        Ok(()) => panic!("expected missing Windows resize churn marker to fail"),
        Err(error) => error,
    };

    assert!(error.contains("resize churn"), "got: {error}");
}

#[test]
fn verify_windows_zellij_package_smoke_accepts_expected_markers() {
    if let Err(error) = verify_windows_zellij_package_smoke(
        VALID_WINDOWS_ZELLIJ_SMOKE_TEST,
        VALID_WINDOWS_ZELLIJ_SMOKE_TEST,
        VALID_WINDOWS_ZELLIJ_SMOKE_TEST,
    ) {
        panic!("expected Windows Zellij smoke markers to validate - {error}");
    }
}

#[test]
fn verify_windows_zellij_package_smoke_rejects_missing_package_marker() {
    let invalid_package_smoke = VALID_WINDOWS_ZELLIJ_SMOKE_TEST
        .replace("TERMINAL_NODE_EXTERNAL_ZELLIJ_SESSION", "TERMINAL_NODE_ZELLIJ_DISABLED");
    let error = match verify_windows_zellij_package_smoke(
        VALID_WINDOWS_ZELLIJ_SMOKE_TEST,
        &invalid_package_smoke,
        VALID_WINDOWS_ZELLIJ_SMOKE_TEST,
    ) {
        Ok(()) => panic!("expected missing staged package marker to fail"),
        Err(error) => error,
    };

    assert!(error.contains("staged package smoke"), "got: {error}");
}

use super::shared_data::{
    VALID_ELECTRON_CHECKLIST, VALID_NATIVE_CHECKLIST, VALID_NODE_PACKAGE_BUILD_SCRIPT,
    VALID_NODE_PACKAGE_PACK_SCRIPT, VALID_NODE_PACKAGE_STAGE_SCRIPT,
    VALID_NODE_PACKAGE_VERIFY_SCRIPT, VALID_TMUX_CHECKLIST, VALID_WINDOWS_NATIVE_ZELLIJ_CHECKLIST,
    VALID_WINDOWS_ZELLIJ_SMOKE_TEST, VALID_ZELLIJ_CHECKLIST,
};
