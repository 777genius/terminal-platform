# Workspace Gateway Node Implementation Plan

**Status**: implementation plan  
**Goal**: add a reusable Node host/gateway package that lets Electron, desktop, CLI, and local web hosts embed Terminal Platform without copying demo code.

## Decision

Build a new greenfield SDK package:

- package name: `@terminal-platform/workspace-gateway-node`
- runtime: Node.js
- transport: local WebSocket server based on `ws`
- public protocol: `@terminal-platform/workspace-adapter-websocket/protocol`
- server side port: `WorkspaceRuntimeClientPort`
- first production consumer: `claude_team`
- demo usage: migrate later as a proof that demo no longer owns reusable truth

The package must not import from `apps/terminal-demo`, React, Lit, Electron, `node-pty`, or app-specific runtime code.

## Why This Package Exists

The current SDK already has:

- public runtime DTO mirrors in `@terminal-platform/runtime-types`
- transport client contracts in `@terminal-platform/workspace-contracts`
- browser/client WebSocket transport in `@terminal-platform/workspace-adapter-websocket`
- Web Component and React UI packages

What is missing is the reusable server side counterpart:

- a host can expose a local workspace gateway
- the React/Web Component UI can connect through the public WebSocket adapter
- app-specific Electron IPC remains thin and host-owned
- demo code stops being the only example of a gateway

## Non Goals

- Do not build a new terminal UI in this package.
- Do not embed `node-pty`.
- Do not define canonical DTOs.
- Do not duplicate `runtime-types`.
- Do not copy legacy `TerminalGateway*` demo protocol.
- Do not add Electron APIs to the Node gateway package.
- Do not make `claude_team` the owner of reusable terminal logic.
- Do not weaken the separation between control plane and stream plane.

## Product Invariants

- `NativeMux` remains the product truth.
- `tmux` and Zellij are foreign backends.
- Public contracts must not leak backend-native refs.
- The gateway must expose only public workspace protocol messages.
- Control plane and data plane must stay separate.
- Degraded semantics must be explicit.
- Host applications own policy, not canonical terminal behavior.

## Package Boundary

### Allowed Dependencies

Runtime dependencies:

- `ws`
- `@terminal-platform/runtime-types`
- `@terminal-platform/workspace-contracts`
- `@terminal-platform/workspace-adapter-websocket`

Dev dependencies can use existing SDK test tools:

- `vitest`
- `typescript`
- `@types/ws`

### Forbidden Dependencies

- `@terminal-platform/workspace-react`
- `@terminal-platform/workspace-elements`
- `apps/terminal-demo/*`
- Electron
- `node-pty`
- generated demo package paths
- host app aliases

## Public API Draft

Main entrypoint: `@terminal-platform/workspace-gateway-node`

Exports:

- `startWorkspaceGatewayNodeServer(options)`
- `WorkspaceGatewayNodeServer`
- `WorkspaceGatewayNodeServerHandle`
- `WorkspaceRuntimeClientPort`
- `WorkspaceGatewayAuthPolicy`
- `WorkspaceGatewayLogger`
- `WorkspaceGatewayFaultInjectionPort`
- `WorkspaceGatewayNodeServerOptions`
- `WorkspaceGatewayNodeServerUrls`
- `WorkspaceGatewayCloseReason`

Optional subpaths are not needed initially. Keep the public surface small.

## Core Ports

### `WorkspaceRuntimeClientPort`

Server-side inverse of `WorkspaceTransportClient`.

Methods:

- `handshake()`
- `listSessions()`
- `listSavedSessions()`
- `discoverSessions(backend)`
- `getBackendCapabilities(backend)`
- `createSession(backend, request)`
- `importSession(route, title)`
- `getSavedSession(sessionId)`
- `listCommandHistory(sessionId, limit)`
- `getPaneHistory(sessionId, paneId, options)`
- `deleteSavedSession(sessionId)`
- `pruneSavedSessions(keepLatest)`
- `restoreSavedSession(sessionId)`
- `attachSession(sessionId)`
- `getTopologySnapshot(sessionId)`
- `getScreenSnapshot(sessionId, paneId)`
- `getScreenDelta(sessionId, paneId, fromSequence)`
- `dispatchMuxCommand(sessionId, command)`
- `openSubscription(sessionId, spec)`
- `close()`

This should be assignable from an existing `WorkspaceTransportClient` where practical, but named separately because it is a host/runtime port.

### `WorkspaceGatewayAuthPolicy`

Default:

- bind to `127.0.0.1`
- generate an opaque token
- require token query parameter on both planes
- reject unauthorized sockets with close code `1008`

Future extension:

- custom token generator
- fixed token for tests
- allowlist hosts
- Electron-only signed bootstrap payload

### `WorkspaceGatewayLogger`

Minimal structured callbacks:

- `debug(message, context?)`
- `info(message, context?)`
- `warn(message, context?)`
- `error(message, context?)`

Default logger should be silent.

### `WorkspaceGatewayFaultInjectionPort`

