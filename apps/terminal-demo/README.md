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

Renderer-only UI preview for sandboxed or offline checks:

```bash
cd apps/terminal-demo
npm run smoke:renderer-static
open "file://$(pwd)/dist/renderer/index.html?demoStaticWorkspace=1"
```

This mode renders the full workspace shell from a static NativeMux preview snapshot and verifies that the renderer bundle still includes the static preview contract. It is useful for fast visual QA when the native runtime or localhost preview server is unavailable, but it does not replace `npm run smoke:browser`.

Fast offline verification for sandboxed or dependency-constrained environments:

```bash
cd apps/terminal-demo
npm run test:offline
```

This gate verifies architecture boundaries, renderer type safety, the React/static workspace composition contract, and the static renderer bundle without requiring Cargo network access, a native daemon build, or a localhost preview server.

## Verification Matrix

| Command | Scope | Requires |
| --- | --- | --- |
| `npm run test:offline` | fresh workspace SDK staging, architecture boundaries, renderer types, static workspace composition, renderer bundle, terminal layout contracts | Node dependencies |
| `npm run smoke:renderer-static` | fresh workspace SDK staging, renderer bundle, static NativeMux preview contract | Node dependencies |
| `npm run verify:renderer-static:browser` | static NativeMux preview in real headless Chrome, command input flow, attached terminal layout, screenshot artifact | Node dependencies, Chrome CDP |
| `cd ../../sdk && npm run test:public-api` | workspace elements and React public exports, composer action IDs, keyboard hints, row layout helpers | Node dependencies |
| `npm run smoke:browser` | full native host, WebSocket gateway, browser UI, terminal layout, command composer interaction | Cargo dependencies, local bind to `127.0.0.1`, Chrome |

Use `npm run test:offline` as the fast sandbox gate and `cd ../../sdk && npm run test:public-api` before changing SDK exports. Use `npm run verify:renderer-static:browser` for real Chrome QA when native dependencies are unavailable. Use `npm run smoke:browser` before release or when the native runtime and localhost browser preview are available.

Chrome-based smoke commands use `TERMINAL_DEMO_CHROME_BIN` when Chrome is not installed in a standard location. They try modern and legacy headless modes by default. Set `TERMINAL_DEMO_STATIC_SMOKE_HEADLESS_MODE=new`, `old`, or `new,old` for `verify:renderer-static:browser`; set `TERMINAL_DEMO_SMOKE_HEADLESS_MODE` the same way for `smoke:browser`.

## Notes

- `postinstall` first builds and links the local workspace SDK packages into `node_modules/@terminal-platform/`
- `stage:workspace-sdk` performs only that JS SDK staging step for offline renderer checks
- the same staging step then stages the local `terminal-platform-node` SDK into `.generated/`
- the same staging step also builds the local `terminal-daemon` binary in `target/debug/`
- if native Rust dependencies are unavailable, the staging step still leaves JS workspace packages linked for renderer-only UI checks while failing the native step explicitly
- default runtime slug is `terminal-demo`, not `default`, so the demo does not attach to unrelated local daemons
- the gateway normalizes `bigint` fields into decimal strings for JSON-safe WebSocket transport
- if persistence store bootstrap fails, the daemon falls back to in-memory mode and logs the reason
