# V1 Workspace Bootstrap Spec

**Проверено**: 2026-04-19
**Статус**: bootstrap source of truth

## Why this exists

`final-v1-blueprint-rust-terminal-platform.md` фиксирует архитектуру.

Но для реального старта нужен ещё и точный bootstrap spec:

- где живёт Rust workspace
- какие файлы должны появиться первыми
- какие crates создаём сразу
- какие зависимости разрешены по направлению
- какие команды и quality gates должны заработать в первый же день

🔥 Этот документ нужен, чтобы старт implementation не развалился в ad hoc scaffold.

## Placement decision

Для v1 Rust runtime оформляем как отдельный product repository:

```text
/Users/belief/dev/projects/claude/terminal-platform/
```

Почему так:

- Rust platform сразу живёт как самостоятельный product unit
- host integration не диктует layout runtime repo
- проще публиковать, тестировать и выпускать platform отдельно от конкретного app shell

## Bootstrap objectives

Стартовый bootstrap обязан дать:

- compile-ready пустой workspace
- зафиксированную crate graph форму
- базовый lint/test discipline
- одно место для shared dependency versions
- нулевой соблазн делать giant crate или binding-first architecture

## Required root files on day one

### Must exist immediately

- `./Cargo.toml`
- `./rust-toolchain.toml`
- `./.cargo/config.toml`
- `./rustfmt.toml`
- `./clippy.toml`
- `./.config/nextest.toml`
- `./README.md`
- `./crates/*/Cargo.toml`
- `./crates/*/src/lib.rs`

### Can wait until after bootstrap

- `./deny.toml`
- `./release-plz.toml`
- `./dist-workspace.toml`
- `./fuzz/`
- `./scripts/`

## Root workspace skeleton

```text
./
  Cargo.toml
  rust-toolchain.toml
  rustfmt.toml
  clippy.toml
  README.md
  .cargo/
    config.toml
  .config/
    nextest.toml
  crates/
    terminal-domain/
    terminal-mux-domain/
    terminal-backend-api/
    terminal-protocol/
    terminal-application/
    terminal-projection/
    terminal-persistence/
    terminal-backend-native/
    terminal-backend-tmux/
    terminal-backend-zellij/
    terminal-daemon/
    terminal-daemon-client/
    terminal-node/
    terminal-capi/
    terminal-testing/
```

## Workspace root policy

### `Cargo.toml`

Root workspace file должен:

- включать все crates через `members = ["crates/*"]`
- использовать `resolver = "2"`
- держать shared dependency versions в `[workspace.dependencies]`
- держать общие lint expectations в `[workspace.lints]`, если решим это включить сразу

### `rust-toolchain.toml`

Фиксируем:

- stable toolchain
- `rustfmt`
- `clippy`

Никаких nightly assumptions в v1 bootstrap.

### `.cargo/config.toml`

На старте достаточно:

- sensible target-dir defaults
- alias-ы для частых команд, если реально помогут

Не надо превращать bootstrap в shell framework.

### `rustfmt.toml` and `clippy.toml`

Нужны сразу, чтобы:

- стиль не дрейфовал между crates
- warnings policy была одинаковой

### `.config/nextest.toml`

Нужен сразу как signal, что это будет большой workspace, а не toy crate.

## Shared dependency baseline for bootstrap

В `[workspace.dependencies]` на первом проходе стоит завести только foundation:

- `tokio`
- `tokio-util`
- `bytes`
- `tracing`
- `serde`
- `serde_json`
- `uuid`
- `thiserror`

Следующий слой можно подключать, когда соответствующие milestones реально начнутся:

- `interprocess`
- `portable-pty`
- `alacritty_terminal`
- `rusqlite`
- `rusqlite_migration`
- `proptest`
- `insta`

🔥 Не надо тащить сразу весь final stack в bootstrap, если crate ещё не использует его.

## Crate dependency direction rules

Это важнее, чем конкретные версии.

### Inward crates

- `terminal-domain`
- `terminal-mux-domain`

Они:

- не знают о Tokio
- не знают о subprocesses
- не знают о Electron/Node/C
- не знают о конкретных backends

### Boundary and shared application crates

- `terminal-backend-api`
- `terminal-protocol`
- `terminal-projection`
- `terminal-application`
- `terminal-persistence`

Они:

- зависят только inward
- не знают о конкретном host binding
- не знают о конкретном backend implementation

