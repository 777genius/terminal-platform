# Contributing

Thanks for contributing to Terminal Platform.

## Before You Change Code

Read these first:

1. [`README.md`](./README.md)
2. [`docs/terminal/start-here-v1-implementation-pack.md`](./docs/terminal/start-here-v1-implementation-pack.md)
3. [`docs/terminal/v1-implementation-roadmap-and-task-breakdown.md`](./docs/terminal/v1-implementation-roadmap-and-task-breakdown.md)
4. [`docs/terminal/v1-verification-and-acceptance-plan.md`](./docs/terminal/v1-verification-and-acceptance-plan.md)

## Core Rules

- keep `NativeMux` as canonical runtime truth
- do not introduce fake parity between `Native`, `tmux`, and `Zellij`
- keep backend-specific semantics behind capability truth
- keep UI concerns out of the Rust runtime core
- prefer additive hardening over scope creep during v1 closeout

## Development Loop

Main workspace gates:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo nextest run --workspace
```

Readiness audit:

```bash
cargo run -p xtask -- verify-v1-readiness
```

Package proof for release-facing changes:

```bash
node crates/terminal-node-napi/package/scripts/build-local-package.mjs --out /tmp/terminal-platform-node
node crates/terminal-node-napi/package/scripts/verify-package.mjs --package-dir /tmp/terminal-platform-node
export npm_config_cache=/tmp/terminal-platform-node-npm-cache
TARBALL="$(node crates/terminal-node-napi/package/scripts/pack-local-package.mjs --out /tmp/terminal-platform-node-pack | tail -n 1)"
test -f "$TARBALL"
cargo build -p terminal-capi
cargo run -p xtask -- stage-capi-package --out /tmp/terminal-capi
cargo run -p xtask -- verify-capi-package --package-dir /tmp/terminal-capi
cargo run -p xtask -- install-capi-package --package-dir /tmp/terminal-capi --prefix /tmp/terminal-capi-install
cargo run -p xtask -- verify-capi-install --prefix /tmp/terminal-capi-install
```

Strict release handoff gate:

```bash
cargo run -p xtask -- verify-v1-readiness --require-recorded-passes
```

Offline handoff when GitHub is unavailable from the working environment:

```bash
git format-patch origin/main..HEAD --stdout > terminal-platform-v1-closeout-local.patch
git bundle create terminal-platform-v1-closeout.bundle origin/main..HEAD
git bundle verify terminal-platform-v1-closeout.bundle
```

Apply the patch or bundle from a network-enabled checkout, rerun `verify-v1-readiness`, then push.

## Pull Requests

- keep changes narrow and intentional
- explain user-facing impact and architecture impact
- add or strengthen regression coverage for every real bug
- use conventional commits for commit messages
- do not weaken the published support matrix just to make CI pass

## Manual Acceptance

Ship-ready v1 still requires recorded manual passes for:

- Electron embed
- Unix `tmux`
- Windows `Native + Zellij`

See:

- [`crates/terminal-testing/manual/`](./crates/terminal-testing/manual/)
- [`crates/terminal-testing/manual/runs/`](./crates/terminal-testing/manual/runs/)

## Scope Discipline

If you are unsure whether a change belongs in v1, default to:

- reliability
- verification
- packaging polish
- documentation clarity

Default away from:

- new runtime families
- fake compatibility layers
- large public API redesigns
