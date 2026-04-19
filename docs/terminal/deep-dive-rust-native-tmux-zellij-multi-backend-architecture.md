# Deep Dive: Rust Native + tmux + Zellij Multi-Backend Architecture

**Проверено**: 2026-04-19

## Context

Новая жёсткая цель:

- делать **свой Rust terminal platform**
- иметь **свой canonical runtime contract**
- но при этом уметь поддерживать одновременно:
  - `NativeMux`
  - `tmux`
  - `Zellij`

Ключевой вопрос здесь уже не "можно ли вызвать tmux или zellij".

Ключевой вопрос такой:

**как спроектировать архитектуру так, чтобы `tmux` и `zellij` усиливали продукт, а не становились хозяевами его модели**

## Short answer

🔥 Да, поддерживать `tmux` и `zellij` одновременно стоит.

Но healthiest path only one:

1. **Canonical mux contract + NativeMux reference implementation + `TmuxAdapter` + `ZellijAdapter`**
   `🎯 10   🛡️ 9   🧠 8`
   Примерно `9000-18000` строк до сильного первого production слоя.

2. **NativeMux as the only full truth, `tmux` and `zellij` only as attachable external providers**
   `🎯 8   🛡️ 10   🧠 6`
   Примерно `6000-12000` строк.

3. **`tmux` and `zellij` as co-equal product truths**
   `🎯 3   🛡️ 4   🧠 10`
   Примерно `14000-26000` строк.

✅ Я бы фиксировал **вариант 1**.

Это единственный путь, где:

- ваш runtime остаётся главным продуктом
- `tmux` и `zellij` реально поддерживаются
- архитектура не превращается в "lowest common denominator wrapper"

## Official seam check

### `tmux`

По текущей official surface на `2026-04-19`:

- GitHub Releases page показывает latest release `tmux 3.6a`
- control mode даёт:
  - `%output`
  - `%window-add`
  - `%window-close`
  - `%window-renamed`
  - `%sessions-changed`
  - `%session-window-changed`
  - `%subscription-changed`
- `refresh-client -B` даёт format subscriptions
- `refresh-client -f pause-after=...` даёт flow control
- `capture-pane` остаётся важным snapshot/resync tool
- one control-mode client is attached to one session at a time
- tmux output in control mode is raw pane output, not a complete "render model"
- tmux itself explicitly notes that copy/choose mode output is not sent to control clients

Что это значит архитектурно:

- `tmux` хорошо годится как **foreign mux backend**
- но он не должен быть source of truth для вашего public render contract

### `Zellij`

По текущей official surface на `2026-04-19`:

- GitHub Releases page показывает latest release `v0.44.1`
- `zellij action` уже даёт сильный automation/control surface:
  - `list-panes`
  - `list-tabs`
  - `current-tab-info`
  - `new-pane`
  - `new-tab`
  - `send-keys`
  - `dump-screen`
  - `dump-layout`
  - `focus-pane-id`
  - `override-layout`
  - `save-session`
  - `switch-session`
- `zellij subscribe` даёт live pane streaming:
  - raw text viewport mode
  - JSON NDJSON mode
  - initial viewport delivery
  - optional scrollback
  - pane closed events
- official docs now explicitly position `action + subscribe` as the programmatic control surface
- current release also adds more CLI control seams like `focus-pane-with-id`, `--tab-id`, layout override improvements and more

Что это значит архитектурно:

- `zellij` тоже очень хорошо годится как **foreign mux backend**
- причём его external control surface богаче и ближе к machine-control shape, чем у `tmux`
- но его plugin/floating/layout worldview тоже нельзя делать вашим product truth

## The most important new rule

🔥 **Нельзя делать `tmux` и `zellij` хозяевами вашей доменной модели.**

Правильный порядок власти должен быть такой:

1. your canonical domain
2. your canonical protocol and DTOs
3. native runtime implementation
4. foreign backend adapters
5. host UI adapters

Не наоборот.

## Review corrections

После дополнительной проверки official seams у этого плана есть несколько важных уточнений.

Их лучше зафиксировать сразу, чтобы не заложить ложные promises в v1.

### 1. `tmux` и `Zellij` дают не один и тот же тип live stream

Это самый важный architectural correction.

- `tmux` control mode даёт прежде всего **raw pane output stream**
- `Zellij subscribe` даёт **rendered viewport stream**