### Concrete infrastructure crates

- `terminal-backend-native`
- `terminal-backend-tmux`
- `terminal-backend-zellij`
- `terminal-daemon`
- `terminal-daemon-client`

Они:

- зависят inward
- могут зависеть на `terminal-backend-api`, `terminal-application`, `terminal-protocol`
- не должны образовывать циклы

### Host leaves

- `terminal-node`
- `terminal-capi`

Они:

- зависят inward
- не должны становиться источником канонических DTO
- не должны тащить host-specific semantics назад в protocol/domain crates

## Initial crate surfaces

### `terminal-domain`

Стартовые модули:

- `ids.rs`
- `backend_kind.rs`
- `session_route.rs`
- `degraded_mode.rs`
- `lib.rs`

### `terminal-mux-domain`

Стартовые модули:

- `pane_tree.rs`
- `tab_snapshot.rs`
- `pane_snapshot.rs`
- `focus.rs`
- `lib.rs`

### `terminal-backend-api`

Стартовые модули:

- `capabilities.rs`
- `commands.rs`
- `errors.rs`
- `ports.rs`
- `subscriptions.rs`
- `lib.rs`

### `terminal-protocol`

Стартовые модули:

- `handshake.rs`
- `envelope.rs`
- `requests.rs`
- `responses.rs`
- `subscriptions.rs`
- `errors.rs`
- `lib.rs`

### `terminal-projection`

Стартовые модули:

- `screen_snapshot.rs`
- `screen_delta.rs`
- `topology_snapshot.rs`
- `projection_source.rs`
- `lib.rs`

### `terminal-application`

Стартовые модули:

- `services/`
- `use_cases/`
- `registry/`
- `lib.rs`

### `terminal-persistence`

Стартовые модули:

- `schema/`
- `migrations/`
- `repositories/`
- `lib.rs`

### `terminal-backend-native`

Стартовые модули:

- `backend.rs`
- `session.rs`
- `runtime.rs`
- `lib.rs`

### `terminal-backend-tmux`

Стартовые модули:

- `backend.rs`
- `control_mode.rs`
- `capture.rs`
- `mapping.rs`
- `lib.rs`

### `terminal-backend-zellij`

Стартовые модули:

- `backend.rs`
- `actions.rs`
- `subscribe.rs`
- `mapping.rs`
- `lib.rs`

### `terminal-daemon`

Стартовые модули:

- `server.rs`
- `routing.rs`
- `peer_auth.rs`
- `state.rs`
- `lib.rs`

### `terminal-daemon-client`

Стартовые модули:

- `client.rs`
- `subscription.rs`
- `lib.rs`

### `terminal-node`

Стартовые модули:

- `lib.rs`
- `client.rs`
- `generated/`

### `terminal-capi`

Стартовые модули:

- `lib.rs`
- `handles.rs`
- `ffi_types.rs`

### `terminal-testing`

Стартовые модули:

- `fixtures/`
- `golden/`
- `smoke/`
- `lib.rs`

## What bootstrap must not do

Во время bootstrap запрещено:

- писать реальный PTY runtime
- писать `tmux` subprocess orchestration
- писать `Zellij` subscribe workers
- писать Node bindings как product center
- тащить persistence schema глубже, чем нужно для compile-ready scaffolding
- лепить один util crate для всего подряд

## Initial quality gates

На первом проходе должны работать эти команды:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo nextest run --workspace
```

Если какая-то из них не запускается на bootstrap phase, bootstrap ещё не завершён.

## Minimum CI lanes to mirror locally

### Lane 1 - Format

- `cargo fmt --all --check`

### Lane 2 - Lints

- `cargo clippy --workspace --all-targets --all-features`

### Lane 3 - Tests

- `cargo nextest run --workspace`

Later, but not on day one:

- fuzz
- adapter smoke suites
- semver checks
- cargo-deny

## Bootstrap Definition Of Done

Bootstrap можно считать завершённым только если:

- workspace root создан
- все agreed crates существуют
- все crates компилируются
- dependency directions не нарушены
- root lint/test commands работают
- есть короткий workspace README с map of crates
- дальнейшая работа может идти по milestones, а не по "надо ещё придумать каркас"

## Main bootstrap rule

🔥 **Bootstrap phase должен заморозить seams, а не начать случайную реализацию.**
