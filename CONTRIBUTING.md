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
