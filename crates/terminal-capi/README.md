# `terminal-capi`

`terminal-capi` is the C ABI leaf for non-Node embedders.

It intentionally stays narrow:

- opaque client and subscription handles
- JSON request/reply carriers for the public daemon contract
- explicit `open -> poll -> close -> free` subscription lifecycle
- generated headers via `cbindgen`

## Build shape

The crate ships as:

- `cdylib`
- `staticlib`
- `rlib`

## Header generation

`terminal-capi` keeps the public C header generated from Rust definitions.

Current test coverage includes:

- header generation via `cbindgen`
- Rust-side C ABI smoke against a live daemon fixture
- external C consumer smoke that compiles a real C program against the generated header and links to the built `cdylib`
- external consumer coverage for native request/subscription flow and `tmux` discover/import flow
- staged package smoke via `cargo run -p xtask -- stage-capi-package`
- staged package emits `pkg-config` metadata for standard C consumer integration

## Local verification

Typical local loop:

```bash
cargo test -p terminal-capi -- --nocapture
```

Stage and verify a local C package layout:

```bash
cargo run -p xtask -- stage-capi-package --out ./crates/terminal-capi/artifacts/local
cargo run -p xtask -- verify-capi-package --package-dir ./crates/terminal-capi/artifacts/local
```

The staged package now includes:

- `include/terminal-platform-capi.h`
- `lib/<dynamic library>`
- `lib/<static library>`
- `lib/pkgconfig/terminal-platform-capi.pc`
- `manifest.json`

Full workspace quality gates:

```bash
cargo nextest run --workspace
cargo clippy --workspace --all-targets --all-features
```
