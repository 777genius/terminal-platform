# terminal-capi

`terminal-capi` is the C ABI host surface for Terminal Platform.

It exists for non-Node embedders that still want access to the Rust runtime, daemon protocol, and subscription model through a narrow, explicit FFI boundary.

## What This Surface Provides

- opaque client handles
- opaque subscription handles
- JSON request and reply carriers for the public daemon contract
- explicit `open -> poll -> close -> free` lifecycle
- generated headers via `cbindgen`

## Why It Stays Narrow

This crate is intentionally conservative.

- Rust still owns runtime truth
- public protocol DTOs still stay in the Rust domain and daemon contract
- C gets a stable embedding seam, not a second architecture

## Build Outputs

The crate currently ships as:

- `cdylib`
- `staticlib`
- `rlib`

## Current Verification

`terminal-capi` is covered by real integration paths, not only unit tests.

Current coverage includes:

- header generation through `cbindgen`
- Rust-side C ABI smoke against a live daemon fixture
- external C consumer smoke that compiles and links against the generated header and built library
- recovery coverage for shutdown and restart flows
- staged package verification through `xtask`
- installed prefix verification through `xtask`
- `pkg-config` metadata generation for conventional C consumer integration

## Local Verification

Typical local loop:

```bash
cargo test -p terminal-capi -- --nocapture
```

Stage and verify a local C package layout:

```bash
cargo run -p xtask -- stage-capi-package --out ./crates/terminal-capi/artifacts/local
cargo run -p xtask -- verify-capi-package --package-dir ./crates/terminal-capi/artifacts/local
cargo run -p xtask -- install-capi-package --package-dir ./crates/terminal-capi/artifacts/local --prefix ./crates/terminal-capi/artifacts/install
cargo run -p xtask -- verify-capi-install --prefix ./crates/terminal-capi/artifacts/install
```

## Package Layout

The staged package includes:

- `include/terminal-platform-capi.h`
- `lib/<dynamic library>`
- `lib/<static library>`
- `lib/pkgconfig/terminal-platform-capi.pc`
- `manifest.json`

Installed prefix layout includes:

- `include/terminal-platform-capi.h`
- `lib/<dynamic library>`
- `lib/<static library>`
- `lib/pkgconfig/terminal-platform-capi.pc`
- `share/terminal-capi/manifest.json`
- `share/terminal-capi/README.md`

## Status

⚠️ This is a documented secondary embedding surface for v1, not the primary product entrypoint.

For v1:

- Node and Electron remain the primary host guarantee
- the C ABI is still a real, tested leaf
- Windows public host promise is still centered on `Node/Electron/package`, not on expanding the C ABI promise separately

## Repository

- [Terminal Platform repository](https://github.com/777genius/terminal-platform)