Это не одна и та же truth.

Следствие:

- imported `tmux` sessions нельзя моделировать как будто они дают тот же stream contract, что `Zellij subscribe`
- imported `Zellij` sessions нельзя моделировать как будто они дают raw PTY transcript
- semantic analyzers, replay truth and transcript truth должны знать источник данных явно

🔥 Значит capability model должна различать:

- raw byte-ish output access
- rendered viewport access
- rendered snapshot access

### 2. backend IDs не надо называть просто "stable"

`tmux` pane/window IDs и `Zellij` pane/tab refs полезны для machine control, но слово `stable` слишком легко читается как "durable across restart".

Более честная формулировка:

- `session-scoped refs`
- `backend-scoped refs`

То есть:

- tmux `%pane` and `@window` are stable for the life of the server
- Zellij pane/tab refs are stable enough for the current session/control surface
- but none of these should be treated as canonical durable product IDs

### 3. read-only attach нельзя считать security boundary

Да, официально:

- `tmux attach-session -r` / read-only client exists
- `zellij watch` and read-only session sharing exist

Но это не отменяет главного правила:

- trust and authority still belong to our daemon policy
- backend read-only modes are useful runtime capabilities, not the primary security model

### 4. `tmux window == canonical tab` only by mapping policy

Это допустимое product mapping, но не ontological truth.

Нужно явно считать это:

- `tmux window -> canonical tab` by adapter policy
- not "tmux already has the same tab semantics as our model"

### 5. `refresh-client -B` в tmux - useful, but advisory

Format subscriptions у `tmux` полезны, но:

- session-attached scoped
- not a full topology event bus
- not a substitute for snapshot/resync logic

Значит их надо проектировать как:

- advisory metadata feed
- not the sole source of topology truth

### 6. `tmux` multi-client semantics do not match per-client tab focus by default

Это ещё один большой architectural correction.

У `tmux`:

- client attached to a session shares that session's current window semantics
- if different attached terminals should see different windows, official guidance is to use grouped sessions or manually linked windows
- windows may be linked into multiple sessions

Следствие:

- `tmux` session focus нельзя напрямую считать equivalent вашему host-side per-client tab focus
- canonical `TabId` mapping for tmux must be scoped by route/session context
- in some modes `TmuxAdapter` may need dedicated grouped sessions or a shadow-session strategy to give independent host views safely

🔥 Значит canonical "tab" for tmux should really be modeled as:

- a route-local tab projection
- often backed by `(tmux session ref, tmux window ref)`
- not just by `@window`

### 7. `Zellij` control is subprocess-based, so adapter ordering must be owner-task enforced

Official Zellij programmatic control docs are explicit:

- all interaction happens through subprocess invocation
- separate `zellij action` processes have no ordering guarantee when issued concurrently

Следствие:

- `ZellijAdapter` must not expose "just call the CLI from anywhere" as architecture
- one owner task should serialize mutations per imported session
- subscriptions and queries can be concurrent, but mutation ordering should be adapter-owned

🔥 This means `ZellijAdapter` needs an internal command lane, not just a pile of spawned child processes.

### 8. `tmux` size semantics need one explicit resize authority

Это ещё один важный correction point.

Official tmux control-mode docs explicitly note:

- control-mode clients do not affect window size unless `refresh-client -C` is used
- read-only attach on current tmux lines is tied to `ignore-size` semantics as well

Следствие:

- resize policy for imported tmux sessions cannot be accidental
- `TmuxAdapter` should define one explicit resize authority per attached route
- observe-only or read-only imports should usually avoid being resize authorities by default
- host-facing "read-only" and backend-side "ignore-size" must not be collapsed into one concept

🔥 In other words:

- write authority
- resize authority
- observation authority

are three different knobs.

### 9. `Zellij` pane references are typed, not just numeric

Official CLI docs are explicit:

- terminal panes and plugin panes can have overlapping numeric IDs
- the control surface disambiguates them through typed refs like `terminal_1` and `plugin_2`
- tab state also distinguishes `tab_id` from presentation `position`

Следствие:

- `ZellijPaneRef` must preserve pane kind
- adapter mappings must never normalize these refs down to bare integers as canonical truth
- `tab_id` should be treated as backend ref, while `position` is presentation/order metadata

