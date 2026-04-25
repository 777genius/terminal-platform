# Terminal Demo

Terminal Platform demo host with both Electron and browser entrypoints.

## What It Proves

- renderer can consume the public SDK packages directly
- renderer is not coupled to Electron IPC
- Electron stays a thin desktop shell
- host starts a local `terminal-daemon` sidecar
- renderer talks to a local WebSocket gateway
- terminal truth still lives in Rust runtime and protocol

## Stack

- Electron 41.2.2
- React 19.2.5
- Zustand 5.0.12
- Vite 8.0.9
- TypeScript 6.0.3
- `ws` 8.20.0

## Architecture

```text
src/renderer/app/*
  -> @features/terminal-workspace/renderer
  -> @features/terminal-workspace/contracts

src/host/*
  -> @features/terminal-workspace/main
  -> @features/terminal-workspace/preload
  -> @features/terminal-workspace/contracts
```

### Boundaries

- `src/features/terminal-workspace/contracts/` - bootstrap config, DTOs, transport messages
- `src/features/terminal-workspace/core/` - pure semantics and framework-free orchestration
- `src/features/terminal-workspace/main/` - daemon supervision, runtime adapters, WS gateway composition
- `src/features/terminal-workspace/preload/` - Electron preload bridge and preload path helper
- `src/features/terminal-workspace/renderer/` - SDK consumer bootstrap and React shell for `tp-terminal-*`
- `src/host/` and `src/renderer/app/` - thin shell entrypoints only

Feature standard lives in:

- `docs/FEATURE_ARCHITECTURE_STANDARD.md`
- `src/features/README.md`

## Run

From the repo root or this folder:

```bash
cd apps/terminal-demo
npm install
npm run dev
```

Browser mode with the same daemon and WebSocket gateway:

```bash
cd apps/terminal-demo
npm run dev:browser
```

The browser runner prints a `TERMINAL_DEMO_BROWSER_URL=...` line. Open that URL in Chrome or another browser. Browser mode uses a temporary session store and lets the host create the initial NativeMux shell before the URL is published, so repeated demos open cleanly into one usable terminal. Set `TERMINAL_DEMO_DEFAULT_SHELL=/path/to/shell` to override the launch shell. Set `TERMINAL_DEMO_AUTO_START_SESSION=0` to keep explicit manual launch. Set `TERMINAL_DEMO_SESSION_STORE_PATH=/path/to/session-store.sqlite3` to inspect a specific store, or `TERMINAL_DEMO_BROWSER_PERSIST_SESSION_STORE=1` to use the default persistent daemon store.

Production-style local run:

```bash
cd apps/terminal-demo
npm run build
npm run preview
```

## Notes

- `postinstall` stages the local `terminal-platform-node` SDK into `.generated/`
- the same staging step builds and links the local workspace SDK packages into `node_modules/@terminal-platform/`
- the same staging step also builds the local `terminal-daemon` binary in `target/debug/`
- default runtime slug is `terminal-demo`, not `default`, so the demo does not attach to unrelated local daemons
- the gateway normalizes `bigint` fields into decimal strings for JSON-safe WebSocket transport
- if persistence store bootstrap fails, the daemon falls back to in-memory mode and logs the reason
