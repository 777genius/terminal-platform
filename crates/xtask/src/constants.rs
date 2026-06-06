pub(crate) const CAPI_PACKAGE_NAME: &str = "terminal-capi";
pub(crate) const CAPI_HEADER_NAME: &str = "terminal-platform-capi.h";
pub(crate) const CAPI_LIBRARY_BASENAME: &str = "terminal_capi";
pub(crate) const CAPI_PKGCONFIG_NAME: &str = "terminal-platform-capi";
pub(crate) const CAPI_INSTALL_SHARE_DIR: &str = "share/terminal-capi";
pub(crate) const CAPI_SCHEMA_VERSION: u64 = 1;
pub(crate) const LICENSE_PATH: &str = "LICENSE";
pub(crate) const CONTRIBUTING_PATH: &str = "CONTRIBUTING.md";
pub(crate) const SECURITY_PATH: &str = "SECURITY.md";
pub(crate) const CODE_OF_CONDUCT_PATH: &str = "CODE_OF_CONDUCT.md";
pub(crate) const PULL_REQUEST_TEMPLATE_PATH: &str = ".github/pull_request_template.md";
pub(crate) const WORKSPACE_MANIFEST_PATH: &str = "Cargo.toml";
pub(crate) const ROOT_README_PATH: &str = "README.md";
pub(crate) const NODE_PACKAGE_README_PATH: &str = "crates/terminal-node-napi/package/README.md";
pub(crate) const NODE_PACKAGE_STAGE_SCRIPT_PATH: &str =
    "crates/terminal-node-napi/package/scripts/stage-package.mjs";
pub(crate) const NODE_PACKAGE_BUILD_SCRIPT_PATH: &str =
    "crates/terminal-node-napi/package/scripts/build-local-package.mjs";
pub(crate) const NODE_PACKAGE_PACK_SCRIPT_PATH: &str =
    "crates/terminal-node-napi/package/scripts/pack-local-package.mjs";
pub(crate) const NODE_PACKAGE_VERIFY_SCRIPT_PATH: &str =
    "crates/terminal-node-napi/package/scripts/verify-package.mjs";
pub(crate) const NODE_SMOKE_TEST_PATH: &str = "crates/terminal-node-napi/tests/node_smoke.rs";
pub(crate) const NODE_PACKAGE_SMOKE_TEST_PATH: &str =
    "crates/terminal-node-napi/tests/package_smoke.rs";
pub(crate) const NODE_PACKAGE_INSTALL_SMOKE_TEST_PATH: &str =
    "crates/terminal-node-napi/tests/package_install_smoke.rs";
pub(crate) const ZELLIJ_INSTALLER_PATH: &str = ".github/scripts/install_zellij.py";
pub(crate) const MANUAL_DIR: &str = "crates/terminal-testing/manual";
pub(crate) const MANUAL_DRAFTS_DIR: &str = "crates/terminal-testing/manual/drafts";
pub(crate) const MANUAL_RUNS_DIR: &str = "crates/terminal-testing/manual/runs";
pub(crate) const CI_WORKFLOW_PATH: &str = ".github/workflows/ci.yml";
pub(crate) const RELEASE_READINESS_WORKFLOW_PATH: &str = ".github/workflows/release-readiness.yml";
pub(crate) const RELEASE_PLZ_WORKFLOW_PATH: &str = ".github/workflows/release-plz.yml";
pub(crate) const RELEASE_CANDIDATE_CHECKLIST_PATH: &str =
    "docs/terminal/v1-release-candidate-checklist.md";
pub(crate) const RELEASE_CANDIDATE_SUMMARY_PATH: &str =
    "docs/terminal/v1-release-candidate-summary.md";
pub(crate) const RELEASE_SUMMARY_TEMPLATE_PATH: &str =
    "docs/terminal/v1-release-summary-template.md";
pub(crate) const RELEASE_PLZ_CONFIG_PATH: &str = "release-plz.toml";
pub(crate) const DENY_CONFIG_PATH: &str = "deny.toml";
pub(crate) const FUZZ_DIR: &str = "fuzz";
pub(crate) const VENDORED_PORTABLE_PTY_PSEUDOCON_PATH: &str =
    "vendor/portable-pty/src/win/psuedocon.rs";
pub(crate) const MANUAL_RUN_TEMPLATE_DATE_PLACEHOLDER: &str = "Date: YYYY-MM-DD";
pub(crate) const MANUAL_RUN_TEMPLATE_OS_PLACEHOLDER: &str =
    "OS: macOS 15.4 / Ubuntu 24.04 / Windows 11 24H2";
pub(crate) const MANUAL_RUN_TEMPLATE_CHECKLIST_PLACEHOLDER: &str =
    "Checklist: crates/terminal-testing/manual/<checklist>.md";
pub(crate) const MANUAL_RUN_TEMPLATE_RUST_PLACEHOLDER: &str = "Rust: rustc 1.xx.x";
pub(crate) const MANUAL_RUN_TEMPLATE_NODE_PLACEHOLDER: &str = "Node: vxx.x.x";
