# Deep Dive: Code Architecture Patterns

**Проверено**: 2026-04-19

Этот файл фиксирует уже не market-level выводы, а **конкретные code-level patterns** из самых интересных terminal projects.

Фокус:

- как реально разрезаны foundation layers
- где у проектов transport/runtime/surface границы
- какие low-level patterns стоит утащить в нашу feature
- где заканчивается emulator и начинается product shell

## 1. `wterm`

### Какие исходники особенно важны

- `packages/@wterm/core/src/transport.ts`
- `packages/@wterm/core/src/wasm-bridge.ts`
- `packages/@wterm/dom/src/wterm.ts`
- `packages/@wterm/dom/src/input.ts`

### Что видно по коду

#### A. Transport intentionally tiny

`WebSocketTransport` в `@wterm/core` очень маленький, но архитектурно аккуратный:

- reconnect с exponential backoff
- `_buffer` для данных, пришедших до открытия сокета
- единый `onData / onOpen / onClose / onError`
- бинарный режим по умолчанию

Это хороший reminder:

- transport не обязан знать UI
- transport не обязан знать PTY semantics
- transport может быть очень маленьким и всё равно полезным

#### B. `WasmBridge` - это уже introspection API, а не просто `write()`

`WasmBridge` expose-ит:

- `getCell`
- `isDirtyRow`
- `getCursor`
- `cursorKeysApp`
- `bracketedPaste`
- `usingAltScreen`
- `getTitle`
- `getResponse`
- `getScrollbackCount`
- `getScrollbackCell`

🔥 Это очень важная идея.

Foundation becomes much stronger, когда из него можно читать:

- render-facing state
- input mode state
- terminal responses
- title changes
- scrollback

То есть хороший terminal foundation должен быть не только sink for bytes, но и **state inspection surface**.

#### C. DOM terminal orchestrator lives in one explicit object

`WTerm` в `@wterm/dom` сам собирает:

- `WasmBridge`
- `Renderer`
- `InputHandler`
- `ResizeObserver`
- `requestAnimationFrame` render scheduling

И важная деталь:

- `write()` only mutates bridge state
- actual DOM render deferred to next animation frame

Это хороший паттерн для нас:

- writes should not equal immediate paint
- renderer tick должен быть отдельным шагом

#### D. Input layer сделан через hidden textarea, не через фантазии

`InputHandler` использует скрытый `textarea` и поддерживает:

- composition events
- IME-safe input
- bracketed paste
- app cursor mode
- selection-friendly `Ctrl/Cmd+C`
- native-ish paste/focus behavior

⚠️ Это один из тех слоёв, которые легко недооценить.

Именно такие детали отличают нормальный IDE-like terminal от "вроде печатает символы".

### Что утащить как идею

- маленький buffered transport с reconnect
- introspection-rich bridge API
- explicit render scheduler
- serious IME/composition input layer

### Что не даёт сам `wterm`

- PTY lifecycle
- session runtime
- tabs/splits
- attention system
- persistence

---

## 2. `restty`

### Какие исходники особенно важны

- `src/pty/pty.ts`
- `src/runtime/create-runtime.ts`
- `src/surface/restty.ts`
- `docs/plugins.md`

### Что видно по коду

#### A. PTY transport сделан как typed protocol, а не "просто WebSocket"

В `src/pty/pty.ts` transport знает про:

- lifecycle states: `idle / connecting / connected / closing`
- `connectId` для отбрасывания stale socket callbacks
- binary data, `Blob`, string payloads
- structured server messages:
  - `status`
  - `error`
  - `exit`

🔥 `connectId` особенно хорошая маленькая идея.

Если terminal reconnect-ится или пользователь быстро переподключает pane/session, stale async callbacks нельзя пускать в живой runtime.

#### B. `create-runtime.ts` - это уже целый integration shell

Файл огромный не случайно.  
Там собирается всё, что обычно недооценивают в terminal stack:

- font runtime
- text shaping
- clipboard integration
- IME
- search runtime
- render ticks
- kitty graphics / overlays
- PTY input runtime
- interaction runtime
- runtime reporting/debug tooling

Это очень хороший architectural signal:

