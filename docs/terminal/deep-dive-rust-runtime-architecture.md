# Deep Dive - Rust Runtime Architecture

**Проверено**: 2026-04-19

## Зачем этот deep dive

После первого Rust shortlisting стало понятно, что теперь важен не просто список crates, а то, **как именно сильные Rust terminal stacks режут runtime на слои**:

- PTY transport
- emulator core
- read/write loops
- render/read model
- automation/control daemon
- mux/session donor patterns

Этот файл фиксирует именно source-level patterns, которые полезны для отдельного reusable Rust terminal package под Electron.

## Primary Sources

### WezTerm ecosystem

- [`portable-pty/src/lib.rs`](https://github.com/wezterm/wezterm/blob/main/pty/src/lib.rs)
- [`term/Cargo.toml`](https://github.com/wezterm/wezterm/blob/main/term/Cargo.toml)
- [`term/src/lib.rs`](https://github.com/wezterm/wezterm/blob/main/term/src/lib.rs)
- [`term/src/terminal.rs`](https://github.com/wezterm/wezterm/blob/main/term/src/terminal.rs)
- [`termwiz/README.md`](https://github.com/wezterm/wezterm/blob/main/termwiz/README.md)

### Alacritty

- [`alacritty_terminal/src/event_loop.rs`](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/event_loop.rs)
- [`alacritty_terminal/src/term/mod.rs`](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/term/mod.rs)

### Ghostty bindings

- [`libghostty-rs/crates/libghostty-vt/src/lib.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/lib.rs)
- [`libghostty-rs/crates/libghostty-vt/src/terminal.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/terminal.rs)
- [`libghostty-rs/crates/libghostty-vt/src/render.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/render.rs)

### Product-like Rust layers

- [`termwright/src/terminal.rs`](https://github.com/fcoury/termwright/blob/master/src/terminal.rs)
- [`termwright/src/daemon/server.rs`](https://github.com/fcoury/termwright/blob/master/src/daemon/server.rs)
- [`zellij-server/src/lib.rs`](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/lib.rs)
- [`zellij-server/src/pty_writer.rs`](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/pty_writer.rs)

## 1. `portable-pty` - a very clean PTY port boundary

`portable-pty` is not just "some helper crate".  
Its most important architectural value is that it already models PTY as a set of runtime-selected traits:

- `PtySystem`
- `MasterPty`
- `SlavePty`
- `Child`
- `ChildKiller`

Это сильный sign that PTY should stay its own port.

### Почему это важно

Если сделать `PTY + emulator + session runtime` одним giant abstraction, потом становится трудно:

- заменить backend
- тестировать attach/detach semantics
- держать separate read/write loops
- изолировать OS-specific weirdness

🔥 `portable-pty` уже даёт shape, который очень хорошо ложится в hexagonal architecture.

## 2. WezTerm stack actually splits into `portable-pty + wezterm-term + termwiz`

Это важная коррекция к предыдущему ресёрчу.

### `wezterm-term` is the actual emulator core

В `term/Cargo.toml` package называется `wezterm-term` и описан как:

- `The Virtual Terminal Emulator core from wezterm`

В `term/src/lib.rs` прямо сказано:

- full featured VT emulator core
- no GUI
- no direct PTY management
- embedder supplies a writer and feeds bytes via `advance_bytes`

То есть `wezterm-term` - это не UI toolkit, а именно emulator core.

### `termwiz` is adjacent toolkit, not the same layer

`termwiz` полезен, но он другой по роли:

- surface/cell modeling
- escape parsing helpers
- capabilities
- terminal abstraction
- widgets/line editor

⚠️ Если говорить строго архитектурно, `termwiz` и `wezterm-term` не надо смешивать в один conceptual box.

Практический вывод:

- если хочется reuse WezTerm internals, core-кандидат это скорее `wezterm-term`
- но это, судя по текущей картине, git-first dependency from repo, not a stable crates.io path

Это делает его сильным technically, но менее удобным как foundation для отдельно публикуемого package.

## 3. `alacritty_terminal` - parser/state and transport are clearly separate

`alacritty_terminal` source очень полезен именно тем, как там разведены границы.

### Event loop owns PTY IO, not terminal state

В `event_loop.rs` видно:

- отдельный `EventLoop`
- отдельный PTY `reader()/writer()`
- channel messages `Input / Shutdown / Resize`
- parser state хранится отдельно
- terminal state обновляется через shared `Term`

Полезные практические идеи:

- read loop and write queue deserve explicit runtime objects
- resize is a first-class message, not side effect of render code
- shutdown is protocol/state transition, not random flag

### Reader loop is careful about lock time

В `EventLoop::pty_read` есть очень здоровые ограничения:

- large read buffer
- unfair/try lock first
- force full lock only when necessary
- explicit `MAX_LOCKED_READ`
- wake UI only when sync bytes are not enough

🔥 Это хороший reminder, что emulator lock should not sit in the hot path longer than needed.

### `Term` is the terminal state object, not the runtime shell

В `term/mod.rs` видно:

- `TermMode`
- damage tracking
- selection/vi state
- screen/grid model

То есть `alacritty_terminal` очень хорошо подтверждает правильную shape:

- emulator core owns parser/state/damage
- session runtime owns PTY lifecycle and orchestration
- UI layer owns render policy and product shell

## 4. `libghostty-vt` - terminal state and render state are explicitly separate

Это, возможно, самый сильный source-level pattern из Rust deep dive.

### Terminal object is single-threaded by contract

В `lib.rs` bindings прямо сказано:

- objects are `!Send + !Sync`
- expectation is single-thread ownership
- communication with other threads through channels

Это не баг, а очень полезное architectural constraint.

Если брать `libghostty-vt`, runtime надо проектировать так, чтобы:

- emulator core жил на одном thread/actor
- cross-thread coordination шла через command/event channels

### Effects are synchronous and must stay cheap

В `terminal.rs` особенно важно:

- `vt_write` invokes effects synchronously
- callbacks must not block for too long
- expensive work in effect callbacks is explicitly discouraged

🔥 Это прямой design rule для нашего future `control surface`:

- hot path callbacks should only emit lightweight domain events
- analytics, persistence and expensive reactions must leave the hot path quickly

### Render state is its own object, not "just read from terminal"

`render.rs` показывает очень зрелую модель:

- separate `RenderState`
- update from `Terminal`
- dirty tracking at global and row levels
- iterators over rows and cells

Это очень сильный pattern:

- terminal state is one thing
- render snapshot/read model is another
- render/update loop should not poke arbitrary screen state directly

## 5. `termwright` - product-like runtime on top of `portable-pty + vt100`

`termwright` интересен не только как test tool, а как proof, how a compact runtime can be assembled quickly.

### Runtime stack is intentionally narrow

Из `src/terminal.rs` видно:

- `portable_pty`
- `vt100::Parser`
- background reader task
- shared `master`, `writer`, `parser`, `child`
- builder that injects env and cwd

Это довольно показательно:

✅ для first useful runtime package не обязательно сразу брать giant stack.  
Можно начать с узкого honest composition.

### Control daemon is simple JSON-RPC-ish over local socket

`daemon/server.rs` показывает:

- Unix socket daemon
- explicit `PROTOCOL_VERSION`
- one-line JSON request/response protocol
- methods like `screen`, `screenshot`, `type`, `press`, `wait_for_text`, `status`, `close`

Это очень сильный reference for Electron embedding:

- Rust runtime can expose a narrow local daemon/control surface
- UI can stay totally separate from PTY and parser ownership

### Product and automation can share the same runtime

Это один из лучших терминальных patterns в этом проходе:

- same runtime object
- automation methods hit the real session
- no fake parallel stack for tests

Это exactly what we want if this becomes a universal embeddable package.

## 6. `zellij-server` - heavy but instructive donor for thread topology

`zellij-server` слишком тяжёлый как dependency, но очень полезен как architecture donor.

### Runtime is split into explicit threads/modules

Из `src/lib.rs` видно разбиение на:

- `pty`
- `pty_writer`
- `screen`
- `route`
- `plugins`
- `background_jobs`
- `thread_bus`

Это подтверждает важную вещь:

large terminal runtime naturally decomposes into specialized loops, not one god object.

### Write path deserves its own thread and backpressure policy

`pty_writer.rs` особенно полезен:

- separate `PtyWriteInstruction`
- queued pending writes per terminal
- explicit byte cap `MAX_PENDING_BYTES`
- resize caching/apply cycle

🔥 Это очень сильный practical donor for our future write path:

- PTY writes deserve their own queueing policy
- resize storms should be coalesced
- pending bytes need hard caps

## 7. Practical architecture rules from all of this

### A. PTY and emulator must be different ports

Это подтверждают почти все сильные stacks:

- `portable-pty`
- `alacritty_terminal`
- `wezterm-term`
- `termwright`

PTY transport is not terminal state.

### B. Emulator core should not own session/runtime shell

Это видно и в `alacritty_terminal`, и в `wezterm-term`, и в `libghostty-vt`.

Core usually owns:

- parser
- screen/grid/state
- modes
- maybe render/read snapshots

But not:

- attach/detach session policy
- replay windows
- workstream shell
- notifications
- browser adjacency

### C. Single-threaded core is a valid and often healthier design

`libghostty-vt` особенно явно это подтверждает.

Sometimes the right move is:

- single-thread emulator actor
- channels around it
- explicit snapshots/events outward

instead of trying to make terminal state magically thread-safe everywhere.

### D. The write path needs separate design attention

`zellij-server` and `alacritty_terminal` both strongly suggest:

- do not casually write into PTY from random places
- queue writes
- cap buffers
- coalesce resizes

### E. Automation should reuse the same runtime

`termwright` is very strong proof here.

If we later want:

- tests
- AI agent inspection
- screenshots
- wait conditions

then they should sit on top of the real runtime/control surface, not a fake alternative stack.

## Updated practical take

If the goal is a **universal embeddable modern terminal runtime package in Rust**, the most convincing shape right now is:

1. `portable-pty` as PTY port
2. `alacritty_terminal` or `libghostty-vt` as emulator port
3. separate session-runtime/application layer
4. optional local daemon/control surface for Electron
5. automation/test APIs built on the same runtime

And one important nuance after the deeper dive:

⚠️ if we want to borrow from WezTerm, the serious core to study is `wezterm-term`, not just `termwiz`.
