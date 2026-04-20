# V1 Implementation Roadmap And Task Breakdown

**Проверено**: 2026-04-19
**Статус**: execution roadmap

## How to read this plan

Это не research note.

Это operational roadmap, по которому можно заводить задачи и делать implementation.

Bootstrap details for the very first phase live in:

- [v1-workspace-bootstrap-spec.md](./v1-workspace-bootstrap-spec.md)

Каждый этап содержит:

- цель
- что делаем
- что не делаем
- какие артефакты должны появиться
- exit criteria

## Milestone 0 - Bootstrap

### Goal

Сделать пустой, но дисциплинированный workspace.

### Scope

- создать Rust workspace
- зафиксировать toolchain
- зафиксировать base lint/test tooling
- создать пустые crates

### Deliverables

- workspace root
- `Cargo.toml`
- `rust-toolchain.toml`
- `clippy`, `fmt`, `nextest` baseline
- base docs in crate READMEs if нужны

### Not in scope

- real PTY
- real daemon
- real host integration

### Exit criteria

- workspace compiles
- empty crates compile
- local test/lint pass is green

## Milestone 1 - Contract Freeze

### Goal

Заморозить public semantics до начала большого implementation.

### Scope

- identity types
- backend kind and route types
- capability model
- mux command model
- topology DTO
- screen DTO
- handshake DTO
- error/degraded envelopes

### Deliverables

- `terminal-domain`
- `terminal-backend-api`
- `terminal-protocol`

### Exit criteria

- all contract crates compile
- zero backend-specific refs in public DTOs
- DTO naming no longer churns every day

## Milestone 2 - Daemon Skeleton

### Goal

Сделать minimal daemon boundary, чтобы host and runtime already speak through real transport.

### Scope

- local transport
- handshake
- request/reply framing
- subscription lifecycle skeleton
- peer/auth policy hook

### Deliverables

- `terminal-daemon`
- `terminal-daemon-client`
- smoke example client

### Not in scope

- real session logic
- real streaming

### Exit criteria

- client connects
- receives handshake
- can call `list_sessions`
- can open/close dummy subscription

## Milestone 3 - Native Session Registry

### Goal

Сделать первую настоящую session truth.

### Scope

- session registry
- create/list/attach native session
- canonical IDs
- route registry

### Deliverables

- `terminal-application`
- initial `terminal-persistence`
- session lifecycle service

### Exit criteria

- native sessions persist in structured store
- attach works through daemon boundary
- route mappings exist

## Milestone 4 - Native PTY And Emulator

### Goal

Сделать реальный Native terminal.

### Scope

- PTY start/stop/resize
- emulator integration
- input path
- first screen snapshot

### Dependencies

- `portable-pty`
- `alacritty_terminal`

### Deliverables

- `terminal-backend-native`
- `ScreenSnapshot`
- `ProjectionSource::NativeEmulator`

### Exit criteria

- can run shell in native session
- can send text/keys
- can resize
- can fetch current screen snapshot

## Milestone 5 - Native Topology And Commands

### Goal

Сделать минимально useful mux layer for native runtime.

### Scope

- tabs
- split tree
- focus
- close/open pane
- rename tab

### Deliverables

- `TopologySnapshot`
- `MuxCommand` handlers for native backend

### Exit criteria

- host can create tabs
- host can split pane
- host can switch focus
- topology snapshot matches runtime state

## Milestone 6 - Projection Proof

### Goal

Доказать, что public contract не дрейфует и уже пригоден как truth for host.

### Scope

- golden snapshots
- projection source tests
- degraded-mode tests
- protocol golden tests
- attach/detach sequencing tests

### Deliverables

- `terminal-testing`
- snapshot corpus
- protocol fixtures

### Exit criteria

- golden tests stable
- protocol compatibility tests stable
- regressions can be reproduced from fixtures

## Milestone 7 - Node/Electron First Consumer

### Goal

Подключить первый реальный host.

### Scope

- `terminal-node`
- `terminal-node-napi`
- TS DTO export
- Electron integration seam
- one working embedded terminal consumer path

### Deliverables

- safe host facade + `napi-rs` leaf wrapper
- staged JS package + TS types
- integration notes for repo feature slice

### Exit criteria

- Electron app can connect to daemon
- can list/create/attach native sessions
- can render topology and screen snapshots

## Milestone 8 - `tmux` Adapter

### Goal

Сделать conservative imported `tmux` route.

### Scope

- discover/list/import tmux sessions
- attach to one tmux route
- import raw output
- import screen snapshots via `capture-pane`
- route-local tab mapping `(session, window)`
- explicit resize authority policy

### Not in scope

- full parity
- magic independent per-client focus
- full topology event fidelity

### Exit criteria

- imported tmux session appears in canonical session registry
- host can observe topology
- host can fetch screen snapshot
- host can issue basic control actions where representable
- degraded-mode reasons are explicit for unsupported actions

## Milestone 9 - `Zellij` Adapter

### Goal

Сделать conservative imported `Zellij` route.

### Scope

- discover/list/import zellij sessions
- JSON topology import
- subscribe viewport updates
- dump snapshot import
- typed pane refs
- one ordered mutation lane

