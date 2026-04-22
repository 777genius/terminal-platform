# Feature Architecture Standard

**Status**: app standard  
**Reference implementation**: `src/features/terminal-runtime-host`

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

For shared app-owned DTOs and pure policies used by multiple direct features, prefer a shared kernel feature such as `src/features/terminal-workspace-kernel`.

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
