# Terminal Platform

Reusable Rust terminal platform with:

- `NativeMux` as canonical runtime truth
- daemon-first host protocol
- conservative foreign backend adapters for `tmux` and `Zellij`
- Node/Electron as the first consumer, not the architectural center

## Start here

Read in this order:

1. [`docs/terminal/start-here-v1-implementation-pack.md`](./docs/terminal/start-here-v1-implementation-pack.md)
2. [`docs/terminal/final-v1-blueprint-rust-terminal-platform.md`](./docs/terminal/final-v1-blueprint-rust-terminal-platform.md)
3. [`docs/terminal/v1-workspace-bootstrap-spec.md`](./docs/terminal/v1-workspace-bootstrap-spec.md)
4. [`docs/terminal/v1-implementation-roadmap-and-task-breakdown.md`](./docs/terminal/v1-implementation-roadmap-and-task-breakdown.md)
5. [`docs/terminal/v1-verification-and-acceptance-plan.md`](./docs/terminal/v1-verification-and-acceptance-plan.md)

## Workspace

Current workspace shape:

- `crates/terminal-domain`
- `crates/terminal-mux-domain`
- `crates/terminal-backend-api`
- `crates/terminal-protocol`
- `crates/terminal-application`
- `crates/terminal-projection`
- `crates/terminal-persistence`
- `crates/terminal-backend-native`
- `crates/terminal-backend-tmux`
- `crates/terminal-backend-zellij`
- `crates/terminal-daemon`
- `crates/terminal-daemon-client`
- `crates/terminal-node`
- `crates/terminal-capi`
- `crates/terminal-testing`

## Quality gates

Bootstrap target:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo nextest run --workspace`

If `cargo nextest` is not installed yet, bootstrap work may temporarily use `cargo test --workspace` until the tool is installed.

