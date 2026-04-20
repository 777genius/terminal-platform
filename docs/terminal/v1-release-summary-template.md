# Terminal Platform v1 Release Summary Template

Use this as the starting point for the first public v1 release PR or release notes.

```md
## Terminal Platform v1

Embeddable terminal platform in Rust with a native PTY runtime, daemon-first transport,
and capability-gated `tmux` and `Zellij` adapters for Electron and other hosts.

### Support matrix

- macOS + Linux - Native + tmux + Zellij
- Windows - Native + Zellij
- tmux remains Unix-only in v1

### Included in v1

- daemon-first local transport
- native PTY runtime with topology and mux control
- conservative tmux import and control support
- conservative Zellij import and control support
- Node and Electron package surface
- documented C ABI leaf for non-Node embedders

### What this release is for

- IDE terminal surfaces
- Electron apps with real terminal sessions
- agent or workspace products
- host applications that want terminal runtime truth in Rust, not in JS

### Not promised in v1

- fake parity between Native, tmux, and Zellij
- Windows tmux support
- full floating-pane parity from Zellij
- release signing, notarization, or installer families

### Known degraded semantics

- legacy Zellij 0.43.x import remains explicit `MissingCapability`
- imported backends are capability-gated, not product truth
- backend-specific behavior is exposed through capability truth, not hidden behind fake parity

### Verification status

- hosted CI matrix green
- readiness audit green
- required recorded manual passes captured

### Repository

- https://github.com/777genius/terminal-platform
```
