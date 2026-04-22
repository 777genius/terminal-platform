# Feature Architecture Standard

**Status**: app standard  
**Reference implementation**: `src/features/terminal-workspace`

This document defines the default architecture for medium and large features in `apps/terminal-demo`.

## Goals

- keep business rules isolated from Electron-specific runtime details
- make features easier to scale, test, and review
- keep renderer code closer to browser portability
- enforce architecture through structure and public entrypoints

## Canonical Template

```text
src/features/<feature-name>/
  contracts/
  core/
    domain/
    application/
  main/
    composition/
    adapters/
      input/
      output/
    infrastructure/
  preload/
  renderer/
```

Use this template by default when a feature:

- spans more than one process boundary
- introduces its own use case or business policy
- needs its own transport bridge or integration surface
- is expected to grow with new providers, sources, or presentation flows

## Layer Responsibilities

### `contracts/`

Cross-process public API for the feature.

Allowed content:

- DTOs
- API fragment types
- transport message constants and shapes

Not allowed:

- store access
- Electron APIs
- business orchestration

### `core/domain/`

Pure business rules and invariants.

Examples:

- merge policies
- provider-agnostic models
- selection rules
- dedupe logic
- pure feature helpers

Not allowed:

- infrastructure access
- framework access
- side effects

### `core/application/`

Use cases and ports.

Examples:

- orchestration flow
- output ports
- source ports
- response models

Not allowed:

- Electron, React, Zustand, child processes, WebSocket server/client implementations

### `main/composition/`

Feature composition root in the main process.

Responsibilities:

- instantiate infrastructure
- wire adapters
- wire use cases
- expose a small facade to app shell entrypoints

### `main/adapters/input/`

Driving adapters for the main process.

Examples:

- WebSocket gateway handlers
- IPC handlers
- HTTP route registration

Responsibilities:

- translate transport input into use case calls
- keep transport concerns out of use cases

### `main/adapters/output/`

Driven adapters that implement application ports.

Examples:

- presenters
- runtime adapters
- source adapters

Responsibilities:

- translate between external data and core models
- stay thin around infrastructure helpers

### `main/infrastructure/`

Concrete technical implementation details.

Examples:

- file system adapters
- JSON-RPC transport clients
- binary discovery
- cache implementation

Responsibilities:

- know about runtime, process, OS, or protocol details

### `preload/`

Thin transport bridge between renderer and main.

Responsibilities:

- expose a feature API fragment
- depend on `contracts/`

Not allowed:

- main composition code
- renderer logic

### `renderer/`

Feature presentation and interaction.

Recommended structure:

```text
renderer/
  index.ts
  adapters/
  hooks/
  ui/
  utils/
```

Responsibilities:

- `ui/` renders
- `hooks/` orchestrate interaction and state usage
- `adapters/` transform transport/bootstrap into renderer-facing APIs
- `utils/` contain small pure renderer helpers

## Import Rules

### Public entrypoints only

Outside the feature, import only:

- `@features/<feature>/contracts`
- `@features/<feature>/main`
- `@features/<feature>/preload`
- `@features/<feature>/renderer`

Do not deep-import feature internals from app shell or from other features.

### Core isolation

`core/domain` must not import:

- `main/*`
- `renderer/*`
- `preload/*`
- adapters
- infrastructure
- Electron APIs
- child process modules
- `ws`

`core/application` must not import:

- `main/*`
- `renderer/*`
- Electron APIs
- child process modules
- `ws`

### UI isolation

`renderer/ui` must not import:

- app shell modules
- `main/*`
- Electron APIs
- runtime transport implementations

Push transport and store access into feature hooks or adapters.

## Browser-Friendly Guidance

The default transport direction should be:

`renderer -> feature contracts -> renderer adapter -> preload/http/ws adapter`

To keep that path clean:

- never call `window.terminalDemo` directly inside feature UI
- keep Electron-specific concerns in `main/` and `preload/`
- keep business rules in `core/`

## Definition Of Done For A Reference Feature

A feature is reference-quality when:

- structure matches the canonical template
- core is side-effect free
- app shell imports only public entrypoints
- renderer UI is dumb and presentational
- main domain and application rules are isolated from framework/runtime code
- feature has a concise local README if it introduces a reusable pattern
