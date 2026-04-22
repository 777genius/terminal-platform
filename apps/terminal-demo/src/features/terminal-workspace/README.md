# Terminal Workspace Feature

This feature is the canonical full-slice example for `apps/terminal-demo`.

Read first:
- [Feature Architecture Standard](../../../docs/FEATURE_ARCHITECTURE_STANDARD.md)
- [Feature root guidance](../CLAUDE.md)

Public entrypoints:
- `@features/terminal-workspace/contracts`
- `@features/terminal-workspace/main`
- `@features/terminal-workspace/preload`
- `@features/terminal-workspace/renderer`

Responsibilities:
- `contracts/` owns bootstrap config, DTOs, transport messages, and public types
- `core/` owns pure feature semantics and framework-free orchestration
- `main/` owns daemon supervision, runtime adapters, and WebSocket gateway composition
- `preload/` owns the Electron preload bridge and path resolution
- `renderer/` owns browser/Electron bootstrap adapters, hook orchestration, and presentational UI
- foreign backend refs stay inside `main/` runtime/input adapters; renderer sees only `origin` metadata and opaque `importHandle` values
- control plane and data plane are separate by design:
  - control plane handles request/reply for catalog, capabilities, create/import/restore, and mux commands
  - session state stream handles only live `session_state` subscription traffic
- bootstrap config is explicit:
  - host publishes `controlPlaneUrl` and `sessionStreamUrl`
  - renderer never derives transport topology from hidden host conventions, except for legacy fallback compatibility
- main output adapters are split by responsibility:
  - control runtime adapter owns request/reply use cases
  - session-state runtime adapter owns live watch semantics
- session stream adapter owns reconnect and resubscribe behavior for transient socket loss
- application state exposes explicit `sessionStreamHealth` so reconnecting and failed live-stream states are visible without overloading action errors
- degraded semantics are explicit public contracts, not implicit transport failures

Quality gates:
- `npm run check:architecture` verifies feature boundary rules
- `npm test` verifies domain and application seams against the built host artifacts
