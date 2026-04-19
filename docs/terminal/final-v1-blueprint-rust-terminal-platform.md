# Final V1 Blueprint: Rust Terminal Platform

**Проверено**: 2026-04-19
**Статус**: canonical v1 blueprint

## Executive decision

Для v1 фиксируем один основной путь:

1. **Canonical terminal platform with one daemon/runtime truth**
   `🎯 10   🛡️ 9   🧠 8`
   Примерно `14000-26000` строк до сильного v1.

2. **Native-only terminal runtime without foreign backends in v1**
   `🎯 7   🛡️ 10   🧠 6`
   Примерно `9000-16000` строк.

3. **Immediate full-parity Native + tmux + Zellij**
   `🎯 4   🛡️ 5   🧠 10`
   Примерно `22000-40000+` строк.

✅ Берём **вариант 1**, но с очень важным ограничением:

- `NativeMux` is the reference truth
- `tmux` and `Zellij` enter v1 only in conservative `observe/control` scope
- no false promise of parity

## Companion implementation pack

Этот blueprint теперь надо читать не в одиночку, а вместе с operational docs:

- [start-here-v1-implementation-pack.md](./start-here-v1-implementation-pack.md)
- [v1-workspace-bootstrap-spec.md](./v1-workspace-bootstrap-spec.md)
- [v1-implementation-roadmap-and-task-breakdown.md](./v1-implementation-roadmap-and-task-breakdown.md)
- [v1-verification-and-acceptance-plan.md](./v1-verification-and-acceptance-plan.md)

## North star

Мы строим не просто terminal widget и не просто mux.

Мы строим **universal embeddable terminal platform**:

- Rust runtime truth
- host-neutral protocol
- JS UI as first consumer
- future C/Node/other hosts as additional consumers
- optional standalone app later

## V1 product boundary

V1 обязан уметь:

- local daemon/runtime
- Native terminal sessions
- screen snapshots and deltas
- tabs and split tree in canonical model
- session persistence for native runtime
- `tmux` imported session routes
- `Zellij` imported session routes
- typed control plane for Electron/Node host

V1 не обязан уметь:

- full feature parity across all backends
- remote SSH as first-class stable route
- plugin platform
- media protocols as a core feature
- complete standalone GUI app

## Canonical architecture

```text
Host UI / Electron / Node / future C hosts
        |
        v
typed host SDK / protocol client
        |
        v
terminal-daemon
  - composition root
  - auth / capability handshake
  - session registry
  - route registry
  - subscriptions
        |
        v
terminal-runtime
  - canonical IDs
  - mux application services
  - projection services
  - persistence services
  - capability negotiation
        |
        +--> NativeMuxBackend
        +--> TmuxAdapter
        +--> ZellijAdapter
```

## Canonical truths

### Truth we own

- canonical `SessionId`, `TabId`, `PaneId`
- canonical topology DTOs
- canonical host protocol
- canonical capability vocabulary
- canonical degraded-mode reasons
- canonical projection format

### Truth we import

- tmux pane output and session/window topology
- Zellij rendered viewport and tab/pane state

### Truth we never leak

- raw tmux `%pane`, `@window`, socket names
- raw Zellij `terminal_N`, `plugin_N`, `tab_id` as public identity
- internal PTY handles
- emulator internals

## Crate graph

### Public crates

```text
terminal-sdk
terminal-protocol
terminal-daemon-client
terminal-node
terminal-capi
```

### Internal workspace crates

```text
terminal-domain
terminal-mux-domain
terminal-application
terminal-backend-api
terminal-backend-native
terminal-backend-tmux
terminal-backend-zellij
terminal-projection
terminal-persistence
terminal-daemon
terminal-testing
```

## Crate responsibilities

### `terminal-domain`

Pure domain types:

- `SessionId`, `TabId`, `PaneId`
- `BackendKind`
- `SessionRoute`
- `PaneTreeNode`
- `SplitDirection`
- `DegradedModeReason`
- `ProjectionSource`

No IO, no Tokio, no subprocesses.

### `terminal-mux-domain`

Mux-specific concepts:

- `WorkspaceSession`
- `TabSnapshot`
- `PaneSnapshot`
- route-local focus concepts
- imported foreign pane categories

### `terminal-backend-api`

Ports and backend DTOs:

- `MuxBackendPort`
- `BackendSessionPort`
- `BackendCapabilities`
- `MuxCommand`
- `BackendError`
- `SubscriptionSpec`

### `terminal-application`

Use cases:

- `CreateSession`
- `AttachSession`
- `ListSessions`
- `DispatchMuxCommand`
- `GetTopologySnapshot`
- `GetScreenSnapshot`
- `SubscribeSession`

### `terminal-projection`

Projection builders:

- `ScreenSnapshot`
- `ScreenDelta`
- `TopologySnapshot`
- `SessionHealthSnapshot`
- projection source handling

### `terminal-persistence`

Durable structured truth:

- SQLite schemas
- migrations
- route registry
- canonical ID mappings
- rebuildable read models

### `terminal-backend-native`

Reference runtime:

- PTY lifecycle
- emulator integration
- native mux topology
- native subscriptions
- native snapshots/deltas

### `terminal-backend-tmux`

Foreign adapter:

- control-mode client
- `capture-pane` importer
- format query importer
- route-local window bindings
- explicit resize authority policy

### `terminal-backend-zellij`

Foreign adapter:

- subprocess action lane
- subscribe workers
- JSON topology import
- typed pane refs
- `tab_id` vs `position` handling

### `terminal-daemon`

Composition root:

- local socket server
- handshake
- auth/peer verification
- subscription fanout
- operation routing

## Canonical traits

```rust
pub trait MuxBackendPort: Send + Sync {
    fn kind(&self) -> BackendKind;
    async fn capabilities(&self) -> Result<BackendCapabilities, BackendError>;
    async fn create_session(&self, spec: CreateSessionSpec) -> Result<BackendSessionBinding, BackendError>;
    async fn attach_session(&self, route: BackendRoute) -> Result<Box<dyn BackendSessionPort>, BackendError>;
    async fn list_sessions(&self, scope: BackendScope) -> Result<Vec<BackendSessionSummary>, BackendError>;
}

pub trait BackendSessionPort: Send + Sync {
    async fn topology_snapshot(&self) -> Result<TopologySnapshot, BackendError>;
    async fn screen_snapshot(&self, pane: PaneId, spec: ScreenSnapshotSpec) -> Result<ScreenSnapshot, BackendError>;
    async fn dispatch(&self, command: MuxCommand) -> Result<MuxCommandResult, BackendError>;
    async fn subscribe(&self, spec: SubscriptionSpec) -> Result<BackendSubscription, BackendError>;
}
```

## Canonical DTOs to freeze in v1

### Identity

```rust
pub struct SessionId(Uuid);
pub struct TabId(Uuid);
pub struct PaneId(Uuid);
pub struct OperationId(Uuid);
pub struct SubscriptionId(Uuid);
```

### Routing

```rust
pub enum BackendKind {
    Native,
    Tmux,
    Zellij,
}

pub struct SessionRoute {
    pub backend: BackendKind,
    pub authority: RouteAuthority,
    pub external: Option<ExternalSessionRef>,
}
```

### Backend-specific refs

```rust
pub struct TmuxTabBinding {
    pub session: TmuxSessionRef,
    pub window: TmuxWindowRef,
}

pub enum ZellijPaneKind {
    Terminal,
    Plugin,
}

pub struct ZellijPaneRef {
    pub kind: ZellijPaneKind,
    pub id: u32,
}
```

### Topology

```rust
pub struct TopologySnapshot {
    pub session_id: SessionId,
    pub backend_kind: BackendKind,
    pub tabs: Vec<TabSnapshot>,
    pub focused_tab: Option<TabId>,
}
```

### Screen

```rust
pub struct ScreenSnapshot {
    pub pane_id: PaneId,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: ProjectionSource,
    pub surface: ScreenSurface,
}
```

## Capability model

```rust
pub struct BackendCapabilities {
    pub tiled_panes: bool,
    pub floating_panes: bool,
    pub split_resize: bool,
    pub session_scoped_tab_refs: bool,
    pub session_scoped_pane_refs: bool,
    pub raw_output_stream: bool,
    pub rendered_viewport_stream: bool,
    pub rendered_viewport_snapshot: bool,
    pub rendered_scrollback_snapshot: bool,
    pub layout_dump: bool,
    pub layout_override: bool,
    pub read_only_client_mode: bool,
    pub explicit_session_save: bool,
    pub plugin_panes: bool,
    pub advisory_metadata_subscriptions: bool,
    pub independent_resize_authority: bool,
}
```