⚠️ IDE-like terminal - это не "renderer + PTY".  
Между ними есть большой runtime layer.

#### C. `Restty` class - это уже surface coordinator

`src/surface/restty.ts` показывает очень полезную форму:

- top-level `Restty` object
- inside it:
  - pane manager
  - shader ops
  - plugin ops
- global operations are synchronized to panes

Это удачный пример, как строить terminal feature не вокруг одного widget, а вокруг **workspace surface coordinator**.

#### D. Plugin contract сделан как first-class primitive

`docs/plugins.md` и runtime code показывают, что plugin API у `restty` допускает:

- input interceptors
- output interceptors
- lifecycle hooks
- render hooks
- GPU shader stages

Это необычно сильный seam.

Для нашей feature это не означает "сразу писать plugin platform", но означает:

- foundation boundary should stay extensible
- intercept/observe hooks полезно закладывать заранее

### Что утащить как идею

- typed PTY protocol
- stale-connection guards
- explicit runtime layer between PTY and renderer
- top-level terminal surface coordinator
- interceptor/hook seams

### Что не надо путать

`restty` всё ещё не решает за нас:

- full app chrome
- session persistence
- workspace model
- command history / execution timeline

---

## 3. `ghostty-web`

### Какие исходники особенно важны

- `lib/index.ts`
- `lib/terminal.ts`

### Что видно по коду

#### A. Shared engine bootstrap is explicit

`init()` создаёт shared module-level `Ghostty` instance, а `Terminal` использует его потом как foundation.

Это хороший pattern, если foundation тяжёлый:

- один engine bootstrap per process
- много terminal instances поверх него

#### B. xterm-compatible wrapper - это большой product-adaptation layer

`Terminal` в `ghostty-web` - не thin wrapper.  
Там есть:

- event emitters
- addon system
- selection manager
- link detector + providers
- canvas renderer
- input handler
- buffer namespace
- smooth scrolling state
- runtime mutable options через `Proxy`
- internal write queue

🔥 Это полезный reminder:

если выбирать stronger engine, всё равно нужен толстый adapter layer, чтобы он стал удобной app-facing API.

#### C. Runtime option mutation handled explicitly

Через `Proxy` options runtime changes переводятся в:

- resize
- font recalculation
- cursor style updates
- partial renderer updates

Это гораздо лучше, чем silently ignoring option changes или forcing full re-create every time.

### Что утащить как идею

- shared engine bootstrap
- additive compatibility wrapper, not engine leakage into UI
- explicit mutable-options bridge
- addon/link/selection boundaries as first-class parts of API

### Что не снимает `ghostty-web`

- session model
- persistence
- workspace UX
- command/search/history surfaces

---

## 4. `Hermes IDE`

### Какие исходники особенно важны

- `src/terminal/TerminalPool.ts`

### Что видно по коду

#### A. Terminal pool is above UI

`TerminalPool` живёт module-level и выступает как orchestrator:

- create / attach / detach
- session phase
- cwd tracking
- history provider
- suggestion subscribers
- focus tracking

Это подтверждает сильный architectural rule:

terminal runtime нельзя привязывать к lifecycle одного React pane.

#### B. Intelligence gated by session phase

В input pipeline проверяются:

- `idle`
- `shell_ready`
- overlay visibility
- alternate buffer state

И overlay немедленно dismiss-ится, когда terminal уходит в alternate screen.

🔥 Это очень хороший UX rule.

Нельзя держать smart overlays поверх `vim`, `less`, Claude Code и других TUI просто потому, что "overlay technically ещё открыт".

#### C. Native SIGINT bridge

На macOS Hermes отдельно слушает native event и форвардит `\x03` в активную session.

Это полезный reminder:

- terminal input не всегда живёт полностью в web layer
- host-specific key handling иногда нужно решать на desktop shell уровне

#### D. Input buffer model very deliberate

Код аккуратно обрабатывает:

- surrogate pairs
- paste payloads with control chars
- history logging on Enter
- clear/dismiss on `Ctrl-C`, `Ctrl-U`, `Escape`

Это показывает, что command UX above terminal требует свой маленький input state machine.

### Что утащить как идею

- module-level terminal pool
- session phase state machine
- overlay suppression on alt-screen
- host-level key bridge for problematic shortcuts

