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

Build the addon and stage a local package in one command:

```bash
node ./scripts/build-local-package.mjs --out /tmp/terminal-platform-node
```

Build, verify and pack a local tarball:

```bash
node ./scripts/pack-local-package.mjs --out /tmp/terminal-platform-node
```

The staged directory contains:

- `index.cjs`
- `index.mjs`
- `index.d.ts`
- `bindings/*.d.ts`
- `native/manifest.json`
- `native/terminal_node_napi.<platform>.<arch>[.<libc>].node`
