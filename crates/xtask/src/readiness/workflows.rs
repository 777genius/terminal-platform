use crate::support::{assert_contains_all, assert_value, section_between};

pub(crate) fn verify_v1_workflows(
    ci_workflow: &str,
    release_readiness_workflow: &str,
    release_plz_workflow: &str,
) -> Result<(), String> {
    assert_contains_all(
        ci_workflow,
        "ci workflow",
        &[
            "ubuntu-latest",
            "macos-latest",
            "windows-latest",
            "name: unix-${{ matrix.os }}",
            "name: windows-v1",
            "name: governance",
            "name: fuzz-baseline",
            "cargo clippy --workspace --all-targets --all-features",
            "cargo nextest run --profile ci --workspace",
            "cargo run -p xtask -- verify-v1-readiness",
            "cargo-deny",
            "cargo-public-api",
            "cargo-semver-checks",
            "release-plz",
            "cargo-fuzz",
            "protocol_frames",
            "tmux_layout",
            "zellij_surface",
            "screen_delta",
            "zellij --version",
        ],
    )?;

    let unix_job = section_between(ci_workflow, "  unix-matrix:", "\n  windows-v1:")
        .ok_or_else(|| "ci workflow is missing unix-matrix job section".to_string())?;
    assert_contains_all(
        unix_job,
        "ci unix-matrix job",
        &[
            "tmux -V",
            "zellij --version",
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
        ],
    )?;

    let windows_job = section_between(ci_workflow, "  windows-v1:", "\n  governance:")
        .ok_or_else(|| "ci workflow is missing windows-v1 job section".to_string())?;
    assert_contains_all(
        windows_job,
        "ci windows-v1 job",
        &[
            "windows-latest",
            "install_fzf.py",
            "Get-Command $tool",
            "cargo nextest run",
            "--test-threads 1",
            "-p terminal-backend-native",
            "-p terminal-daemon",
            "-p terminal-daemon-client",
            "-p terminal-node",
            "-p terminal-node-napi",
            "-p terminal-protocol",
            "-p terminal-testing",
            "zellij --version",
            "build-local-package.mjs",
            "verify-package.mjs",
            "pack-local-package.mjs",
            "npm_config_cache",
            "Test-Path -Path $tarball -PathType Leaf",
            "npm install --ignore-scripts --no-audit --no-fund --no-package-lock",
        ],
    )?;
    assert_value(
        !windows_job.contains("tmux"),
        "ci windows-v1 job must not include tmux lanes or tooling",
    )?;

    assert_contains_all(
        release_readiness_workflow,
        "release readiness workflow",
        &[
            "workflow_dispatch",
            "timeout-minutes: 45",
            "verify-v1-readiness --require-recorded-passes",
            "cargo-public-api",
            "cargo-semver-checks",
            "release-plz",
            "rustup toolchain install nightly --profile minimal",
            "cargo +nightly public-api -p terminal-domain",
            "cargo +nightly public-api -p terminal-protocol",
            "cargo +nightly public-api -p terminal-node",
            "cargo semver-checks --version",
        ],
    )?;
    assert_contains_all(
        release_plz_workflow,
        "release-plz workflow",
        &[
            "contents: write",
            "pull-requests: write",
            "timeout-minutes: 30",
            "release-plz release-pr --git-token",
        ],
    )?;

    Ok(())
}
