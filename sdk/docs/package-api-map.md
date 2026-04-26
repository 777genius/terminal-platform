# Package API Map

**Checked**: 2026-04-22  
**Status**: frozen package and entrypoint map

## Goal

Freeze the intended public API shape of each package before implementation starts.

This document does not replace detailed API design during implementation, but it defines the public entrypoint boundaries so packaging and SemVer stay under control.

## Naming

Package names are fixed as:

- `@terminal-platform/foundation`
- `@terminal-platform/runtime-types`
- `@terminal-platform/design-tokens`
- `@terminal-platform/workspace-contracts`
- `@terminal-platform/workspace-core`
- `@terminal-platform/workspace-adapter-websocket`
- `@terminal-platform/workspace-adapter-preload`
- `@terminal-platform/workspace-adapter-memory`
- `@terminal-platform/workspace-elements`
- `@terminal-platform/workspace-react`
- `@terminal-platform/testing`

## Public Entrypoint Policy

Every package must expose only explicit entrypoints through `exports`.

Default policy:

- `"."` is the main public entrypoint
- extra subpaths are allowed only when intentionally documented
- undocumented deep imports are forbidden

## Package Entry Expectations

### `@terminal-platform/foundation`

Main entrypoint:

- `@terminal-platform/foundation`

Expected exports:

- store primitives
- lifecycle primitives
- async primitives
- base errors
- telemetry interfaces

### `@terminal-platform/runtime-types`

Main entrypoint:

- `@terminal-platform/runtime-types`

Expected exports:

- generated public runtime mirrors only

### `@terminal-platform/design-tokens`

Main entrypoint:

- `@terminal-platform/design-tokens`

Optional documented subpaths:

- `@terminal-platform/design-tokens/css`
- `@terminal-platform/design-tokens/themes`

Expected exports:

- token metadata
- theme manifests
- CSS variable bundles

### `@terminal-platform/workspace-contracts`

Main entrypoint:

- `@terminal-platform/workspace-contracts`

Optional documented subpaths:

- `@terminal-platform/workspace-contracts/ports`
- `@terminal-platform/workspace-contracts/commands`
- `@terminal-platform/workspace-contracts/observations`
- `@terminal-platform/workspace-contracts/errors`

Expected exports:

- IDs
- models
- ports
- commands
- observations
- errors
- compat metadata

### `@terminal-platform/workspace-core`

Main entrypoint:

- `@terminal-platform/workspace-core`

Optional documented subpaths:

- `@terminal-platform/workspace-core/testing`

Expected exports:

- `WorkspaceKernel`
- kernel factory
- selectors
- diagnostics interfaces
- workspace read models, including `WorkspaceCommandHistorySnapshot`
- workspace preference defaults, including `DEFAULT_COMMAND_HISTORY_LIMIT`

### `@terminal-platform/workspace-adapter-websocket`

Main entrypoint:

- `@terminal-platform/workspace-adapter-websocket`
- `@terminal-platform/workspace-adapter-websocket/protocol`

Expected exports:

- websocket adapter factory
- config types
- gateway protocol types for host implementations

### `@terminal-platform/workspace-adapter-preload`

Main entrypoint:

- `@terminal-platform/workspace-adapter-preload`

Expected exports:

- preload adapter factory
- bridge config types

### `@terminal-platform/workspace-adapter-memory`

Main entrypoint:

- `@terminal-platform/workspace-adapter-memory`

Expected exports:

- fake adapter factory
- fixtures for examples/tests where appropriate

### `@terminal-platform/workspace-elements`

Main entrypoint:

- `@terminal-platform/workspace-elements`

Optional documented subpaths:

- `@terminal-platform/workspace-elements/define`
- `@terminal-platform/workspace-elements/styles`

Expected exports:

- public element classes
- `defineTerminalPlatformElements()`
- `TerminalCommandQuickCommand` and quick command defaults/resolvers for command dock customization
- command composer events, action presentations with stable action IDs and keyboard hints, layout helpers, row defaults, and typed event details
- documented style helpers only if necessary
- command dock and command composer events and parts documented in feature model docs

### `@terminal-platform/workspace-react`

Main entrypoint:

- `@terminal-platform/workspace-react`

Expected exports:

- React wrappers
- hooks
- event typings
- thin `TerminalCommandComposer` wrapper with typed custom-event props
- command composer action presentations and layout helpers re-exported from `@terminal-platform/workspace-elements`

### `@terminal-platform/testing`

Main entrypoint:

- `@terminal-platform/testing`

Expected exports:

- fakes
- conformance harnesses
- race helpers
- packed-consumer smoke helpers

## Custom Element Namespace

The public custom element namespace is frozen as:

- `tp-terminal-*`

Examples:

- `tp-terminal-workspace`
- `tp-terminal-session-list`
- `tp-terminal-toolbar`
- `tp-terminal-screen`
- `tp-terminal-pane-tree`
- `tp-terminal-saved-sessions`

## Registration Policy

Public element registration follows these rules:

- self-defining modules are supported
- element classes are exported
- a `defineTerminalPlatformElements()` helper is provided
- v1 uses the global registry by default
- the design must remain compatible with future scoped-registry support

## TypeScript Policy

Packages publish bundled `.d.ts` and use `types` plus `exports`.

Type declarations are generated from source and published with the package, not via DefinitelyTyped.

## Public API Verification

Run `npm run test:public-api` from `sdk/` before changing public SDK exports.

The gate covers:

- `@terminal-platform/workspace-elements` package entrypoint exports
- command composer action presentation contract and stable action IDs
- command composer row layout helpers and defaults
- `@terminal-platform/workspace-react` wrapper types and re-exported composer helpers