🔥 This means:

- pane identity import needs `(kind, numeric_id)` shape
- tab ordering import needs `tab_id + position`, not one mixed field

## The right mental model

Ваш продукт - это не "`tmux` clone с `zellij` support".

И не "`zellij` controller with extra wrappers".

Самая точная модель такая:

- у вас есть **terminal platform**
- внутри неё есть **mux bounded context**
- этот mux bounded context умеет работать через разные backend routes:
  - native
  - tmux
  - zellij

То есть нужно проектировать не "один backend".

Нужно проектировать **backend family architecture**.

## Core architecture decision

### 1. Split runtime into two backend families

Это ключевой разрез.

Нужно с самого начала различать:

- **native backends**
- **foreign mux backends**

#### Native backend

В native backend ваша система владеет:

- PTY truth
- emulator truth
- screen diff truth
- replay truth
- persistence truth
- mux topology truth

#### Foreign mux backend

В foreign backend ваша система **не владеет** исходным terminal truth.

Она владеет:

- route identity
- canonical IDs and mapping tables
- attach lifecycle
- host-facing projections
- capability negotiation
- snapshots and imported event streams
- fallback/degraded-mode policy

А backend владеет:

- actual pane/session topology
- actual screen/render truth
- actual attach semantics

🔥 Это очень важный разрез.

Потому что иначе вы попытаетесь одинаково обращаться с:

- собственной PTY/emulator truth
- и с уже существующим `tmux`/`zellij` server truth

Это архитектурно неверно.

## Canonical bounded contexts

### `MuxDomain`

Отвечает только за host-neutral concepts:

- `WorkspaceSession`
- `Tab`
- `PaneNode`
- `PanePlacement`
- `FocusState`
- `SplitTree`
- `SessionRoute`
- `BackendKind`
- `CapabilitySet`
- `DegradedModeReason`

### `BackendRoutingDomain`

Отвечает за то, **где** реально живёт session:

- `SessionRoute::Native`
- `SessionRoute::Tmux`
- `SessionRoute::Zellij`

И за backend-specific refs:

- `TmuxServerRef`
- `TmuxSessionRef`
- `TmuxWindowRef`
- `TmuxPaneRef`
- `ZellijSessionRef`
- `ZellijTabRef`
- `ZellijPaneRef`

Для tmux тут нужен ещё один very important nuance:

- `TmuxWindowRef` alone is not enough as a canonical tab binding when windows can be linked into multiple sessions
- adapter-level route bindings should preserve session context explicitly

For Zellij there is another nuance:

- pane refs should preserve terminal/plugin kind
- tab refs should preserve backend `tab_id` separately from visual `position`

### `ProjectionDomain`

Отвечает за host-facing surfaces:

- `ScreenSnapshot`
- `ScreenDelta`
- `PaneLifecycleEvent`
- `TopologySnapshot`
- `SessionHealth`
- `BackendPhase`

### `CapabilityDomain`

Отвечает за feature negotiation:

- `SplitDirection`
- `FloatingPanes`
- `LayoutTemplates`
- `LayoutOverride`
- `ReadOnlyAttach`
- `PaneScrollbackQuery`
- `LivePaneSubscribe`
- `StableTabIds`
- `Plugins`
- `SessionSave`
- `FormatSubscriptions`

### `FallbackPolicyDomain`

Отвечает за явные degraded semantics:

- `Unsupported`
- `FallbackApplied`
- `NotRepresentable`
- `RequiresNewAttach`
- `ForeignBackendRestriction`

## Canonical public traits

Ниже самый здоровый shape для Rust core.

```rust
pub trait MuxBackendPort: Send + Sync {
    fn kind(&self) -> BackendKind;
    async fn capabilities(&self) -> Result<BackendCapabilities, BackendError>;
    async fn create_session(&self, spec: CreateSessionSpec) -> Result<BackendSessionBinding, BackendError>;
    async fn attach_session(&self, route: BackendRoute) -> Result<BackendSessionHandle, BackendError>;
    async fn list_sessions(&self, scope: BackendScope) -> Result<Vec<BackendSessionSummary>, BackendError>;
}

pub trait BackendSessionPort: Send + Sync {
    async fn topology_snapshot(&self) -> Result<TopologySnapshot, BackendError>;
    async fn screen_snapshot(&self, pane: PaneId, opts: ScreenSnapshotSpec) -> Result<ScreenSnapshot, BackendError>;
    async fn subscribe(&self, spec: SubscriptionSpec) -> Result<BackendSubscription, BackendError>;
    async fn dispatch(&self, command: MuxCommand) -> Result<MuxCommandResult, BackendError>;
}
```

