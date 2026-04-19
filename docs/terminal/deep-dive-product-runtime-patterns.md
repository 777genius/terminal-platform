# Deep Dive: Product And Runtime Patterns

**Проверено**: 2026-04-19

Этот файл про то, что часто теряется между "engine research" и "feature architecture":

- search behavior
- output batching
- overlay/search chrome
- agent/session status
- resilience and diagnostics
- automation/session protocols

Это не foundation-only тема.  
Это про то, как terminal перестаёт быть просто emulator widget и становится useful product surface.

## 1. `wterm` - render discipline matters

### Какие исходники особенно полезны

- `packages/@wterm/dom/src/renderer.ts`
- `packages/@wterm/core/src/__tests__/websocket-transport.test.ts`

### Что стало понятнее

#### A. Dirty-row renderer - это не только perf, но и shape of API

`Renderer` у `wterm` не перерисовывает всё вслепую.  
Он:

- группирует cell runs по style
- отдельно рисует block characters and quadrants
- синхронизирует scrollback rows отдельно от active grid
- держит previous cursor position

Это важный reminder:

- renderer wants structured cell/state access
- renderer should have its own scrollback synchronization logic
- cursor and scrollback should not быть побочкой общего `innerHTML = ...`

#### B. Transport tests reveal intended guarantees

Тесты `websocket-transport.test.ts` фиксируют реальную contract surface:

- buffering before socket open
- flush on open
- reconnection after unexpected close
- no reconnect after explicit `close()`
- binary payloads become `Uint8Array`

🔥 Это полезно как design rule для нас:

если делать terminal transport, его expected behavior стоит формализовать тестами, а не только кодом.

### Что утащить как идею

- render API should expose enough structure for incremental updates
- scrollback sync is its own concern
- transport guarantees should be locked by small focused tests

---

## 2. `restty` - search and output are their own runtimes

### Какие исходники особенно полезны

- `src/runtime/create-runtime/search-runtime.ts`
- `src/runtime/pty-output-buffer.ts`
- `src/surface/pane-app-manager.ts`
- `src/surface/pane-search-ui.ts`

### Что стало понятнее

#### A. Search is incremental, not synchronous big-scan

`createRuntimeSearch` в `restty` делает search не одним blocking pass, а через:

- RAF scheduling
- `SEARCH_STEP_BUDGET = 64`
- `pending / complete / generation`
- viewport matches as separate read model
- explicit `markDirty()` and `handleWasmReset()`

🔥 Это очень сильная идея.

Большой terminal search лучше проектировать как runtime with progress, а не как "вызвал find и мгновенно всё нашёл".

#### B. PTY output buffering has dual thresholds

`createPtyOutputBufferController` буферизует output по двум условиям:

- idle timeout
- max timeout

Это простой, но очень полезный pattern:

- не флудить renderer слишком часто
- не задерживать вывод бесконечно под continuous stream

#### C. Search chrome sits above panes, not inside terminal core

`pane-app-manager.ts` и `pane-search-ui.ts` показывают, что built-in search UI:

- отдельный controller
- регистрируется per-pane
- слушает app search state callbacks
- имеет свой shortcut/open/close/focus behavior
- умеет style options and status formatting

Это хороший boundary:

- search logic может жить в terminal runtime
- search UI should stay in product shell layer

#### D. Every pane gets canvas + IME input + debug surface

Pane manager создаёт на каждый pane:

- `canvas`
- hidden `imeInput`
- `termDebugEl`

Это ещё раз подтверждает:

terminal pane - это не один canvas/div.  
Нормальный pane обычно включает несколько cooperating DOM surfaces.

### Что утащить как идею

- frame-budgeted incremental search
- dual-threshold output batching
- search UI controller above pane/app
- multi-surface pane composition

---

## 3. `Hermes IDE` - session intelligence must be state-machine driven

### Какие документы особенно полезны

- `ARCHITECTURE.md`
- `src/terminal/TerminalPool.ts`

### Что стало понятнее

#### A. Session phases are explicit across frontend and backend

Из `ARCHITECTURE.md` видно, что у них `SessionPhase` - first-class concept:

- `Creating`
- `Initializing`
- `ShellReady`
- `LaunchingAgent`
- `Idle`
- `Busy`
- `NeedsInput`
- `Closing`
- `Destroyed`

И это используется не ради красоты, а для:

- suggestion gating
- prompt/agent detection
- timeline updates
- UI status

#### B. Output analysis is a backend seam

Hermes явно выделяет:

- `ProviderAdapter`
- `OutputAnalyzer`
- `Execution Node`

То есть raw PTY output у них не сразу идёт в красивый UI смысл.  
Есть отдельный analysis layer, который:

- strips ANSI for analysis
- detects active AI provider
- parses token/tool/prompt semantics
- drives phase transitions

⚠️ Это сильный сигнал, если терминал должен быть agent-aware.

### Что утащить как идею