### Not in scope

- plugin parity with native model
- floating pane parity promises
- total semantic unification

### Exit criteria

- imported Zellij session appears in canonical session registry
- host can observe topology and rendered updates
- host can run basic actions
- typed pane refs preserved internally
- `tab_id` and `position` are not conflated

## Milestone 10 - Hardening

### Goal

Не добавлять features, а закрепить reliability.

### Scope

- fuzzing
- race checks
- shutdown/order tests
- backpressure tests
- corruption/recovery tests
- stale daemon recovery tests

### Exit criteria

- CI matrix stable
- core golden tests stable
- adapter smoke suites stable

## Suggested task hierarchy

### Track A - Contracts

- define DTOs
- define error model
- define handshake
- define capability matrix

### Track B - Daemon

- transport
- auth
- request routing
- subscriptions

### Track C - Native runtime

- PTY lifecycle
- emulator bridge
- topology
- commands
- projections

### Track D - Host integration

- Node leaf
- TS type generation
- Electron bridge

### Track E - Foreign adapters

- tmux
- Zellij

### Track F - Verification

- golden fixtures
- protocol fixtures
- smoke suites
- manual QA capture and regression conversion

## Sprint 0 ticket starter pack

Ниже не abstract milestone text, а прямой стартовый пакет задач.

### T0.1 - Create workspace root

Scope:

- add root `Cargo.toml`
- add `rust-toolchain.toml`
- add root config files from bootstrap spec

Done when:

- workspace resolves
- empty command `cargo metadata` works

### T0.2 - Scaffold all agreed crates

Scope:

- create all crates from bootstrap spec
- add minimal `Cargo.toml`
- add `src/lib.rs`

Done when:

- `cargo check --workspace` passes with empty crates

### T0.3 - Freeze base domain types

Scope:

- `SessionId`
- `TabId`
- `PaneId`
- `BackendKind`
- `SessionRoute`
- `DegradedModeReason`

Done when:

- `terminal-domain` compiles
- no backend-specific refs appear publicly

### T0.4 - Freeze mux and projection DTOs

Scope:

- `TopologySnapshot`
- `TabSnapshot`
- `PaneSnapshot`
- `ScreenSnapshot`
- `ProjectionSource`

Done when:

- `terminal-mux-domain` and `terminal-projection` compile
- snapshot fixtures can already be sketched

### T0.5 - Freeze backend API and command model

Scope:

- `BackendCapabilities`
- `MuxBackendPort`
- `BackendSessionPort`
- `MuxCommand`
- `BackendError`

Done when:

- `terminal-backend-api` compiles
- command vocabulary stops drifting

### T0.6 - Freeze daemon protocol envelope

Scope:

- handshake DTOs
- request envelope
- reply envelope
- subscription open/close DTOs
- degraded/error envelope

Done when:

- `terminal-protocol` compiles
- first protocol fixture files can be written

### T0.7 - Create daemon skeleton

Scope:

- local server bootstrap
- handshake path
- dummy `list_sessions`
- dummy subscription lifecycle

Done when:

- simple smoke client connects and receives handshake

### T0.8 - Create in-memory registry stub

Scope:

- session registry trait
- in-memory implementation
- empty result for `list_sessions`

Done when:

- daemon can answer one real request path through application service

### T0.9 - Verification scaffold

Scope:

- nextest config active
- first snapshot test crate path
- first protocol fixture folder

Done when:

- CI-equivalent local commands run successfully

## Recommended execution order for the first two sprints

### Sprint 1

- T0.1
- T0.2
- T0.3
- T0.4
- T0.5
- T0.6

### Sprint 2

- T0.7
- T0.8
- T0.9
- first Node/Electron connection spike against dummy daemon

## Stop conditions

При старте implementation надо останавливаться и фиксировать решение в docs, если:

- public DTO wants backend-specific ids
- daemon API wants to leak PTY/emulator internals
- host leaf asks for direct `tmux` or `Zellij` control
- concrete backend implementation forces contract churn

Если один из этих сигналов появился, это не "мелкая деталь", а architectural drift.

### Track F - Verification

- property tests
- snapshot tests
- fuzz
- interaction harness

## First sprint task list

### Sprint 1 must produce

- workspace scaffold
- contract crates
- daemon handshake
- `list_sessions` happy path
- docs frozen for v1 semantics

### Sprint 1 must not expand into

- UI polish
- foreign backends
- search/timeline extras
- SSH/remote

## Second sprint task list

- native session registry
- PTY bootstrap
- first native snapshot
- protocol fixture tests

## Third sprint task list

- topology
- native commands
- host client
- first repo integration smoke path

## Work partitioning rule

Если работа идёт параллельно, safest split такой:

- worker 1: contracts/protocol
- worker 2: daemon transport/composition
- worker 3: native backend
- worker 4 later: Node/Electron host

`tmux` and `Zellij` should not begin before the native contract and projection tests exist.

## Definition of done for v1 start phase

Старт считается реально подготовленным, когда:

- frozen docs exist
- workspace scaffold exists
- contract crates compile
- daemon handshake works
- native empty session lifecycle works

До этого всё ещё phase `pre-implementation`.
