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

After the recorded manual pass files are added:

```bash
cargo run -p xtask -- verify-v1-readiness --require-recorded-passes
```

## GitHub closeout steps

1. Push `main` to the canonical GitHub remote.
2. Wait for the hosted CI matrix to go green.
3. Add recorded manual pass files under `crates/terminal-testing/manual/runs/`.
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

```md
## Terminal Platform v1

### Support matrix
- macOS + Linux - Native + tmux + Zellij
- Windows - Native + Zellij
- tmux remains Unix-only in v1

### Included in v1
- daemon-first local transport
- native PTY runtime with topology and mux control
- conservative tmux import/control support
- conservative Zellij import/control support
- Node/Electron package surface

### Not promised in v1
- fake parity between Native, tmux, and Zellij
- Windows tmux support
- plugin parity or floating-pane parity from Zellij
- release signing, notarization, or installer families

### Known degraded semantics
- legacy Zellij 0.43.x import remains explicit `MissingCapability`
- imported backends are capability-gated, not product truth
```