- explicit session phase machine
- backend output analysis seam
- provider-specific adapters instead of giant `if/else` parser

---

## 4. `cmux` - attention UX needs a host-level status bridge

### Какие документы особенно полезны

- `docs/notifications.md`

### Что стало понятнее

`cmux` не ограничивается "popup notify".  
У него есть связка:

- `notify`
- `set-status`
- `clear-status`
- notification panel
- macOS system notifications fallback
- injected env vars:
  - `CMUX_SOCKET_PATH`
  - `CMUX_TAB_ID`
  - `CMUX_PANEL_ID`

🔥 Это особенно полезно для agent workflows.

Нормальный attention system should allow:

- notify user
- persist lightweight status
- target specific pane/tab
- let tools know where they are running

### Что утащить как идею

- session/tab/pane status bridge
- fallback-friendly notification strategy
- shell-visible panel/session env vars

---

## 5. `Factory Floor` - resilience must be designed, not added later

### Какие документы особенно полезны

- `docs/terminal-resilience-design.md`
- `docs/terminal-spawning.md`

### Что стало понятнее

`Factory Floor` очень явно описывает, где terminal stack ломается:

- surface creation can fail silently
- layered wrappers can swallow errors
- tmux config/load errors can disappear
- launcher path can be invalid
- respawn can race

Их design rule очень полезен:

🔥 failures must become visible states with retry and diagnostics.

То есть для terminal feature нужны заранее:

- launch error state
- retry action
- surfaced command/diagnostics
- orphan cleanup
- no silent fallback without explanation

### Что утащить как идею

- visible launch failure states
- structured diagnostics per layer
- explicit non-goal: no hidden automatic retry loops

---

## 6. `termscope` - headless automation should feel like a real protocol

### Какие исходники особенно полезны

- `README.md`
- `src/session.zig`
- `src/snapshot.zig`

### Что стало понятнее

#### A. Session mode is plain JSON-lines RPC

`termscope session`:

- reads JSON-lines from stdin
- writes JSON-lines to stdout
- supports methods like:
  - `snapshot`
  - `type`
  - `press`
  - `wait_for_text`
  - `wait_for_idle`
  - `query`
  - `resize`
  - `close`

Это очень clean automation seam.

#### B. Snapshot is not only text

`snapshot.zig` показывает несколько output shapes:

- numbered text
- spans
- json
- html
- svg
- ansi

То есть headless terminal API становится сильно полезнее, когда умеет отдавать не одну "строку экрана", а несколько representations for different consumers.

### Что утащить как идею

- JSON-lines session protocol is enough for many automation cases
- multi-format snapshots give leverage for tests, AI and debugging

---

## 7. `zmx` - durable runtime can stay small if IPC is disciplined

### Какие исходники особенно полезны

- `README.md`
- `src/ipc.zig`
- `src/socket.zig`

### Что стало понятнее

#### A. IPC is binary and tagged, not overly abstract

`ipc.zig` defines concrete message tags:

- `Input`
- `Output`
- `Resize`
- `Detach`
- `Kill`
- `Info`
- `Init`
- `History`
- `Run`
- `Ack`
- `Write`
- `TaskComplete`

Это хороший reminder:

- durable local session runtime can use a very small binary protocol
- it does not need a huge framework to stay structured

#### B. Socket path safety is treated as real engineering, not trivia

`socket.zig` explicitly handles:

- session name validation
- path traversal rejection
- stale socket cleanup
- Unix socket path length limits
- dynamic max name length depending on socket dir

⚠️ Это хороший practical lesson.

Local terminal services get weird bugs from filesystem/socket edges surprisingly quickly.

### Что утащить как идею

- narrow IPC vocabulary
- explicit path/name validation
- session/socket identity safety

---

## Synthesis

После этого deep dive product/runtime side looks like this:

### 1. Search should be incremental

Not a blocking global scan.  
Prefer budgets, progress flags and viewport-level read models.

### 2. Output should be buffered intentionally

Not every PTY chunk should become immediate UI work.

### 3. Smart UX should obey terminal state

Alt-screen, session phase, focus, reconnect generation and pending output all matter.

### 4. Attention UX needs a lightweight host bridge

Notifications alone are not enough.  
Need statuses, identities and targeted delivery.

### 5. Resilience must become explicit UI state

Launch failures, stale reconnects, missing runtime pieces and wrapper errors should surface clearly.

### 6. Automation surface should be first-class

Even if not P0, a clean session/query/snapshot protocol creates long-term leverage.

## Sources

- [wterm](https://github.com/vercel-labs/wterm)
- [restty](https://github.com/wiedymi/restty)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
- [cmux notifications](https://github.com/manaflow-ai/cmux/blob/main/docs/notifications.md)
- [Factory Floor](https://github.com/alltuner/factoryfloor)
- [termscope](https://github.com/mwunsch/termscope)
- [zmx](https://github.com/neurosnap/zmx)
