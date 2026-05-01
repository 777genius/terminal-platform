use std::fs;

use crate::{
    constants::*,
    support::{assert_contains_all, assert_value, workspace_root},
};

use super::{
    verify_manual_qa_scope, verify_node_package_scripts, verify_recorded_passes,
    verify_v1_release_configs, verify_v1_workflows, verify_windows_conpty_vendor_patch,
    verify_windows_zellij_package_smoke,
};

pub(crate) fn verify_v1_readiness(require_recorded_passes: bool) -> Result<(), String> {
    let workspace_root = workspace_root();
    let license = workspace_root.join(LICENSE_PATH);
    let contributing = workspace_root.join(CONTRIBUTING_PATH);
    let security = workspace_root.join(SECURITY_PATH);
    let code_of_conduct = workspace_root.join(CODE_OF_CONDUCT_PATH);
    let pull_request_template = workspace_root.join(PULL_REQUEST_TEMPLATE_PATH);
    let workspace_manifest = workspace_root.join(WORKSPACE_MANIFEST_PATH);
    let root_readme = workspace_root.join(ROOT_README_PATH);
    let node_package_readme = workspace_root.join(NODE_PACKAGE_README_PATH);
    let node_package_stage_script = workspace_root.join(NODE_PACKAGE_STAGE_SCRIPT_PATH);
    let node_package_build_script = workspace_root.join(NODE_PACKAGE_BUILD_SCRIPT_PATH);
    let node_package_pack_script = workspace_root.join(NODE_PACKAGE_PACK_SCRIPT_PATH);
    let node_package_verify_script = workspace_root.join(NODE_PACKAGE_VERIFY_SCRIPT_PATH);
    let node_smoke_test = workspace_root.join(NODE_SMOKE_TEST_PATH);
    let node_package_smoke_test = workspace_root.join(NODE_PACKAGE_SMOKE_TEST_PATH);
    let node_package_install_smoke_test = workspace_root.join(NODE_PACKAGE_INSTALL_SMOKE_TEST_PATH);
    let zellij_installer = workspace_root.join(ZELLIJ_INSTALLER_PATH);
    let manual_dir = workspace_root.join(MANUAL_DIR);
    let manual_drafts_dir = workspace_root.join(MANUAL_DRAFTS_DIR);
    let manual_runs_dir = workspace_root.join(MANUAL_RUNS_DIR);
    let manual_readme = manual_dir.join("README.md");
    let electron_checklist = manual_dir.join("electron.md");
    let native_checklist = manual_dir.join("native.md");
    let tmux_checklist = manual_dir.join("tmux.md");
    let zellij_checklist = manual_dir.join("zellij.md");
    let windows_native_zellij_checklist = manual_dir.join("windows-native-zellij.md");
    let ci_workflow = workspace_root.join(CI_WORKFLOW_PATH);
    let release_readiness_workflow = workspace_root.join(RELEASE_READINESS_WORKFLOW_PATH);
    let release_plz_workflow = workspace_root.join(RELEASE_PLZ_WORKFLOW_PATH);
    let release_candidate_checklist = workspace_root.join(RELEASE_CANDIDATE_CHECKLIST_PATH);
    let release_candidate_summary = workspace_root.join(RELEASE_CANDIDATE_SUMMARY_PATH);
    let release_summary_template = workspace_root.join(RELEASE_SUMMARY_TEMPLATE_PATH);
    let release_plz_config = workspace_root.join(RELEASE_PLZ_CONFIG_PATH);
    let deny_config = workspace_root.join(DENY_CONFIG_PATH);
    let fuzz_dir = workspace_root.join(FUZZ_DIR);
    let vendored_portable_pty_psuedocon = workspace_root.join(VENDORED_PORTABLE_PTY_PSEUDOCON_PATH);

    assert_value(license.is_file(), "root LICENSE is missing")?;
    assert_value(contributing.is_file(), "root CONTRIBUTING.md is missing")?;
    assert_value(security.is_file(), "root SECURITY.md is missing")?;
    assert_value(code_of_conduct.is_file(), "root CODE_OF_CONDUCT.md is missing")?;
    assert_value(pull_request_template.is_file(), "pull request template is missing")?;
    assert_value(workspace_manifest.is_file(), "workspace Cargo.toml is missing")?;
    assert_value(root_readme.is_file(), "root README is missing")?;
    assert_value(node_package_readme.is_file(), "Node package README is missing")?;
    assert_value(node_package_stage_script.is_file(), "Node package stage script is missing")?;
    assert_value(node_package_build_script.is_file(), "Node package build script is missing")?;
    assert_value(node_package_pack_script.is_file(), "Node package pack script is missing")?;
    assert_value(node_package_verify_script.is_file(), "Node package verify script is missing")?;
    assert_value(node_smoke_test.is_file(), "Node addon smoke test is missing")?;
    assert_value(node_package_smoke_test.is_file(), "Node package smoke test is missing")?;
    assert_value(
        node_package_install_smoke_test.is_file(),
        "Node installed package smoke test is missing",
    )?;
    assert_value(zellij_installer.is_file(), "Zellij installer script is missing")?;
    assert_value(manual_dir.is_dir(), "manual QA directory is missing")?;
    assert_value(manual_readme.is_file(), "manual QA README is missing")?;
    assert_value(electron_checklist.is_file(), "Electron manual checklist is missing")?;
    assert_value(native_checklist.is_file(), "native manual checklist is missing")?;
    assert_value(tmux_checklist.is_file(), "tmux manual checklist is missing")?;
    assert_value(zellij_checklist.is_file(), "Zellij manual checklist is missing")?;
    assert_value(
        windows_native_zellij_checklist.is_file(),
        "Windows Native + Zellij manual checklist is missing",
    )?;
    assert_value(manual_drafts_dir.is_dir(), "manual draft capture directory is missing")?;
    assert_value(manual_runs_dir.is_dir(), "manual run capture directory is missing")?;
    assert_value(ci_workflow.is_file(), "ci workflow is missing")?;
    assert_value(release_readiness_workflow.is_file(), "release readiness workflow is missing")?;
    assert_value(release_plz_workflow.is_file(), "release-plz workflow is missing")?;
    assert_value(release_candidate_checklist.is_file(), "release candidate checklist is missing")?;
    assert_value(release_candidate_summary.is_file(), "release candidate summary is missing")?;
    assert_value(release_summary_template.is_file(), "release summary template is missing")?;
    assert_value(release_plz_config.is_file(), "release-plz config is missing")?;
    assert_value(deny_config.is_file(), "cargo-deny config is missing")?;
    assert_value(fuzz_dir.is_dir(), "fuzz directory is missing")?;
    assert_value(
        vendored_portable_pty_psuedocon.is_file(),
        "vendored portable-pty Windows ConPTY source is missing",
    )?;

    let workspace_manifest_contents = fs::read_to_string(&workspace_manifest)
        .map_err(|error| format!("failed to read {} - {error}", workspace_manifest.display()))?;
    let root_readme_contents = fs::read_to_string(&root_readme)
        .map_err(|error| format!("failed to read {} - {error}", root_readme.display()))?;
    let contributing_contents = fs::read_to_string(&contributing)
        .map_err(|error| format!("failed to read {} - {error}", contributing.display()))?;
    let node_package_readme_contents = fs::read_to_string(&node_package_readme)
        .map_err(|error| format!("failed to read {} - {error}", node_package_readme.display()))?;
    let node_package_stage_script_contents = fs::read_to_string(&node_package_stage_script)
        .map_err(|error| {
            format!("failed to read {} - {error}", node_package_stage_script.display())
        })?;
    let node_package_build_script_contents = fs::read_to_string(&node_package_build_script)
        .map_err(|error| {
            format!("failed to read {} - {error}", node_package_build_script.display())
        })?;
    let node_package_pack_script_contents =
        fs::read_to_string(&node_package_pack_script).map_err(|error| {
            format!("failed to read {} - {error}", node_package_pack_script.display())
        })?;
    let node_package_verify_script_contents = fs::read_to_string(&node_package_verify_script)
        .map_err(|error| {
            format!("failed to read {} - {error}", node_package_verify_script.display())
        })?;
    let node_smoke_test_contents = fs::read_to_string(&node_smoke_test)
        .map_err(|error| format!("failed to read {} - {error}", node_smoke_test.display()))?;
    let node_package_smoke_test_contents =
        fs::read_to_string(&node_package_smoke_test).map_err(|error| {
            format!("failed to read {} - {error}", node_package_smoke_test.display())
        })?;
    let node_package_install_smoke_test_contents =
        fs::read_to_string(&node_package_install_smoke_test).map_err(|error| {
            format!("failed to read {} - {error}", node_package_install_smoke_test.display())
        })?;
    let zellij_installer_contents = fs::read_to_string(&zellij_installer)
        .map_err(|error| format!("failed to read {} - {error}", zellij_installer.display()))?;
    let manual_readme_contents = fs::read_to_string(&manual_readme)
        .map_err(|error| format!("failed to read {} - {error}", manual_readme.display()))?;
    let electron_checklist_contents = fs::read_to_string(&electron_checklist)
        .map_err(|error| format!("failed to read {} - {error}", electron_checklist.display()))?;
    let native_checklist_contents = fs::read_to_string(&native_checklist)
        .map_err(|error| format!("failed to read {} - {error}", native_checklist.display()))?;
    let tmux_checklist_contents = fs::read_to_string(&tmux_checklist)
        .map_err(|error| format!("failed to read {} - {error}", tmux_checklist.display()))?;
    let zellij_checklist_contents = fs::read_to_string(&zellij_checklist)
        .map_err(|error| format!("failed to read {} - {error}", zellij_checklist.display()))?;
    let windows_native_zellij_checklist_contents =
        fs::read_to_string(&windows_native_zellij_checklist).map_err(|error| {
            format!("failed to read {} - {error}", windows_native_zellij_checklist.display())
        })?;
    let release_candidate_summary_contents = fs::read_to_string(&release_candidate_summary)
        .map_err(|error| {
            format!("failed to read {} - {error}", release_candidate_summary.display())
        })?;
    let release_summary_template_contents =
        fs::read_to_string(&release_summary_template).map_err(|error| {
            format!("failed to read {} - {error}", release_summary_template.display())
        })?;
    let ci_workflow_contents = fs::read_to_string(&ci_workflow)
        .map_err(|error| format!("failed to read {} - {error}", ci_workflow.display()))?;
    let release_readiness_workflow_contents = fs::read_to_string(&release_readiness_workflow)
        .map_err(|error| {
            format!("failed to read {} - {error}", release_readiness_workflow.display())
        })?;
    let release_plz_workflow_contents = fs::read_to_string(&release_plz_workflow)
        .map_err(|error| format!("failed to read {} - {error}", release_plz_workflow.display()))?;
    let release_candidate_checklist_contents = fs::read_to_string(&release_candidate_checklist)
        .map_err(|error| {
            format!("failed to read {} - {error}", release_candidate_checklist.display())
        })?;
    let release_plz_config_contents = fs::read_to_string(&release_plz_config)
        .map_err(|error| format!("failed to read {} - {error}", release_plz_config.display()))?;
    let deny_config_contents = fs::read_to_string(&deny_config)
        .map_err(|error| format!("failed to read {} - {error}", deny_config.display()))?;
    let pull_request_template_contents = fs::read_to_string(&pull_request_template)
        .map_err(|error| format!("failed to read {} - {error}", pull_request_template.display()))?;
    let vendored_portable_pty_psuedocon_contents =
        fs::read_to_string(&vendored_portable_pty_psuedocon).map_err(|error| {
            format!("failed to read {} - {error}", vendored_portable_pty_psuedocon.display())
        })?;

    for expected_line in [
        "- `macOS + Linux` - `Native + tmux + Zellij`",
        "- `Windows` - `Native + Zellij`",
        "- `tmux` stays Unix-only in v1 docs, tests, CI, and acceptance",
    ] {
        assert_value(
            root_readme_contents.contains(expected_line),
            &format!("root README is missing support matrix line: {expected_line}"),
        )?;
    }

    for expected_line in [
        "- `macOS + Linux` - `Native + tmux + Zellij`",
        "- `Windows` - `Native + Zellij`",
        "- `tmux` stays Unix-only in v1 docs, tests, CI, and acceptance",
    ] {
        assert_value(
            node_package_readme_contents.contains(expected_line),
            &format!("Node package README is missing support matrix line: {expected_line}"),
        )?;
    }
    assert_contains_all(
        &node_package_readme_contents,
        "Node package README install proof",
        &[
            "pack-local-package.mjs",
            "npm_config_cache",
            "test -f \"$TARBALL\"",
            "npm install --ignore-scripts --no-audit --no-fund --no-package-lock",
            "node --input-type=module",
        ],
    )?;
    verify_node_package_scripts(
        &node_package_stage_script_contents,
        &node_package_build_script_contents,
        &node_package_pack_script_contents,
        &node_package_verify_script_contents,
    )?;

    for expected_line in [
        "- `macOS + Linux` - `Native + tmux + Zellij`",
        "- `Windows` - `Native + Zellij`",
        "- `tmux` remains Unix-only in docs, CI, and acceptance",
    ] {
        assert_value(
            release_candidate_summary_contents.contains(expected_line),
            &format!("release candidate summary is missing support matrix line: {expected_line}"),
        )?;
    }
    for expected_line in [
        "- `macOS + Linux` - `Native + tmux + Zellij`",
        "- `Windows` - `Native + Zellij`",
        "- `tmux` remains Unix-only in docs, CI, and acceptance",
        "- recorded manual pass artifacts captured for Electron embed, Unix `tmux`, and Windows `Native + Zellij`",
    ] {
        assert_value(
            release_summary_template_contents.contains(expected_line),
            &format!("release summary template is missing v1 line: {expected_line}"),
        )?;
    }

    assert_value(
        !release_candidate_summary_contents.contains("TODO"),
        "release candidate summary still contains TODO placeholders",
    )?;
    assert_value(
        !release_candidate_summary_contents.contains("TBD"),
        "release candidate summary still contains TBD placeholders",
    )?;
    assert_value(
        !release_summary_template_contents.contains("TODO"),
        "release summary template still contains TODO placeholders",
    )?;
    assert_value(
        !release_summary_template_contents.contains("TBD"),
        "release summary template still contains TBD placeholders",
    )?;

    verify_v1_workflows(
        &ci_workflow_contents,
        &release_readiness_workflow_contents,
        &release_plz_workflow_contents,
    )?;
    verify_v1_release_configs(&release_plz_config_contents, &deny_config_contents)?;
    verify_windows_conpty_vendor_patch(
        &workspace_manifest_contents,
        &vendored_portable_pty_psuedocon_contents,
    )?;
    assert_contains_all(
        &contributing_contents,
        "contributing v1 package proof",
        &[
            "build-local-package.mjs",
            "verify-package.mjs",
            "pack-local-package.mjs",
            "npm_config_cache",
            "test -f \"$TARBALL\"",
            "stage-capi-package",
            "verify-capi-package",
            "install-capi-package",
            "verify-capi-install",
            "verify-v1-readiness --require-recorded-passes",
            "git format-patch origin/main..HEAD --stdout",
            "git bundle create terminal-platform-v1-closeout.bundle origin/main..HEAD",
            "git bundle verify terminal-platform-v1-closeout.bundle",
        ],
    )?;
    assert_contains_all(
        &pull_request_template_contents,
        "pull request template v1 gates",
        &[
            "Node staged package smoke",
            "Node installed package smoke",
            "C ABI package stage/install smoke",
            "verify-v1-readiness --require-recorded-passes",
            "Windows Native + Zellij recorded pass",
        ],
    )?;
    assert_contains_all(
        &release_candidate_checklist_contents,
        "release candidate checklist package proof",
        &[
            "build-local-package.mjs",
            "verify-package.mjs",
            "pack-local-package.mjs",
            "npm_config_cache",
            "test -f \"$TARBALL\"",
            "npm install --ignore-scripts --no-audit --no-fund --no-package-lock",
            "stage-capi-package",
            "verify-capi-package",
            "install-capi-package",
            "verify-capi-install",
            "Offline handoff when push is unavailable",
            "git format-patch origin/main..HEAD --stdout",
            "git bundle create terminal-platform-v1-closeout.bundle origin/main..HEAD",
            "git bundle verify terminal-platform-v1-closeout.bundle",
            "git am terminal-platform-v1-closeout-local.patch",
        ],
    )?;
    verify_windows_zellij_package_smoke(
        &node_smoke_test_contents,
        &node_package_smoke_test_contents,
        &node_package_install_smoke_test_contents,
    )?;
    assert_contains_all(
        &zellij_installer_contents,
        "Zellij installer",
        &["assert_supported_zellij_release", "below the v1 minimum 0.44.0"],
    )?;
    assert_contains_all(
        &zellij_installer_contents,
        "Zellij installer retry policy",
        &["REQUEST_TIMEOUT_SECONDS", "REQUEST_ATTEMPTS", "open_url_with_retries"],
    )?;
    assert_value(
        manual_readme_contents.contains("one Windows `Native + Zellij` pass"),
        "manual QA README must require a Windows Native + Zellij pass",
    )?;
    assert_value(
        windows_native_zellij_checklist_contents
            .contains("live `Zellij` import/control path through the package surface"),
        "Windows Native + Zellij checklist must cover Zellij through package smoke",
    )?;
    verify_manual_qa_scope(
        &electron_checklist_contents,
        &native_checklist_contents,
        &tmux_checklist_contents,
        &zellij_checklist_contents,
        &windows_native_zellij_checklist_contents,
    )?;

    for relative_path in [
        "README.md",
        "electron.md",
        "native.md",
        "tmux.md",
        "windows-native-zellij.md",
        "zellij.md",
    ] {
        let path = manual_dir.join(relative_path);
        assert_value(path.is_file(), &format!("manual checklist is missing: {}", path.display()))?;
    }

    let drafts_readme = manual_drafts_dir.join("README.md");
    assert_value(
        drafts_readme.is_file(),
        &format!("manual draft helper is missing: {}", drafts_readme.display()),
    )?;

    for relative_path in ["README.md", "_template.md"] {
        let path = manual_runs_dir.join(relative_path);
        assert_value(
            path.is_file(),
            &format!("manual run artifact helper is missing: {}", path.display()),
        )?;
    }

    if require_recorded_passes {
        verify_recorded_passes(&manual_runs_dir)?;
    }

    Ok(())
}
