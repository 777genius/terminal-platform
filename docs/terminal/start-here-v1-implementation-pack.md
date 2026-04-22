# Start Here: V1 Implementation Pack

**Проверено**: 2026-04-19
**Статус**: стартовый пакет для начала реализации

## Зачем этот файл

Исследовательских документов уже много.

Для старта реализации нужен не ещё один deep dive, а короткий operational center:

- что уже решено
- что нельзя переоткрывать без сильной причины
- в каком порядке читать docs
- какие артефакты и решения должны появиться первыми
- по какому definition of done двигаться

🔥 Этот файл и есть главный `start here`.

## Что читать в каком порядке

Если человек или агент подключается к работе с нуля, порядок такой:

1. [final-v1-blueprint-rust-terminal-platform.md](./final-v1-blueprint-rust-terminal-platform.md)
2. [deep-dive-rust-native-tmux-zellij-multi-backend-architecture.md](./deep-dive-rust-native-tmux-zellij-multi-backend-architecture.md)
3. [v1-workspace-bootstrap-spec.md](./v1-workspace-bootstrap-spec.md)
4. [v1-implementation-roadmap-and-task-breakdown.md](./v1-implementation-roadmap-and-task-breakdown.md)
5. [v1-verification-and-acceptance-plan.md](./v1-verification-and-acceptance-plan.md)
6. [research-rust-runtime-stack.md](./research-rust-runtime-stack.md) только как справочник по зависимостям и donor-проектам
7. [ideas-backlog.md](./ideas-backlog.md) только как backlog, а не как source of truth

## Frozen decisions for v1

Это решения, которые **не надо заново обсуждать** без сильного нового факта.

### Product shape

- делаем **universal embeddable terminal platform**
- UI - внешний consumer
- первый consumer - JS/Electron
- core/runtime - Rust

### Runtime truth

- `NativeMux` - reference truth
- daemon/protocol - first-class already in v1
- host talks only to our daemon/runtime

### Backend strategy

- `tmux` и `Zellij` поддерживаем
- но только как `foreign backends`
- parity с native в v1 не обещаем

### Dependency baseline

- PTY: `portable-pty`
- emulator: `alacritty_terminal`
- async/runtime: `tokio`, `tokio-util`, `bytes`
- local IPC: `interprocess`
- persistence: `rusqlite`, `rusqlite_migration`
- tracing: `tracing`
- tests: `proptest`, `insta`, `cargo-fuzz`, `nextest`, `expectrl`
- Node host leaf: `napi-rs`
- C ABI leaf: `safer-ffi`, `cbindgen`, `cargo-c`

### Architecture rules

- protocol-first, FFI-second
- canonical IDs are ours, backend refs are internal
- control plane and data plane are separate contracts
- capability negotiation is explicit
- degraded-mode reasons are explicit
- foreign adapter semantics must never silently redefine the public contract

## Frozen vocabulary for the start phase

Эти термины теперь считаются каноническими и не должны дрейфовать между docs и кодом:

- `NativeMux` - reference runtime truth
- `foreign backend` - imported backend like `tmux` or `Zellij`
- `SessionRoute` - how canonical session is bound to backend authority
- `TopologySnapshot` - canonical structural view of tabs, panes and focus
- `ScreenSnapshot` - canonical rendered screen surface for one pane
- `ProjectionSource` - where snapshot or delta came from
- `BackendCapabilities` - explicit feature envelope, not implied parity
- `DegradedModeReason` - typed reason why behavior differs or narrows
- `resize authority` - who actually controls size semantics for imported route
- `observe/control scope` - imported backend level we allow in v1

## What is still open

Эти вещи ещё можно решать по ходу реализации:

- exact framed protocol encoding for v1 envelopes
- exact SQLite schema names and table layout
- whether v1 uses `typed-builder` or plain spec structs first
- whether `tmux` independent host views enter v1 or slip to v1.1
- whether first Node host ships through direct client library or through daemon-first package only

## What is explicitly out of scope for v1

- full backend parity
- SSH as first-class stable route
- plugin platform
- standalone GUI app
- media protocols as core requirement
- public promise of identical behavior across Native, tmux and Zellij

## Deliverables required before writing lots of code

### 1. Repo/bootstrap deliverables

- Rust workspace root
- base `Cargo.toml`
- workspace lint/test/profile policy
- release/check tooling config
- exact bootstrap structure from [v1-workspace-bootstrap-spec.md](./v1-workspace-bootstrap-spec.md)

### 2. Contract deliverables

- canonical IDs
- `BackendKind`
- `SessionRoute`
- `BackendCapabilities`
- `MuxCommand`
- `TopologySnapshot`
- `ScreenSnapshot`
- `ProjectionSource`
- `DegradedModeReason`