И отдельно:

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

### Why this shape is right

- host apps always speak one language
- each backend translates from that language
- unsupported semantics are explicit
- you can add `NativeMuxBackend`, `TmuxAdapter`, `ZellijAdapter` without changing host contracts

## Canonical DTOs you should freeze early

### Identity

```rust
pub struct SessionId(Uuid);
pub struct TabId(Uuid);
pub struct PaneId(Uuid);
pub struct SubscriptionId(Uuid);
```

External backend refs stay internal:

```rust
pub enum ExternalSessionRef {
    Tmux(TmuxSessionRef),
    Zellij(ZellijSessionRef),
}
```

🔥 public API should never expose raw `%12`, `@7`, `terminal_3` or tmux socket names as canonical truth.

For tmux-specific adapter internals, prefer bindings closer to:

```rust
pub struct TmuxTabBinding {
    pub session: TmuxSessionRef,
    pub window: TmuxWindowRef,
}
```

instead of assuming `@window` alone is enough.

For Zellij internals, prefer bindings closer to:

```rust
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
    pub focused_tab: TabId,
}

pub struct TabSnapshot {
    pub tab_id: TabId,
    pub title: String,
    pub root: PaneTreeNode,
    pub focused_pane: Option<PaneId>,
    pub capabilities: TabCapabilities,
}

pub enum PaneTreeNode {
    Split(SplitNodeSnapshot),
    Leaf(PaneSnapshot),
}
```

### Screen projection

```rust
pub struct ScreenSnapshot {
    pub pane_id: PaneId,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub surface: ScreenSurface,
    pub source: ProjectionSource,
}
```

`ProjectionSource` matters:

```rust
pub enum ProjectionSource {
    NativeEmulator,
    ImportedTmuxCapture,
    ImportedZellijViewport,
}
```

Это помогает не врать себе про происхождение истины.

## How each backend should fit

### `NativeMuxBackend`

Это ваш reference implementation.

Он должен быть strongest semantics owner.

Он задаёт эталон для:

- canonical IDs
- lifecycle phases
- capability vocabulary
- projection shapes
- fallback/error envelopes

### `TmuxAdapter`

Нужно проектировать его как:

- `tmux` server route
- control-mode session supervisor
- snapshot/resync helper around `capture-pane`
- metadata subscriptions around `refresh-client -B`

Правильный internal shape:

- один `TmuxServerSupervisor` на socket/server route
- один `TmuxSessionController` на attached session
- per-session event translator
- per-pane snapshot importer
- optional grouped-session strategy for independent host views
- one explicit resize-authority policy

Что брать из tmux:

- control-mode `%output`
- control-mode notifications
- `capture-pane`
- list/display formats
- explicit socket isolation via `-L` or `-S`

Чего не делать:

- не делать public contract вокруг tmux IDs
- не делать tmux window model вашей canonical tab model напрямую
- не обещать host-ам, что tmux может всё то же, что native runtime

### `ZellijAdapter`

Нужно проектировать его как:

- action-driven control adapter
- subscribe-driven viewport/event importer
- JSON query adapter for topology and state
- session-scoped CLI transport facade

Правильный internal shape:

- один `ZellijSessionSupervisor` на session route
- one ordered mutation lane / action executor
- long-lived subscribe workers
- topology importer from `list-panes`, `list-tabs`, `current-tab-info`, `dump-layout`
- typed pane-ref importer preserving terminal/plugin distinction
- tab importer preserving both backend `tab_id` and presentation `position`

Что брать из Zellij:

- `zellij action`
- `zellij subscribe`
- `dump-screen`
- `dump-layout`
- `list-panes --json`
- `list-tabs --json`
- `current-tab-info --json`

Чего не делать:

- не поднимать zellij plugin worldview в canonical product truth
- не обещать floating/plugin/layout semantics как universal baseline
- не смешивать Zellij terminal panes with your own host-side adjacent surfaces

## The hardest but most important design choice

### Do not model "pane" as only one thing