### Honest v1 semantics

- `Native`
  - strongest superset
- `tmux`
  - strong on raw output and attach/detach
  - route-local tabs
  - limited resize authority policy
- `Zellij`
  - strong on JSON topology and rendered viewport subscriptions
  - typed pane refs
  - plugin-aware import

## Command model

```rust
pub enum MuxCommand {
    SplitPane(SplitPaneSpec),
    ClosePane { pane_id: PaneId },
    FocusPane { pane_id: PaneId },
    ResizePane(ResizePaneSpec),
    NewTab(NewTabSpec),
    CloseTab { tab_id: TabId },
    FocusTab { tab_id: TabId },
    RenameTab { tab_id: TabId, title: String },
    SendInput(SendInputSpec),
    SendPaste(SendPasteSpec),
    Detach,
    SaveSession,
    OverrideLayout(OverrideLayoutSpec),
}
```

## Transport model

### Control plane

- framed local protocol
- operation/reply envelopes
- subscription open/close
- capability handshake
- errors and degraded reasons

### Data plane

- screen snapshots
- deltas
- imported stream chunks

### Why

- control semantics and terminal data are different contracts
- this keeps future Node/C hosts honest

## Recommended dependencies

### Core runtime

- `tokio`
- `tokio-util`
- `bytes`
- `tracing`
- `interprocess`

### Native backend

- `portable-pty`
- `alacritty_terminal`

### Persistence

- `rusqlite`
- `rusqlite_migration`

### Host leaves

- `napi-rs`
- `safer-ffi`
- `cbindgen`
- `cargo-c`

### Verification

- `proptest`
- `insta`
- `cargo-fuzz`
- `nextest`
- `expectrl`

## Daemon handshake

Host must get:

- `protocol_version`
- `binary_version`
- `daemon_phase`
- `capabilities`
- `available_backends`
- `session_scope`

No host should infer readiness from socket reachability alone.

## Persistence design

SQLite stores:

- canonical sessions
- routes
- backend mappings
- projection metadata
- subscription metadata if needed

Not stored as truth:

- giant serialized emulator blobs as the only source of truth
- backend raw IDs as canonical identity

## V1 implementation order

### Stage 1 - Contract freeze

- `terminal-domain`
- `terminal-backend-api`
- `terminal-protocol`
- canonical DTOs

### Stage 2 - Native reference

- `terminal-backend-native`
- local daemon
- host handshake
- topology and screen snapshot APIs

### Stage 3 - Projection proof

- snapshot tests
- degraded-mode tests
- projection-source tests
- protocol golden tests

### Stage 4 - `tmux` import

Scope:

- list/attach/import
- raw output import
- screen snapshot import
- route-local tab policy
- explicit resize policy

Not in scope:

- full native parity
- per-client independent focus parity unless grouped-session policy is added

### Stage 5 - `Zellij` import

Scope:

- list/attach/import
- JSON topology import
- rendered viewport subscribe
- rendered snapshot import
- typed pane-ref import
- ordered mutation lane

Not in scope:

- plugin parity with native host surfaces
- floating/layout parity promises

### Stage 6 - Node/Electron first-class host

- `terminal-node`
- TS DTO generation
- Electron reference integration

## What we promise in v1

- one canonical API
- one canonical daemon
- one canonical capability model
- strong native backend
- conservative `tmux` and `Zellij` support
- explicit degraded semantics

## What we do not promise in v1

- full backend parity
- identical focus semantics
- identical resize semantics
- identical layout semantics
- identical plugin/floating semantics

## Practical v1 recommendation

Если собирать старт implementation прямо сейчас, strongest order такой:

1. `terminal-domain`
2. `terminal-backend-api`
3. `terminal-protocol`
4. `terminal-backend-native`
5. `terminal-daemon`
6. `terminal-node`
7. `terminal-backend-tmux`
8. `terminal-backend-zellij`
9. `terminal-testing`

🔥 Самое важное правило этого blueprint:

**`NativeMux` defines the product. `tmux` and `Zellij` extend the product. They do not define it.**
