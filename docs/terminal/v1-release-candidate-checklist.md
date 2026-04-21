# V1 Release Candidate Checklist

This is the final closeout checklist for calling Terminal Platform a public ship-ready v1.

## Required proof

- canonical GitHub remote exists and `main` is pushed
- GitHub Actions matrix is green:
  - `unix-matrix`
  - `windows-v1`
  - `governance`
  - `fuzz-baseline`
- recorded manual pass artifacts exist:
  - one Electron embed pass
  - one Unix `tmux` pass
  - one Windows `Native + Zellij` pass
- support matrix is still synced between the root README and Node package README
- no-scope-creep audit confirms v1 promises are unchanged

## Local closeout commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo nextest run --workspace
cargo run -p xtask -- verify-v1-readiness
```

Package layout and install proof:

```bash
node crates/terminal-node-napi/package/scripts/build-local-package.mjs --out /tmp/terminal-platform-node
node crates/terminal-node-napi/package/scripts/verify-package.mjs --package-dir /tmp/terminal-platform-node
cargo build -p terminal-capi
cargo run -p xtask -- stage-capi-package --out /tmp/terminal-capi
cargo run -p xtask -- verify-capi-package --package-dir /tmp/terminal-capi
cargo run -p xtask -- install-capi-package --package-dir /tmp/terminal-capi --prefix /tmp/terminal-capi-install
cargo run -p xtask -- verify-capi-install --prefix /tmp/terminal-capi-install
```

After the recorded manual pass files are added:

```bash
cargo run -p xtask -- verify-v1-readiness --require-recorded-passes
```

## GitHub closeout steps

1. Push `main` to the canonical GitHub remote.
2. Wait for the hosted CI matrix to go green.
3. Add recorded manual pass files under `crates/terminal-testing/manual/runs/`.
   Start with drafts under `crates/terminal-testing/manual/drafts/`, then move only completed `Result: pass` artifacts into `manual/runs/`.
4. Re-run the readiness audit in strict mode.
5. Trigger `.github/workflows/release-readiness.yml`.
6. Trigger `.github/workflows/release-plz.yml` or let the `main` push update the release PR.

## No-scope-creep audit

Verify the public v1 promise is still exactly:

- `macOS + Linux` - `Native + tmux + Zellij`
- `Windows` - `Native + Zellij`
- `tmux` remains Unix-only in docs, CI, and acceptance
- `NativeMux` remains canonical runtime truth
- `tmux` and `Zellij` remain capability-gated foreign adapters
- no new host SDK surface beyond the current Rust protocol, Node/Electron package, and documented C ABI leaf

## Release summary template

Use:

- [`v1-manual-closeout-runbook.md`](./v1-manual-closeout-runbook.md)
- [`v1-release-candidate-summary.md`](./v1-release-candidate-summary.md)
- [`v1-release-summary-template.md`](./v1-release-summary-template.md)