Test-only hooks:

- `beforeControlRequest?(request)`
- `beforeSubscriptionOpen?(request)`
- `beforeSubscriptionClose?(request)`
- `beforeServerSend?(message)`

Must not be required by production hosts.

## Internal Components

### `WorkspaceGatewayNodeServer`

Responsibilities:

- own `WebSocketServer`
- expose generated control and stream URLs
- authenticate incoming sockets
- route sockets by path
- track control and stream connections
- close all sockets on dispose
- settle all subscriptions on dispose

It must not know about Electron, UI, or daemon startup.

### `WorkspaceGatewayRequestDispatcher`

Responsibilities:

- parse `WorkspaceGatewayControlClientMessage`
- validate method and payload shape
- call `WorkspaceRuntimeClientPort`
- serialize success and error envelopes
- preserve public error codes when available
- normalize unknown errors into safe gateway envelopes

It must not own sockets.

### `WorkspaceSubscriptionPump`

Responsibilities:

- open a runtime subscription
- send `workspace_subscription_ack`
- pump `nextEvent()` results to stream plane
- send `workspace_subscription_event`
- send `workspace_subscription_error`
- send `workspace_subscription_closed`
- close runtime subscription on unsubscribe or gateway dispose
- suppress late events after unsubscribe
- bound close with timeout so hung runtime close cannot hang the gateway

### `WorkspaceGatewayMessageCodec`

Responsibilities:

- reuse `encodeWorkspaceWebSocketPayload`
- reuse `decodeWorkspaceWebSocketPayload`
- preserve BigInt-safe encoding behavior
- reject malformed JSON before app ports

### `WorkspaceGatewayErrorMapper`

Responsibilities:

- convert unknown thrown values to `WorkspaceGatewayErrorEnvelope`
- preserve `WorkspaceError.code` when available
- avoid leaking backend-native refs in error codes
- keep message useful for host diagnostics

## Control Plane Handling

Route: `/workspace/control`

Required behavior:

- only accepts `type: "request"`
- request IDs are opaque strings
- response has the same request ID and method
- one bad request must not crash the server
- malformed JSON gets an error response where possible
- invalid methods get an error response
- invalid payload gets an error response before runtime call

Control methods must match `WorkspaceGatewayControlRequestMap` exactly.

## Stream Plane Handling

Route: `/workspace/stream`

Required behavior:

- only accepts `workspace_subscribe` and `workspace_unsubscribe`
- each stream socket owns its subscriptions
- same subscription ID cannot be active twice on one socket
- unsubscribe of unknown subscription should be idempotent and return closed
- closing socket closes all subscriptions
- gateway dispose closes all subscriptions
- runtime `nextEvent()` returning null closes the subscription
- send failure closes the socket and subscription

## Host Lifecycle Layer

The first package should include only generic gateway lifecycle:

- `startWorkspaceGatewayNodeServer({ runtime })`
- returns `{ controlUrl, streamUrl, dispose }`

Do not include daemon supervision in v1 of this package unless it is behind a separate port:

- `WorkspaceRuntimeSupervisorPort`
- `ensureRunning()`
- `dispose()`

Reason: daemon startup is packaging and host policy. `claude_team`, demo, and future hosts may stage binaries differently.

## `claude_team` Integration Shape

After the SDK package lands, `claude_team` should add a feature:

`src/features/terminal-workspace/`

Suggested folders:

- `contracts`
- `core/application`
- `main/adapters`
- `main/composition`
- `preload`
- `renderer/hooks`
- `renderer/ui`

Responsibilities:

- main starts the terminal runtime host/gateway
- preload exposes only URLs, status, and lifecycle commands
- renderer creates `WorkspaceKernel` with `createWorkspaceWebSocketTransport`
- renderer uses `@terminal-platform/workspace-react`
- UI owns layout and product placement, not terminal truth

The existing `EmbeddedTerminal` can remain as legacy for auth/login dialogs until migrated.

## UI Product Plan For `claude_team`

The target UI should feel like an operational desktop tool, not a demo page.

Primary zones:

- left team/member navigator
- center active terminal workspace
- top compact status strip
- terminal tab strip per selected team/member
- command dock at bottom
- right collapsible inspector with session details, panes, diagnostics, saved sessions

Expected UX:

- each team/member can have a terminal workspace context
- terminals can have multiple tabs
- command history persists through SDK history APIs
- saved sessions are visible and restorable
- reconnect state is explicit
- degraded backend behavior is visible
- user can retry gateway/runtime startup
- terminal does not steal focus unexpectedly outside terminal mode

Visual principles:

- dense but readable
- restrained color
- no marketing hero
- no decorative cards inside cards
- all terminal controls are icon-first where possible
- diagnostics are clear but not noisy
- keyboard and focus states are visible

## Edge Cases To Cover

### Gateway lifecycle

- server fails to bind
- port is already in use
- runtime client fails handshake
- runtime dies after gateway starts
- dispose called twice
- dispose while requests are in flight
- dispose while subscription close hangs
- socket closes while runtime request is in flight

