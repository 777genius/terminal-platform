# Terminal Platform v1 Release Summary Template

Use this as the starting point for the first public v1 release PR or release notes.

```md
## Terminal Platform v1

Embeddable terminal platform in Rust with a native PTY runtime, daemon-first transport,
and capability-gated `tmux` and `Zellij` adapters for Electron and other hosts.

### Support matrix

- `macOS + Linux` - `Native + tmux + Zellij`
- `Windows` - `Native + Zellij`
- `tmux` remains Unix-only in docs, CI, and acceptance

### Included in v1

- daemon-first local transport over Unix sockets and Windows named pipes
- native PTY runtime with sessions, tabs, panes, topology snapshots, screen snapshots, and screen deltas
- conservative `tmux` import and control support
- conservative rich `Zellij 0.44+` import and ordered mutation support
- Node and Electron package surface through `napi-rs`
- documented C ABI leaf for non-Node embedders

### What this release is for

- IDE terminal surfaces
- Electron apps with real terminal sessions
- agent or workspace products that need Rust-owned terminal runtime truth
- host applications that want a reusable terminal runtime layer instead of a UI-bound widget

### Not promised in v1

- fake parity between Native, `tmux`, and `Zellij`
- Windows `tmux` support
- full `Zellij` floating-pane parity
- signing, notarization, or installer families

### Known degraded semantics

- legacy `Zellij 0.43.x` import remains explicit `MissingCapability`
- imported backends are capability-gated foreign adapters, not product truth
- backend-specific behavior is exposed through capability truth instead of hidden fallback behavior

### Verification status

- hosted `ci` green on `unix-matrix`, `windows-v1`, `governance`, and `fuzz-baseline`
- `cargo run -p xtask -- verify-v1-readiness --require-recorded-passes` green
- recorded manual pass artifacts captured for Electron embed, Unix `tmux`, and Windows `Native + Zellij`
- release PR generated or updated by `release-plz`

### Repository

- https://github.com/777genius/terminal-platform
```
