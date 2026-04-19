# Deep Dive - Rust Workspace Topologies For Reusable Terminal Platforms

**Проверено**: 2026-04-19

## Зачем этот deep dive

После разбора отдельных crates стало видно, что для terminal project мирового уровня мало выбрать:

- хороший PTY layer
- хороший emulator core
- хороший embed boundary

Нужно ещё правильно выбрать **crate topology всего workspace**.

Именно это определяет:

- можно ли публиковать части системы отдельно
- не прилипает ли UI к runtime
- не превращается ли `utils` crate в помойку
- выдержит ли проект одновременно:
  - standalone terminal app
  - embeddable SDK
  - Electron host adapter
  - future remote/runtime/server modes

## Primary Sources

### WezTerm

- [`wezterm/Cargo.toml`](https://github.com/wezterm/wezterm/blob/main/Cargo.toml)
- [`wezterm/mux/Cargo.toml`](https://github.com/wezterm/wezterm/blob/main/mux/Cargo.toml)
- [`wezterm/wezterm-mux-server/Cargo.toml`](https://github.com/wezterm/wezterm/blob/main/wezterm-mux-server/Cargo.toml)
- [`wezterm/wezterm-ssh/Cargo.toml`](https://github.com/wezterm/wezterm/blob/main/wezterm-ssh/Cargo.toml)
- [`wezterm/wezterm-gui/Cargo.toml`](https://github.com/wezterm/wezterm/blob/main/wezterm-gui/Cargo.toml)

### Zellij

- [`zellij/Cargo.toml`](https://github.com/zellij-org/zellij/blob/main/Cargo.toml)
- [`zellij/zellij-client/Cargo.toml`](https://github.com/zellij-org/zellij/blob/main/zellij-client/Cargo.toml)
- [`zellij/zellij-server/Cargo.toml`](https://github.com/zellij-org/zellij/blob/main/zellij-server/Cargo.toml)
- [`zellij/zellij-utils/Cargo.toml`](https://github.com/zellij-org/zellij/blob/main/zellij-utils/Cargo.toml)

### Alacritty

- [`alacritty/Cargo.toml`](https://github.com/alacritty/alacritty/blob/master/Cargo.toml)

### Rio

- [`rio/Cargo.toml`](https://github.com/raphamorim/rio/blob/main/Cargo.toml)
- [`rio/teletypewriter/Cargo.toml`](https://github.com/raphamorim/rio/blob/main/teletypewriter/Cargo.toml)
- [`rio/copa/Cargo.toml`](https://github.com/raphamorim/rio/blob/main/copa/Cargo.toml)
- [`rio/rio-backend/Cargo.toml`](https://github.com/raphamorim/rio/blob/main/rio-backend/Cargo.toml)
- [`rio/frontends/rioterm/Cargo.toml`](https://github.com/raphamorim/rio/blob/main/frontends/rioterm/Cargo.toml)
- [`rio/sugarloaf/Cargo.toml`](https://github.com/raphamorim/rio/blob/main/sugarloaf/Cargo.toml)

## Freshness signals

- [`wezterm/wezterm`](https://github.com/wezterm/wezterm) - `25.6k+` stars, pushed `2026-04-01`
- [`zellij-org/zellij`](https://github.com/zellij-org/zellij) - `31.7k+` stars, pushed `2026-04-17`
- [`alacritty/alacritty`](https://github.com/alacritty/alacritty) - `63.5k+` stars, pushed `2026-04-14`
- [`raphamorim/rio`](https://github.com/raphamorim/rio) - `6.6k+` stars, pushed `2026-04-19`

## Короткий вывод

🔥 Самый сильный topology pattern для вашей цели это не "один большой crate" и не "отдельный runtime crate + куча случайных glue модулей".

Лучший shape сейчас выглядит как гибрид:

- `Alacritty` по чистоте reusable emulator crate
- `WezTerm` по разрезу `pty / term / mux / ssh / gui / server`
- `Zellij` по явному `client / server / protocol-ish shared utils / plugin leaves`
- `Rio` по `backend + frontend leaf + reusable low-level crates`

## Top 3 Workspace Shapes

### 1. `Core + protocol + runtime + adapters + host leaves`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `9000-18000` строк до первой сильной topology.

Идея:

- `terminal-core`
- `terminal-protocol`
- `terminal-runtime`
- `terminal-daemon`
- `terminal-capi`
- `terminal-node`
- host-specific apps as leaf crates

Почему это лучший путь:

- reusable crates clearly separated from host/app code
- UI can be swapped freely
- C ABI and Node adapters remain thin
- standalone app can be built on top of the same runtime

### 2. `Client / server / shared / plugins`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `7000-15000` строк.

Идея:

- explicit client crate
- explicit server crate
- shared protocol/utils crate
- adapters/plugins as separate leaves

Почему это сильно:

- good for daemon mode and future remote/web features
- makes session ownership clearer

Главный риск:

- `shared/utils` crate can become a dumping ground fast

### 3. `Backend library + frontend leaf + reusable low-level crates`

`🎯 7   🛡️ 8   🧠 6`  
Примерно `5000-12000` строк.

Идея:

- reusable low-level crates for parser/pty/render support
- one `backend` crate
- one or more frontend binaries as leaves

Почему это хорошо:

- simple and clean
- especially good if first goal is standalone app with later embedding

Главный риск:

- if daemon/protocol/adapter story grows later, backend crate can become too fat

## Project-by-project findings

## 1. WezTerm - the strongest evidence for separating `term`, `pty`, `mux`, `server`, `gui`

WezTerm workspace root already shows a strong multi-crate shape:

- `portable-pty`
- `wezterm-term`
- `mux`
- `wezterm-mux-server`
- `wezterm-ssh`
- `wezterm-gui`
- `wezterm`

### What this teaches

#### `mux` is its own crate

This is one of the strongest patterns in the whole research.

Their `mux` crate depends on:

- `portable-pty`
- `wezterm-term`
- `wezterm-ssh`
- `termwiz`

🔥 This is a very clear signal:

- emulator core is not the session platform
- session/multiplexer/runtime orchestration deserves its own crate boundary

#### GUI is a leaf, not the owner of terminal truth

`wezterm-gui` depends on runtime-ish things like:

- `mux`
- `wezterm-term`
- `portable-pty`
- `wezterm-client`

That is the right direction for us too:

- UI crate should consume runtime truths
- UI crate should not define them

#### Some crates are reusable, some are product-internal

Several app-level crates are `publish = false`, while reusable lower layers are shaped like library crates.

This is a good packaging lesson:

✅ publish only what is genuinely reusable  
⚠️ do not force app-shell crates to masquerade as public SDK crates

## 2. Zellij - strongest evidence for explicit client/server split and plugin leaves

Zellij root package depends on:

- `zellij-client`
- `zellij-server`
- `zellij-utils`

Workspace members also include many plugin crates:

- `default-plugins/*`
- `zellij-tile`
- `zellij-tile-utils`

### What this teaches

#### Client and server should be different crates

This is a very strong pattern if you want:

- local daemon mode
- future remote mode
- recoverable sessions
- multiple host surfaces

#### Plugins should stay as leaves

Their plugin crates are not mixed into server/client core boundaries.

That is exactly how adapters/extensions should behave in our project too:

- plugin/adapter leaves should depend on core
- core should not depend on plugin/adapter leaves

#### `utils` crates are useful but dangerous

`zellij-utils` exists for a reason, but it also shows the classic risk:

- config
- logging
- ids
- protocol helpers
- assets
- transport helpers

all start accumulating in one place.

⚠️ This is the main topology trap I want to avoid in our design.

If we need shared crates, better to prefer:

- `terminal-protocol`
- `terminal-config`
- `terminal-telemetry-types`

over one giant generic `terminal-utils`.

## 3. Alacritty - best evidence that emulator core can stay clean and reusable

Alacritty workspace stays minimal:

- `alacritty`
- `alacritty_terminal`
- `alacritty_config`
- `alacritty_config_derive`

### What this teaches

#### The emulator crate can stay beautifully narrow

`alacritty_terminal` proves that:

- terminal emulation
- parser/state
- event loop concerns

can live in a reusable library crate without dragging in the app shell.

🔥 This is one of the best references for our future `terminal-emulator-*` crates.

#### App crate should be a consumer, not the architecture center

The top application crate remains a leaf over reusable components.

That is exactly the discipline we want if Electron embedding is only one consumer among many.

## 4. Rio - best evidence for splitting parser, PTY, renderer, backend and frontend

Rio workspace is very instructive because it splits along capability lines:

- `teletypewriter` - PTY
- `copa` - parser
- `sugarloaf` - renderer
- `rio-backend` - backend infrastructure
- `frontends/rioterm` - app frontend

### What this teaches

#### Low-level crates can be reusable products

Both `teletypewriter` and `sugarloaf` are described as reusable crates in their own right.

That is a very useful mindset:

- not every reusable crate must be hidden under one "sdk" package
- some subsystems may deserve independent public identities

#### Frontend leaf under `frontends/`

This is a very clean pattern:

- backend stays library-shaped
- standalone app is just one frontend leaf

✅ I like this a lot for our future standalone host story.

#### Backend crate can still become too broad

`rio-backend` bundles many concerns:

- parser
- PTY
- rendering integration
- clipboard, URLs, window-adjacent logic

That is useful as a warning:

⚠️ `backend` is a convenient name, but it can easily become another monolith if protocol/runtime boundaries are not explicit.

## Recommended workspace topology for our package

If we optimize for:

- standalone app
- embeddable runtime
- Electron host
- future other languages

I would target this:

```text
terminal-runtime/
  Cargo.toml
  crates/
    terminal-domain/
    terminal-protocol/
    terminal-runtime/
    terminal-pty-portable/
    terminal-emulator-alacritty/
    terminal-daemon/
    terminal-capi/
    terminal-node/
    terminal-testing/
  apps/
    terminal-cli/
    terminal-desktop/
```

## Roles

### `terminal-domain`

- session identity
- runtime mode
- commands/events
- value objects and error taxonomy

### `terminal-protocol`

- host-neutral DTOs
- event envelopes
- versioning and capability negotiation

### `terminal-runtime`

- session lifecycle
- replay/snapshot
- backpressure
- orchestration

### `terminal-pty-portable`

- `portable-pty` adapter implementation

### `terminal-emulator-alacritty`

- `alacritty_terminal` adapter implementation

### `terminal-daemon`

- local socket host
- attach/detach
- process isolation from UI

### `terminal-capi`

- C ABI layer
- generated headers
- installable library surface

### `terminal-node`

- `napi-rs` adapter for Electron/Node

### `terminal-testing`

- conformance tests
- automation helpers
- snapshot/test harness

### `apps/*`

- leaf hosts
- no product truth should live only here

## Packaging policy

Очень полезный урок из WezTerm/Rio:

- some crates should be publishable
- some crates should stay product-internal

Recommended rule:

- publish:
  - `terminal-protocol`
  - `terminal-pty-portable`
  - `terminal-emulator-*`
  - maybe `terminal-testing`
- decide case-by-case:
  - `terminal-runtime`
  - `terminal-daemon`
  - `terminal-capi`
  - `terminal-node`
- keep internal or host-specific:
  - `apps/*`

## Biggest topology risks

### 1. Giant `utils` crate

This is the most obvious trap after reading Zellij.

### 2. Backend monolith

This is the main cautionary lesson from Rio-style backend crates.

### 3. UI truth leakage

This is what WezTerm avoids better than many projects:

- GUI consumes runtime
- GUI does not define core runtime truth

### 4. Adapter-owned semantics

If Node/C ABI adapters start owning lifecycle rules, reusable package quality drops immediately.

## Final take

If we want a **universal Rust terminal platform**, the strongest workspace design is:

- `Alacritty`-style narrow reusable core crates
- `WezTerm`-style separation of `pty / term / mux / server / gui`
- `Zellij`-style explicit `client / server / plugins`
- `Rio`-style leaf frontends over reusable backend pieces

Итоговый practical rule:

🔥 **host apps must be leaves, runtime must be central, adapters must stay thin, and shared crates must be explicit by role, not generic by convenience.**
