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
- `crates/terminal-node-napi`
- `crates/terminal-capi`
- `crates/terminal-testing`

## Node package surface

`terminal-node` owns safe Rust DTO and facade truth.

`terminal-node-napi` owns the Node/Electron leaf:

- raw native addon via `napi-rs`
- staged package surface in `crates/terminal-node-napi/package`
- `index.cjs` and `index.mjs` loader entrypoints
- `index.d.ts` plus generated `bindings/*.d.ts`
- publish/install smoke through a packed local tarball in a temp Node project

## C ABI surface

`terminal-capi` is the secondary host leaf for non-Node embedders.

Current shape:

- opaque client handle constructors for runtime slug, namespaced address and filesystem path
- JSON request/reply functions for handshake, session listing, native session create, attach, topology, screen snapshot, screen delta and mux dispatch
- explicit subscription handles with open, poll-next-event, close and free semantics
- Rust-owned C string carriers plus explicit free functions
- checked-in `cbindgen.toml` plus tested header generation path
- external C consumer smoke that compiles against the generated header and the built `cdylib`
- `xtask` packaging lane for staging and verifying a local C package layout
- staged C package now includes `pkg-config` metadata for conventional host integration

## Quality gates

Bootstrap target:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo nextest run --workspace`

If `cargo nextest` is not installed yet, bootstrap work may temporarily use `cargo test --workspace` until the tool is installed.