### 3. Daemon deliverables

- local socket transport
- handshake
- operation routing
- subscription lifecycle
- peer/auth policy hooks

### 4. Native backend deliverables

- create/list/attach native sessions
- topology snapshots
- screen snapshots
- input path
- split/tab/focus/close commands

### 5. Host deliverables

- Node/Electron client
- TS DTO generation
- first integration path for repo app shell

## Mapping to the main repo standard

Главный репозиторный стандарт живёт в:

- host repo specific feature standard should be mapped at integration time, not copied into this Rust repo as a second source of truth

Новая terminal feature должна уважать его, но здесь есть важная оговорка:

- terminal runtime itself is a separate Rust project/platform
- Electron repo consumes it as a feature integration

Значит для JS/Electron части делаем так:

```text
src/features/terminal/
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

Но Rust workspace живёт рядом как отдельный product unit.

🔥 Важно:

- не пытаться впихнуть весь Rust runtime в `src/features/terminal`
- и не тащить Electron semantics в Rust core

## Workspace shape to create first

```text
./
  Cargo.toml
  rust-toolchain.toml
  crates/
    terminal-domain/
    terminal-mux-domain/
    terminal-backend-api/
    terminal-protocol/
    terminal-runtime/
    terminal-projection/
    terminal-persistence/
    terminal-transport/
    terminal-backend-native/
    terminal-backend-tmux/
    terminal-backend-zellij/
    terminal-daemon/
    terminal-daemon-client/
    terminal-node/
    terminal-capi/
    terminal-testing/
```

## What "ready to start coding" actually means

Считать, что старт phase завершён, можно только если:

- blueprint frozen enough that DTO names stop drifting
- bootstrap spec is accepted
- roadmap milestones are accepted
- verification plan already exists before heavy code
- first sprint tasks are clear enough to open directly
- no one is still debating whether `tmux` and `Zellij` are product truth

Если хотя бы один из этих пунктов плавает, команда ещё не на старте implementation, а всё ещё в design churn.

## First week checklist

### Day 1-2

- freeze crate names
- freeze DTO names
- freeze capability vocabulary
- freeze `BackendKind` and route model
- freeze host handshake envelope

### Day 3-4

- scaffold workspace
- scaffold domain/api/protocol crates
- compile empty types and traits
- set up CI/test commands locally

### Day 5-7

- daemon skeleton with handshake
- in-memory native session registry stub
- no real PTY yet
- one end-to-end smoke path:
  - host connects
  - handshake succeeds
  - `list_sessions` returns empty array

## First month target

К концу первого большого implementation цикла должно быть:

- working daemon
- Native create/list/attach
- topology snapshot
- screen snapshot
- first Node/Electron client
- basic split/tab/focus/send input
- projection golden tests

Не должно быть обязательным к этому моменту:

- complete restore story
- tmux independent host views
- full Zellij parity

## Main risks to watch from day one

### Risk 1 - backend semantics leak into public contract

Симптомы:

- `tmux` ids or `Zellij` refs начинают жить в public DTO
- host code начинает делать branch по backend instead of capability

### Risk 2 - daemon becomes a thin tunnel instead of runtime authority

Симптомы:

- host expects to talk directly to PTY-ish primitives
- subscriptions and lifecycle drift outside daemon

### Risk 3 - Node/Electron becomes the real architecture center

Симптомы:

- DTO shape starts following binding quirks
- Rust crates start waiting on preload/renderer assumptions

### Risk 4 - foreign adapter parity gets promised too early

Симптомы:

- docs or code assume `Native == tmux == Zellij`
- unsupported behavior is hidden instead of surfaced via degraded semantics

## Non-negotiable coding rules

- no backend-specific refs in public contracts
- no host binding as architectural center
- no giant crate
- no direct UI -> tmux/zellij access
- no ad hoc JSON blobs instead of typed DTOs
- no “we’ll fix degraded semantics later”

## Documentation rules during implementation

Во время работы все новые решения должны ложиться в один из трёх buckets:

- blueprint updates
- roadmap updates
- verification plan updates

Не надо снова плодить research-style docs на каждую маленькую мысль.

## Decision rule when blocked

Если появляется спорный вопрос, решать так:

1. does this preserve canonical product truth?
2. does this keep foreign backend semantics contained?
3. does this improve testability and explicit degraded behavior?
4. does this avoid coupling the public contract to one host or backend?

Если ответ `нет`, решение почти наверняка плохое.

## Main implementation mantra

🔥 **Сначала freezes and seams. Потом native truth. Потом host client. Потом foreign adapters. Потом convenience.**