Для такой системы нужен минимум такой разрез:

```rust
pub enum PaneRuntimeKind {
    NativeTerminal,
    ForeignTerminal,
    ForeignAuxiliary,
}
```

Почему:

- native pane = ваш PTY + ваш emulator
- tmux pane = foreign pane with imported screen
- zellij terminal pane = foreign pane with imported screen
- zellij plugin pane = foreign auxiliary pane

Если этого разреза не сделать, вы:

- либо начнёте тащить plugin panes в core truth
- либо потеряете важную часть topology import

Самый здоровый путь:

- `ForeignAuxiliary` is representable
- but many operations on it are capability-gated or unavailable

## Capability system you need from day one

Минимальный capability set:

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

### Expected rough capability shape

- `NativeMux`
  - strongest long-term superset
- `tmux`
  - strong on sessions, windows, panes, attach/detach, notifications, socket-scoped servers
  - `raw_output_stream = true`
  - `rendered_viewport_stream = false`
  - `rendered_viewport_snapshot = true` through `capture-pane`
  - `advisory_metadata_subscriptions = true`
  - `independent_resize_authority = limited / policy-shaped`
  - weaker on rich machine-readable layout/state compared with Zellij
- `Zellij`
  - stronger on JSON state, CLI actions, subscribe, layout import/export, floating panes, read-only/watch style surfaces
  - `raw_output_stream = false`
  - `rendered_viewport_stream = true`
  - `rendered_viewport_snapshot = true`
  - `rendered_scrollback_snapshot = true`
  - `plugin_panes = true`
  - different worldview around plugins and layouts

🔥 Host logic must negotiate against capabilities, not against backend names.

## Fallback policy

Нельзя просто возвращать "unsupported" на всё.

Нужна явная политика:

### `Strict`

- command unsupported -> explicit error

### `BestEffort`

- command unsupported -> closest safe fallback

### `ObserveOnly`

- backend is imported mostly for attach/watch/control, not full editing

Это особенно полезно для:

- `tmux` imported sessions
- read-only `zellij` routes
- future remote/browser routes

## Session identity and route model

Вот где многие такие архитектуры начинают врать.

Нужно хранить:

```rust
pub struct SessionRoute {
    pub backend: BackendKind,
    pub authority: RouteAuthority,
    pub external_ref: Option<ExternalSessionRef>,
}
```

Почему:

- canonical `SessionId` and actual tmux session name are different truths
- canonical `PaneId` and `%pane` are different truths
- canonical `TabId` and tmux `(session, @window)` or zellij `tab_id` are different truths

Нужно явное mapping table storage:

- `canonical_session_id -> backend route`
- `canonical_tab_id -> external tab/window ref`
- `canonical_pane_id -> external pane ref`

## Daemon topology

Правильная runtime topology такая:

```text
host UI / Electron / SDK
        |
        v
terminal daemon
  - session registry
  - route registry
  - canonical IDs
  - capability negotiation
  - projection bus
        |
        +--> NativeMuxBackend
        +--> TmuxAdapter
        +--> ZellijAdapter
```

🔥 Host apps never talk directly to `tmux` or `zellij`.

Они всегда говорят с вашим daemon/runtime.

Иначе:

- capability logic расползётся по host-ам
- lifecycle drift пойдёт в Node/Electron
- вы потеряете reusable package story

## Crate layout I would freeze now

```text
crates/
  terminal-domain/
  terminal-mux-domain/
  terminal-backend-api/
  terminal-protocol/
  terminal-runtime/
  terminal-backend-native/
  terminal-backend-tmux/
  terminal-backend-zellij/
  terminal-daemon/
  terminal-node/
  terminal-capi/
  terminal-testing/
```

### Roles

- `terminal-backend-api`
  - traits
  - DTOs
  - capability and fallback envelopes
- `terminal-backend-native`
  - your real mux/runtime implementation
- `terminal-backend-tmux`
  - adapter around control-mode, capture-pane, format queries
- `terminal-backend-zellij`
  - adapter around action/subscribe/json queries
- `terminal-runtime`
  - orchestration and registry
- `terminal-daemon`
  - protocol server and composition root

## Implementation phases

### Phase 1

Freeze the contract only:

