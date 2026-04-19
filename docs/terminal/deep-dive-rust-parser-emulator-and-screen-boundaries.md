# Deep Dive - Rust Parser, Emulator, And Screen Boundaries

**Проверено**: 2026-04-19

## Зачем этот deep dive

В Rust terminal ecosystem очень легко спутать 4 разных слоя:

1. raw escape parser
2. terminal emulator core
3. virtual screen / diff model
4. host-side TUI / app shell

Если их смешать, быстро появляется ложное решение вида:

- "давайте возьмём `crossterm` как terminal engine"
- "давайте строить runtime на `ratatui`"
- "давайте сразу тащить `vt100` как единственный emulator core"

Для universal embeddable terminal package это опасно.
Нужно очень чётко понимать, какой crate за что отвечает.

## Primary Sources

### Parser and screen layers

- [`vte` README](https://github.com/alacritty/vte/blob/master/README.md)
- [`vte/src/lib.rs`](https://github.com/alacritty/vte/blob/master/src/lib.rs)
- [`vt100-rust` README](https://github.com/doy/vt100-rust/blob/main/README.md)
- [`vt100-rust/src/lib.rs`](https://github.com/doy/vt100-rust/blob/main/src/lib.rs)
- [`asciinema/avt` README](https://github.com/asciinema/avt/blob/main/README.md)
- [`asciinema/avt/src/lib.rs`](https://github.com/asciinema/avt/blob/main/src/lib.rs)

### Emulator cores and PTY layer

- [`portable-pty/src/lib.rs`](https://github.com/wezterm/wezterm/blob/main/pty/src/lib.rs)
- [`alacritty_terminal/src/lib.rs`](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/lib.rs)
- [`termwiz` README](https://github.com/wezterm/wezterm/blob/main/termwiz/README.md)
- [`shadow-terminal` README](https://github.com/tattoy-org/shadow-terminal/blob/main/README.md)

### Host-side terminal UI libraries

- [`crossterm` README](https://github.com/crossterm-rs/crossterm/blob/master/README.md)
- [`ratatui` README](https://github.com/ratatui/ratatui/blob/main/README.md)

## Freshness signals

### Parser / screen model crates

- `vte 0.15.0` - repo `311` stars, pushed `2026-02-28`
- `vt100 0.16.2` - repo `112` stars, pushed `2025-07-12`
- `avt 0.17.0` - repo `102` stars, pushed `2025-09-20`

### Emulator / PTY crates

- `portable-pty 0.9.0`
- `alacritty_terminal 0.26.0`
- `termwiz 0.23.3`
- `shadow-terminal 0.2.3`

### Host-side UI crates

- `crossterm 0.29.0` - repo `4010` stars, pushed `2026-04-08`
- `ratatui 0.30.0` - repo `19.9k` stars, pushed `2026-04-17`

## Короткий вывод

🔥 Самый важный новый вывод этого прохода:

- `vte` - **parser brick**
- `vt100` / `avt` - **virtual screen layer**
- `alacritty_terminal` / `libghostty-vt` - **real emulator core**
- `crossterm` / `ratatui` - **host-side terminal UI libraries, not runtime core**

Это значит, что для вашего Rust terminal package здоровый shape выглядит так:

- `PtyPort` -> `portable-pty`
- `EmulatorPort` -> `alacritty_terminal` or later `libghostty-vt`
- `HeadlessScreenPort` -> `vt100` / `avt` / `shadow-terminal` style tools where needed
- host app shell -> JS UI, or standalone Rust desktop/TUI shell separately

## Top 3 Emulator / parser directions

### 1. `portable-pty + alacritty_terminal` as default emulator stack

`🎯 10   🛡️ 9   🧠 7`  
Примерно `7000-13000` строк.

Это остаётся моим strongest production default.

Почему:

- `portable-pty` already gives the right PTY boundary
- `alacritty_terminal` is explicitly a library for writing terminal emulators
- its module surface already separates:
  - `event_loop`
  - `grid`
  - `selection`
  - `term`
  - `tty`
- it internally sits on top of `vte`, but does not expose parser-only semantics as your main runtime contract

Главный смысл:

🔥 вы берёте не просто parser, а уже battle-tested emulator core

### 2. `portable-pty + libghostty-vt`

`🎯 8   🛡️ 7   🧠 8`  
Примерно `8000-15000` строк.

Это strongest modern bet, если хотите идти в более сильный long-term engine path.

Почему интересно:

- strong terminal semantics
- cleaner render-state separation
- good fit for single-owner runtime model

Почему не default:

- ecosystem ещё моложе
- build and packaging story тяжелее
- для publishable package risk profile выше

### 3. `portable-pty + vt100/avt` only for focused headless surfaces

`🎯 7   🛡️ 8   🧠 5`  
Примерно `4000-9000` строк.

Это хороший путь не как main emulator, а как narrow layer for:

- snapshots
- diffs
- replay tooling
- test harnesses
- tmux/screen-like helper flows

Почему:

- `vt100` already gives screen diffing and formatted content APIs
- `avt` explicitly scopes itself to parser + virtual buffer and says input/rendering are out of scope

Но:

- это narrower virtual terminal layer
- для full modern embeddable terminal platform этого мало

## 1. `vte` - parser brick, not emulator core

`vte` README и `src/lib.rs` очень прямолинейны:

- parser implements Paul Williams ANSI parser state machine
- parser itself assigns no meaning
- caller provides a `Perform` implementation

Это очень полезный architectural lesson.

### Что `vte` хорошо делает

- state machine over byte stream
- UTF-8 handling
- OSC buffering
- low-level action callbacks

### Чего `vte` не даёт

- screen model
- selection
- damage tracking
- diffing
- scrollback semantics
- product runtime lifecycle

Итог:

✅ `vte` is a great low-level brick  
⚠️ `vte` is not the terminal platform you want to expose as your `EmulatorPort`

## 2. `vt100` - parser plus in-memory rendered screen

`vt100` sits one layer higher.

Из README и `src/lib.rs` видно:

- parses terminal byte stream
- gives in-memory representation of rendered contents
- exposes `Parser` and `Screen`
- has APIs like:
  - `contents_formatted()`
  - `contents_diff(...)`
  - cell/color querying

Это очень ценно.

### Где `vt100` особенно хорош

- headless snapshots
- diffing output
- testing terminal apps
- mux-like or replay-oriented helpers

### Где `vt100` слабее как foundation

- much narrower ecosystem signal
- not positioned as full product-grade emulator platform
- less obvious path for rich modern semantics than `alacritty_terminal` or `libghostty-vt`

Практический вывод:

🔥 `vt100` is more interesting as a **screen/read-model tool** than as the core of the whole package

## 3. `avt` - very honest narrow virtual terminal

`avt` README one of the cleanest scoping statements in this whole ecosystem.
It explicitly says it covers:

- parser for ANSI-compatible terminal
- virtual screen buffers
- API for feeding text and querying screen/cursor

And explicitly says out of scope:

- input handling
- rendering

Это excellent architecture signal.

### Почему `avt` полезен

- it shows the exact seam between terminal emulation and host rendering
- it confirms that parser + screen buffer can be a useful standalone layer
- it is already used by multiple `asciinema` products

### Почему этого мало для вашего package

- input pipeline would still be yours
- product runtime and session lifecycle would still be yours
- no stronger signal yet that it should replace `alacritty_terminal` as default emulator core

## 4. `alacritty_terminal` - real reusable emulator core

`alacritty_terminal/src/lib.rs` is blunt and useful:

- it is a library for writing terminal emulators
- module split includes:
  - `event`
  - `event_loop`
  - `grid`
  - `selection`
  - `term`
  - `tty`
  - `vi_mode`
- it re-exports `Term` and `Grid`
- it also re-exports `vte`

This is a very healthy layering signal.

### What it tells us architecturally

- the parser layer is beneath the emulator core
- terminal state is a first-class object
- grid and selection are part of emulator concerns
- PTY event loop is adjacent, but not the product shell

Это exactly why `alacritty_terminal` feels like the safest default.

## 5. `termwiz` - powerful adjacent toolkit, not the same thing as emulator core

`termwiz` remains useful, but this pass made the role boundary clearer.

What it looks better for:

- terminal capabilities
- terminal interactions
- widgets/line-editor style helpers
- text/surface abstractions

What it should not be confused with:

- the main emulator core of your package

Итог:

⚠️ `termwiz` is a strong toolbox, not the cleanest primary `EmulatorPort`

## 6. `shadow-terminal` - strong donor for headless runtime surfaces

`shadow-terminal` README is unusually useful.
It explicitly positions itself as:

- fully-rendered terminal emulator in memory
- useful for E2E testing
- useful as basis for terminal multiplexers
- backed by `wezterm-term`

And exposes two very relevant modes:

- `ActiveTerminal`
- `SteppableTerminal`

This is an important product/runtime clue.

### Why this matters to your package

It suggests that headless terminal surfaces should likely be **first-class**:

- active/live mode for attached UI
- steppable/test mode for automation and verification

Это очень хорошо бьётся с уже найденными ideas from `termscope`, `termwright` and durable runtime research.

## 7. `crossterm` and `ratatui` - do not let host UI crates leak into the runtime core

This is the main anti-pattern of this pass.

## `crossterm`

`crossterm` is excellent at:

- terminal manipulation
- input events
- cursor/style/output control
- alternate screen
- raw mode

Но это **host interaction library**, not emulator core.

If you put `crossterm` into your universal runtime core, then:

- your core starts assuming one class of host terminal interactions
- your package becomes more app-shell-shaped
- the JS/Electron embedding story gets muddier

## `ratatui`

`ratatui` is a fantastic TUI app framework.
But architecturally it belongs to:

- standalone CLI app shell
- debugging tools
- internal operator consoles

It does **not** belong at the heart of embeddable terminal runtime truth.

🔥 Popularity is not the same thing as architectural fit.

## Recommended layered model

```text
Rust terminal package

  PTY layer
    -> portable-pty

  Emulator core layer
    -> alacritty_terminal
    -> optional future libghostty-vt adapter

  Headless screen / replay helpers
    -> vt100
    -> avt
    -> shadow-terminal style surfaces

  Runtime orchestration
    -> tokio owner-task model

  Public protocol / adapters
    -> daemon protocol
    -> Node/Electron adapter
    -> C ABI adapter

Host shells
  -> JS UI
  -> standalone Rust app
  -> other language hosts
```

## What I would choose now

Если выбирать прямо сейчас для package мирового уровня:

1. `portable-pty + alacritty_terminal` as default core
2. keep `libghostty-vt` as planned future alternate `EmulatorPort`
3. use `vt100` / `avt` / `shadow-terminal` patterns for headless snapshots, diffs, replay and testing
4. keep `crossterm` / `ratatui` strictly in leaf host apps, never in domain/runtime truth

## Final architectural rule

🔥 **Do not let the easiest-to-demo crate become the architectural center.**

Для embeddable terminal package центр должен быть:

- PTY lifecycle
- emulator state
- runtime orchestration
- protocol boundary

А parser bricks, headless helpers и host-side UI crates должны оставаться на своих слоях.

