# terminal-platform-node

Thin Node/Electron SDK for Terminal Platform.

This package is intentionally a leaf adapter:

- Rust runtime truth stays outside the package
- native transport semantics stay daemon-first
- JS receives a stable loader and typed client surface

## Local staging

Stage a publishable package directory from a compiled addon:

```bash
node ./scripts/stage-package.mjs \
  --out /tmp/terminal-platform-node \
  --addon /path/to/libterminal_node_napi.so
```

The staged directory contains:

- `index.cjs`
- `index.mjs`
- `index.d.ts`
- `bindings/*.d.ts`
- `native/terminal_node_napi.node`
