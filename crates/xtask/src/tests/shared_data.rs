pub(super) const VALID_NODE_PACKAGE_STAGE_SCRIPT: &str = r#"
assertSafeOutputDir(outDir);
throw new Error(`Refusing to stage package into unsafe output directory: ${outDir}`);
options.out = readFlagValue(argv, index, arg);
options.addon = readFlagValue(argv, index, arg);
throw new Error(`Missing value for ${flag}`);
"#;

pub(super) const VALID_WORKSPACE_MANIFEST: &str = r#"
[workspace]
members = []

[patch.crates-io]
portable-pty = { path = "vendor/portable-pty" }
"#;

pub(super) const VALID_VENDORED_PORTABLE_PTY_PSEUDOCON: &str = r#"
(CONPTY.CreatePseudoConsole)(
size,
input.as_raw_handle() as _,
output.as_raw_handle() as _,
0,
&mut con,
)
// Terminal Platform v1 intentionally
// flag 0 here
// plain UTF-8/VT I/O without
"#;

pub(super) const VALID_NODE_PACKAGE_BUILD_SCRIPT: &str = r#"
options.out = readFlagValue(argv, index, arg);
throw new Error(`Missing value for ${flag}`);
"#;

pub(super) const VALID_NODE_PACKAGE_PACK_SCRIPT: &str = r#"
options.out = path.resolve(readFlagValue(argv, index, arg));
throw new Error(`Missing value for ${flag}`);
npm_config_cache: process.env.npm_config_cache ?? path.join(options.out, ".npm-cache"),
nodePackageManager()
process.platform === "win32" ? "npm.cmd" : "npm"
shell: packageManagerShell()
process.platform === "win32" ? process.env.ComSpec ?? true : false
npm pack failed to launch -
signal ${packResult.signal ?? "<none>"}
"#;

pub(super) const VALID_NODE_PACKAGE_VERIFY_SCRIPT: &str = r#"
options.packageDir = readFlagValue(argv, index, arg);
throw new Error(`Missing value for ${flag}`);
"#;

pub(super) const VALID_ELECTRON_CHECKLIST: &str = r#"
# Electron Embed Checklist

- Create the main-process bridge and preload API against a live daemon.
- Stress renderer resize churn while the bridge is streaming screen updates.
"#;

pub(super) const VALID_NATIVE_CHECKLIST: &str = r#"
# Native Checklist

- Exercise `vim`, `less`, and `fzf`.
- Stress resize churn and confirm subscriptions stay healthy.
"#;

pub(super) const VALID_TMUX_CHECKLIST: &str = r#"
# tmux Checklist

- Import a `tmux` session and verify topology plus screen snapshot.
- Exercise detach/reattach around an imported `tmux` session.
- Run `vim`, `less`, and `fzf` inside imported panes and confirm viewport fidelity.
"#;

pub(super) const VALID_ZELLIJ_CHECKLIST: &str = r#"
# Zellij Checklist

- Discover and import a live `Zellij` session through the daemon.
- For rich `0.44+`, verify topology, focused pane screen, subscriptions, and ordered mutation lane.
- Exercise viewport observation while switching tabs rapidly.
- Exercise detach/reattach around an imported `Zellij` session when the host environment supports it.
- Run `vim`, `less`, and `fzf` in a terminal pane and confirm render stability.
"#;

pub(super) const VALID_WINDOWS_NATIVE_ZELLIJ_CHECKLIST: &str = r#"
# Windows Native + Zellij Checklist

- Verify staged and installed Node package flows on Windows, including the live `Zellij` import/control path through the package surface.
- Verify topology snapshot, screen snapshot, screen delta, and live viewport observation.
- Verify ordered mutation lane for `new_tab`, `rename_tab`, `focus_tab`, and `close_tab`.
- Run `vim`, `less`, and `fzf` in Windows native and imported `Zellij` panes when available, and confirm viewport fidelity.
- Stress resize churn while screen delta and live viewport observation remain active.
- Exercise Electron bridge lifecycle on Windows.
- Confirm `tmux` is absent from Windows acceptance and docs.
"#;

pub(super) const VALID_CI_WORKFLOW: &str = r#"
jobs:
  unix-matrix:
name: unix-${{ matrix.os }}
runs-on: ${{ matrix.os }}
strategy:
  matrix:
    os:
      - ubuntu-latest
      - macos-latest