- `BackendKind`
- `SessionRoute`
- `BackendCapabilities`
- `MuxCommand`
- `TopologySnapshot`
- `ScreenSnapshot`
- `ProjectionSource`
- `DegradedModeReason`

### Phase 2

Implement `NativeMuxBackend`.

Why first:

- it defines your semantics
- it gives you a real test oracle
- it prevents tmux/zellij quirks from shaping your domain prematurely

### Phase 3

Implement **projection and capability tests** before any foreign backend.

Нужно зафиксировать golden contracts для:

- `TopologySnapshot`
- `ScreenSnapshot`
- degraded-mode envelopes
- capability negotiation
- imported-vs-native projection source handling

Why here:

- otherwise `TmuxAdapter` and `ZellijAdapter` начнут shape-ить contract implicitly

### Phase 4

Implement `TmuxAdapter` in **observe/control mode first**.

Why second:

- tmux model is narrower
- control-mode is old and stable
- great for proving imported foreign session semantics

Но v1 scope надо ограничить:

- attach/list/control
- pane snapshot import
- advisory metadata import
- send input / split / focus / close where representable
- explicit route-local tab binding policy
- explicit resize-authority policy
- decide and document whether v1 uses:
  - shared-focus attach semantics
  - or grouped-session isolation for independent host views

Не надо сразу обещать:

- full parity with native replay semantics
- full topology event fidelity
- durable persistence equivalence
- independent per-client focus parity without an explicit grouped-session policy

### Phase 5

Implement `ZellijAdapter` in **observe/control mode first**.

Why after tmux:

- richer external API
- more moving parts
- more optional features that need capability gating
- easier to overfit the public model to Zellij's richer external surface

Initial scope:

- query topology via JSON
- import viewport/scrollback snapshots
- subscribe to rendered pane updates
- control basic pane/tab actions
- serialize mutations through one adapter-owned command lane
- preserve typed pane refs and tab `tab_id` vs `position` semantics
- keep plugin/floating semantics explicitly capability-gated

### Phase 6

Only after both adapters are stable:

- add managed-session helpers
- add backend-specific convenience features
- add backend-specific policy profiles
- add any "preferred backend" automation

🔥 This is the corrected rollout plan.

Not:

- "implement both adapters fully right after the contract"

But:

- freeze canonical contract
- prove canonical projections
- import foreign backends conservatively
- expand only after capability and degraded-mode semantics are proven

## What you must not promise publicly

Не обещайте как public invariant:

- identical pane geometry across all backends
- identical floating-pane semantics
- identical plugin/auxiliary surface behavior
- identical layout persistence
- identical read-only or multi-client semantics
- identical notification/status behavior

Обещать надо другое:

- one canonical API
- explicit capabilities
- explicit degraded reasons
- stable IDs
- stable host protocol

И тут есть одно важное уточнение:

- "stable IDs" means **our canonical IDs**
- not backend refs from `tmux` or `Zellij`

## Final architecture verdict

Если фиксировать strongest architecture прямо сейчас, то она должна звучать так:

🔥 **We build one canonical terminal platform with one canonical mux contract.**

Inside that platform:

- `NativeMuxBackend` is the reference and strongest implementation
- `TmuxAdapter` is a foreign mux adapter
- `ZellijAdapter` is a foreign mux adapter
- host apps only consume canonical IDs, DTOs, capabilities and projections
- backend-specific IDs, sockets, session names, pane ids and plugin semantics stay inside adapters

Если ещё жёстче:

🔥 **`tmux` and `zellij` should be supported as backends, never as architecture owners.**

## Sources

- [tmux releases](https://github.com/tmux/tmux/releases)
- [tmux Control Mode wiki](https://github.com/tmux/tmux/wiki/Control-Mode)
- [tmux Advanced Use wiki](https://github.com/tmux/tmux/wiki/Advanced-Use)
- [tmux Getting Started wiki](https://github.com/tmux/tmux/wiki/Getting-Started)
- [Zellij releases](https://github.com/zellij-org/zellij/releases)
- [Zellij CLI Actions](https://zellij.dev/documentation/cli-actions.html)
- [Zellij Subscribe](https://zellij.dev/documentation/zellij-subscribe.html)
- [Zellij Programmatic Control](https://zellij.dev/documentation/programmatic-control.html)
- [Creating a Layout - Zellij User Guide](https://zellij.dev/documentation/creating-a-layout.html)