---

## 5. `terminalcp`

### Какие исходники особенно важны

- `src/terminal-manager.ts`
- `src/terminal-server.ts`

### Что видно по коду

#### A. Session server remains small

Несмотря на background daemon, кодовая форма остаётся компактной:

- `TerminalManager` держит PTY + headless terminal state
- `TerminalServer` держит local socket API and subscriptions

Это хороший сигнал, что durable session runtime не обязан быть огромным subsystem.

#### B. Ordering is treated as a first-class problem

`WriteQueue` используется отдельно для:

- writes into headless terminal
- writes into PTY

Это надо утащить почти напрямую.

#### C. Attach returns initial state, events continue live

`attach` отдаёт:

- `cols`
- `rows`
- `rawOutput`

А дальше server already pushes live output events to subscribed clients.

Это очень практичный pattern для UI attach/detach.

### Что утащить как идею

- attach with initial state snapshot
- live event stream after attach
- separate subscriber tracking per session
- explicit ordering queues

---

## 6. `Factory Floor`

### Какие документы особенно важны

- `docs/terminal-spawning.md`

### Что видно по дизайну

#### A. Terminal launch is treated as environment contract

Каждый workstream terminal получает стабильные env vars:

- `FF_PROJECT`
- `FF_WORKSTREAM`
- `FF_PROJECT_DIR`
- `FF_WORKTREE_DIR`
- `FF_PORT`
- `FF_DEFAULT_BRANCH`

Это очень сильный pattern для multi-worktree workflows.

#### B. Wrapping chain is explicit and inspectable

Док описывает полную цепочку:

- user action
- run command builder
- optional launcher
- optional tmux wrapper
- final Ghostty command

🔥 Это именно тот уровень прозрачности, который нужен и нам.

Когда terminal feature становится сложнее, command chain нельзя прятать в "какой-то helper".

#### C. Surface IDs are deterministic

Agent/setup/run surfaces derive IDs deterministically from workstream identity and role.

Это полезно для:

- restore
- reconnect
- UI state mapping
- telemetry/debugging

### Что утащить как идею

- workspace/workstream env contract
- explicit command wrapping chain
- deterministic surface/session identities

---

## 7. `cmux`

### Какие документы особенно важны

- `docs/remote-daemon-spec.md`

### Что видно по дизайну

#### A. Remote/browser story built as transport layer, not terminal hack

`cmux` строит remote browsing and terminal reuse через:

- daemon handshake
- session RPC
- transport-scoped local proxy broker
- pushed proxy stream events

Это очень правильный shape.

Если когда-то захотим remote/sandbox terminal, его нельзя делать как ad-hoc hacks around panes.

#### B. Multi-attach semantics are explicit

Док фиксирует правило:

- effective PTY size = minimum across attached clients

Это tmux-like `smallest screen wins`.

Даже если нам это не нужно в P0, семантику multi-attach лучше продумать заранее.

### Что утащить как идею

- transport-scoped runtime/broker thinking
- explicit attachment semantics
- remote feature as daemon/session architecture, not as UI-only feature

---

## Synthesis

После code-level разбора картина стала ещё яснее:

### 1. Foundation must expose state, not only rendering

Это видно у `wterm`, `ghostty-web`, `restty`.

### 2. Session runtime must be separate from pane lifecycle

Это видно у `Hermes`, `terminalcp`, `zmx`, `cmux`.

### 3. IDE UX needs small but explicit state machines

Например:

- session phase
- overlay visibility
- alt-screen suppression
- reconnect generation / connect token
- ordered write queues

### 4. Product shell needs deterministic identity

Это видно у `Factory Floor`, `zmx`, `cmux`.

Нужны стабильные:

- session IDs
- surface IDs
- env-injected identities
- attach semantics

## Sources

- [wterm](https://github.com/vercel-labs/wterm)
- [restty](https://github.com/wiedymi/restty)
- [ghostty-web](https://github.com/coder/ghostty-web)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
- [terminalcp](https://github.com/badlogic/terminalcp)
- [Factory Floor](https://github.com/alltuner/factoryfloor)
- [cmux](https://github.com/manaflow-ai/cmux)