steps:
  - run: tmux -V
  - run: zellij --version
  - run: cargo clippy --workspace --all-targets --all-features
  - run: cargo nextest run --profile ci --workspace
  - run: |
      node crates/terminal-node-napi/package/scripts/build-local-package.mjs
      node crates/terminal-node-napi/package/scripts/verify-package.mjs
      export npm_config_cache="$RUNNER_TEMP/npm-cache"
      node crates/terminal-node-napi/package/scripts/pack-local-package.mjs
      test -f "$TARBALL"
      npm install --ignore-scripts --no-audit --no-fund --no-package-lock
  - run: |
      cargo run -p xtask -- stage-capi-package
      cargo run -p xtask -- verify-capi-package
      cargo run -p xtask -- install-capi-package
      cargo run -p xtask -- verify-capi-install
  windows-v1:
name: windows-v1
runs-on: windows-latest
steps:
  - run: python .github/scripts/install_fzf.py --out $env:RUNNER_TEMP\\fzf-bin
  - run: |
      foreach ($tool in @("vim", "less", "fzf")) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
          throw "missing tool $tool"
        }
      }
  - run: zellij --version
  - run: cargo clippy --workspace --all-targets --all-features
  - run: >
      cargo nextest run
      --profile ci
      --test-threads 1
      -p terminal-backend-native
      -p terminal-daemon
      -p terminal-daemon-client
      -p terminal-node
      -p terminal-node-napi
      -p terminal-protocol
      -p terminal-testing
  - run: |
      node crates/terminal-node-napi/package/scripts/build-local-package.mjs
      node crates/terminal-node-napi/package/scripts/verify-package.mjs
      $env:npm_config_cache = Join-Path $env:RUNNER_TEMP "npm-cache"
      node crates/terminal-node-napi/package/scripts/pack-local-package.mjs
      Test-Path -Path $tarball -PathType Leaf
      npm install --ignore-scripts --no-audit --no-fund --no-package-lock
  governance:
name: governance
steps:
  - run: cargo run -p xtask -- verify-v1-readiness
  - uses: taiki-e/install-action@v2
    with:
      tool: cargo-deny,cargo-public-api,cargo-semver-checks,release-plz
  fuzz-baseline:
name: fuzz-baseline
steps:
  - uses: taiki-e/install-action@v2
    with:
      tool: cargo-fuzz
  - run: |
      cargo fuzz run protocol_frames
      cargo fuzz run tmux_layout
      cargo fuzz run zellij_surface
      cargo fuzz run screen_delta
"#;

pub(super) const VALID_RELEASE_READINESS_WORKFLOW: &str = r#"
on:
  workflow_dispatch:
jobs:
  release-readiness:
timeout-minutes: 45
steps:
  - run: cargo run -p xtask -- verify-v1-readiness --require-recorded-passes
  - uses: taiki-e/install-action@v2
    with:
      tool: cargo-public-api,cargo-semver-checks,release-plz
  - run: rustup toolchain install nightly --profile minimal
  - run: |
      cargo +nightly public-api -p terminal-domain
      cargo +nightly public-api -p terminal-protocol
      cargo +nightly public-api -p terminal-node
  - run: cargo semver-checks --version
"#;

pub(super) const VALID_RELEASE_PLZ_WORKFLOW: &str = r#"
permissions:
  contents: write
  pull-requests: write
jobs:
  release-pr:
timeout-minutes: 30
steps:
  - run: release-plz release-pr --git-token "$GITHUB_TOKEN"
"#;

pub(super) const VALID_RELEASE_PLZ_CONFIG: &str = r#"
[workspace]
allow_dirty = false
git_release_enable = false
pr_branch_prefix = "release-plz-"
semver_check = false
"#;

pub(super) const VALID_DENY_CONFIG: &str = r#"
[advisories]
yanked = "deny"

[licenses]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
"#;

pub(super) const VALID_WINDOWS_ZELLIJ_SMOKE_TEST: &str = r#"
#[cfg(windows)]
let zellij_smoke = support::windows_zellij_smoke_env("package");

#[cfg(windows)]
command
.env("TERMINAL_NODE_RUN_ZELLIJ_SMOKE", "1")
.env("TERMINAL_NODE_EXTERNAL_ZELLIJ_SESSION", &zellij_smoke.session_name);
"#;