### Control plane

- malformed JSON
- missing `type`
- wrong `type`
- unknown method
- duplicate request IDs
- payload with wrong primitive types
- payload with missing required IDs
- unsupported backend
- create session for foreign backend
- BigInt fields in responses

### Stream plane

- subscribe malformed spec
- duplicate subscription ID
- unsubscribe unknown subscription ID
- runtime subscription open rejects
- runtime subscription emits error
- runtime subscription returns null
- runtime subscription close hangs
- stream socket send throws
- client disconnects during ack
- client disconnects during event pump
- late event after unsubscribe
- late rejection after unsubscribe

### Security

- non-localhost exposure disabled by default
- token required on both planes
- no raw backend-native refs in public error codes
- no filesystem paths in protocol unless runtime DTO explicitly includes them
- no Electron IPC exposed by gateway package

### `claude_team`

- app reload leaves no orphan gateway
- window close disposes host
- multiple windows do not share unsafe subscriptions
- renderer crash does not kill unrelated runtime if policy says keep alive
- packaged app can locate runtime binaries
- dev `CLAUDE_DEV_RUNTIME_ROOT` path still works
- UI handles unavailable runtime gracefully

## Testing Plan

### SDK package tests

- public entrypoint test
- packed consumer smoke update
- control request conformance
- stream subscription conformance
- auth policy test
- malformed payload tests
- race tests copied as scenarios from demo, not copied as production code
- dispose tests
- logger/fault injection tests

### Demo migration tests

- demo imports new package only
- no demo-owned workspace gateway protocol
- demo still launches against local gateway
- old legacy gateway protocol is removed or isolated

### `claude_team` tests

- main composition unit tests
- preload API shape tests
- renderer hook lifecycle tests
- UI smoke for terminal workspace panel
- dev app smoke with `pnpm dev:mcp`

## Implementation Phases

### Phase 1 - Plan and package skeleton

- add package to `sdk/package.json`
- add `packages/workspace-gateway-node/package.json`
- add tsconfig
- add public entrypoint
- document package in API map
- add packed consumer smoke expectations

### Phase 2 - Ports and message model

- define `WorkspaceRuntimeClientPort`
- define gateway server options
- define gateway URLs and handle types
- define logger, auth, close reason, fault injection types

### Phase 3 - Control dispatcher

- implement request validation
- implement method dispatch
- implement error mapping
- test all methods with fake runtime port

### Phase 4 - Stream pump

- implement subscription records
- implement ack/event/error/closed
- implement close timeout
- implement late event suppression
- implement send failure handling

### Phase 5 - Node WebSocket server

- bind host/port
- generate token
- expose URLs
- authenticate connections
- route control and stream sockets
- dispose all resources

### Phase 6 - SDK hardening

- public API tests
- packed consumer smoke
- `npm run check`
- docs update

### Phase 7 - Demo migration

- replace demo-owned workspace gateway pieces with new package
- remove or isolate legacy-only protocol
- keep demo as pure consumer

### Phase 8 - `claude_team` integration

- add `terminal-workspace` feature boundary
- add main composition
- add preload bridge
- add renderer shell and navigation entry
- use React SDK UI
- preserve existing `EmbeddedTerminal` until auth modal migration

### Phase 9 - UX polish

- team/member terminal context
- tab management
- persisted command history
- saved sessions panel
- connection status and retry
- diagnostics surface
- keyboard/focus states

### Phase 10 - Full verification

- SDK checks
- `claude_team` typecheck/tests
- run `CLAUDE_DEV_RUNTIME_ROOT=/Users/belief/dev/projects/claude/agent_teams_orchestrator pnpm dev:mcp`
- interact with the UI
- verify terminal launch, tabs, commands, history, saved sessions, and cleanup

## Traps To Avoid

- Do not copy `TerminalRuntimeGatewayServer` wholesale from demo.
- Do not preserve legacy `TerminalGateway*` in the new package.
- Do not let `claude_team` import from SDK internals or demo paths.
- Do not put runtime daemon startup inside the generic gateway server class.
- Do not expose raw `WebSocketServer` as the main public API.
- Do not use global mutable singleton gateway state.
- Do not let one hanging subscription block gateway dispose.
- Do not treat green typecheck as proof of browser/Electron runtime behavior.
- Do not build UI as a marketing page or demo shell.
- Do not use low-level `node-pty` API as the long-term platform abstraction.

## Acceptance Criteria

The package is acceptable when:

- it builds as a normal SDK workspace package
- it exports only documented APIs
- it passes packed consumer smoke
- it can serve all current `WorkspaceGatewayControlRequestMap` methods
- it can serve workspace stream subscriptions
- it has conformance tests for control and stream planes
- it preserves control/data plane separation
- it does not import demo, React, Lit, Electron, or `node-pty`
- `claude_team` can consume it through a thin adapter
- demo can be migrated away from demo-owned workspace gateway logic

