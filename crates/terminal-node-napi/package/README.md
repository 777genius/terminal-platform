# terminal-platform-node

Node and Electron SDK for Terminal Platform.

This package gives JavaScript hosts a typed client surface over the Rust runtime, daemon transport, native PTY engine, and imported multiplexer adapters. It is designed for Electron apps, IDEs, terminal tools, and agent workspaces that want a serious terminal runtime without moving terminal truth into JS.

## What This Package Is

- a typed Node client for the Terminal Platform runtime
- an Electron bridge for main, preload, and renderer integration
- a stable JS-facing surface over daemon transport and runtime state
- a host SDK, not the source of terminal truth

## What This Package Is Not

- a browser-only terminal widget
- a fake universal wrapper that hides backend differences
- a reason to move PTY lifecycle and mux logic out of Rust

## V1 Runtime Support Promise

- `macOS + Linux` - `Native + tmux + Zellij`
- `Windows` - `Native + Zellij`
- `tmux` stays Unix-only in v1 docs, tests, CI, and acceptance

## Current Package Surface

The client currently supports:

- handshake and capability queries
- native session create, list, attach, and restore
- `tmux` and `Zellij` discovery plus import
- topology snapshots, screen snapshots, and screen deltas
- live topology and pane subscriptions
- `watchTopology`, `watchPane`, `watchSession`, and `watchSessionState`
- render-ready session state reducers
- Electron main bridge helpers
- Electron preload bridge helpers for `contextIsolation`
- mux control operations for tabs, panes, input, and save flows

## Quick Example

```js
const { TerminalNodeClient } = require("terminal-platform-node");

const client = TerminalNodeClient.fromRuntimeSlug("default");
const session = await client.createNativeSession({
  name: "workspace",
});

const state = await client.watchSessionState(session.sessionId, (nextState) => {
  render(nextState);
});

await state.close();
await client.close();
```

## Electron Integration

Main process:

```js
const {
  TerminalNodeClient,
  createElectronMainBridge,
} = require("terminal-platform-node");

const client = TerminalNodeClient.fromRuntimeSlug("default");
const bridge = createElectronMainBridge({ ipcMain, client });
```

Preload:

```js
const { installElectronPreloadBridge } = require("terminal-platform-node");

installElectronPreloadBridge({
  contextBridge,
  ipcRenderer,
  exposeKey: "terminalPlatform",
});
```

Renderer:

```js
const subscriptionId = await window.terminalPlatform.subscribeSessionState(
  sessionId,
  (state) => {
    render(state);
  },
);

await window.terminalPlatform.unsubscribeSessionState(subscriptionId);
```

## Reliability Notes

This package is covered by more than happy-path demos.

Current verification includes:

- direct addon smoke
- staged package smoke
- installed tarball smoke
- CJS and ESM entrypoint coverage
- shutdown and restart recovery coverage
- subscription close and backlog-drain coverage
- Electron bridge lifecycle smoke

The package-level watch helpers are explicitly tested for:

- daemon shutdown under active subscriptions
- repeated open and close cycles
- restart recovery on the same client instance
- main and preload disposal during live session-state watchers

## Packaging And Local Staging

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

Build, verify, and pack a local tarball:

```bash
node ./scripts/pack-local-package.mjs --out /tmp/terminal-platform-node
```

The staged package contains:

- `index.cjs`
- `index.mjs`
- `index.d.ts`
- `bindings/*.d.ts`
- `native/manifest.json`
- `native/terminal_node_napi.<platform>.<arch>[.<libc>].node`

## Project Status

⚠️ This package is part of the Terminal Platform v1 release-candidate closeout.

That means:

- the host surface is already real and deeply tested
- the remaining work is mostly hosted CI proof, final acceptance evidence, and release polish
- the package should be evaluated as a serious SDK surface, not as an early sketch

## Repository

- [Terminal Platform repository](https://github.com/777genius/terminal-platform)
