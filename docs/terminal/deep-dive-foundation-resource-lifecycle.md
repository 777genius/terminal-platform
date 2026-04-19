# Deep Dive: Foundation Resource Lifecycle And Attach Semantics

**Проверено**: 2026-04-19

Этот файл не про high-level UX, а про более низкий слой:

- как foundation делят тяжёлые ресурсы между panes
- как переживают attach/reattach
- как устроен input/output hot path
- где живут terminal-generated responses и shell compatibility tricks

Именно этот слой часто определяет, станет terminal feature надёжной subsystem или набором хрупких pane hacks.

## 1. `wterm` - очень чистый embed-first foundation

### Какие исходники особенно полезны

- `packages/@wterm/core/src/transport.ts`
- `packages/@wterm/core/src/wasm-bridge.ts`
- `packages/@wterm/dom/src/renderer.ts`
- `packages/@wterm/dom/src/input.ts`
- `packages/@wterm/dom/src/wterm.ts`

### Что стало понятнее

#### A. Transport intentionally tiny and frontend-friendly

`transport.ts` у `wterm` surprisingly маленький и дисциплинированный:

- WebSocket transport держит pre-open buffer
- при reconnect использует exponential backoff
- `send()` умеет принимать и `string`, и `Uint8Array`
- onmessage сразу делит payload на text vs binary

Это говорит о важной позиции проекта:

- transport должен быть thin
- session semantics не зашиваются сюда
- foundation не пытается стать session manager

#### B. Wasm bridge exposes more than "just write and read cells"

`wasm-bridge.ts` даёт не только grid/cursor:

- `cursorKeysApp()`
- `bracketedPaste()`
- `usingAltScreen()`
- `getTitle()`
- `getResponse()`
- `getScrollbackCount()`
- `getScrollbackLineLen()`

🔥 Особенно полезен `getResponse()`.

Это отдельный канал terminal-generated replies:

- DA responses
- cursor/status replies
- другие control-sequence ответы

То есть foundation уже различает:

- user/app input
- terminal-generated response output back to PTY peer

Это очень хороший seam.

#### C. DOM renderer is more sophisticated than "text nodes in divs"

`renderer.ts` показывает, почему `wterm` выглядит жизнеспособно даже как DOM renderer:

- dirty rows обновляются адресно
- block and quadrant characters не рендерятся как обычный text fallback
- для них используются CSS gradients и специальные `term-block` spans
- normal styled text собирается runs-ами, а не по одному cell span на символ

Это важный вывод:

DOM renderer здесь не "простенький", а осознанно оптимизированный под terminal glyph classes.

#### D. Input layer is built around hidden textarea, not brittle keydown hacks

`input.ts` делает несколько правильных вещей:

- hidden textarea as the true focus/input bridge
- composition events for IME
- bracketed paste awareness
- app cursor mode awareness
- selection-friendly copy behavior
- Cmd/Ctrl behavior аккуратно не ломает platform shortcuts

Это ещё раз подтверждает: хороший terminal input layer почти всегда должен жить отдельно от render surface.

#### E. WTerm class keeps renderer scheduling and scroll semantics small

`wterm.ts` полезен как reference для минимального host shell:

- `requestAnimationFrame` render scheduling
- resize через `ResizeObserver`
- `scrollToBottom` only if user was already at bottom
- focus restore only when selection collapsed

Это хороший "small shell around foundation" pattern.

### Что утащить как идею

- thin transport, no fake session manager in foundation
- separate terminal-generated response channel
- DOM renderer may still need glyph-specific rendering strategies
- hidden textarea input bridge as default
- scroll-to-bottom should depend on pre-write viewport state

---

## 2. `restty` - foundation already thinks in shared sessions and heavy runtime pieces

### Какие исходники особенно полезны

- `src/runtime/create-runtime.ts`
- `src/runtime/session.ts`
- `src/runtime/pty-output-buffer.ts`
- `tests/plugin-system.test.ts`

### Что стало понятнее

#### A. One app session can own shared heavy resources

`session.ts` у `restty` очень полезен:

- `createResttyAppSession()` lazily loads WASM
- lazily initializes WebGPU core
- owns shared font resource store
- allows WASM log listeners
- many panes can share one session

🔥 Это сильный паттерн, который легко недооценить.

Нужно различать:

- PTY/session runtime
- foundation resource session

Один terminal pane не обязан владеть своим отдельным WASM, WebGPU core и font cache.

#### B. Main runtime constructor is already a subsystem boundary

`create-runtime.ts` показывает, что `restty` реально ближе к platform layer, чем к simple renderer:

- input hooks
- font/runtime helpers
- search runtime
- reporting runtime
- interaction runtime
- kitty render runtime
- render tick orchestration
- shader stage runtime
- PTY output buffering

Это значит, что проект уже internalized идею:

- terminal foundation = набор coordinated runtimes
- а не один giant class с event listeners

#### C. PTY output buffering is intentionally tiny and composable

`pty-output-buffer.ts` сам по себе простой, но полезный:

- `idleMs`
- `maxMs`
- `queue / flush / cancel / clear`

Сила тут именно в форме:

- buffer controller можно тестировать отдельно
- его легко встраивать в разные runtime shells

#### D. Plugin tests reveal a serious interception seam

`tests/plugin-system.test.ts` полезен не только тестами.

По нему видно, что `restty` уже рассчитывает на:

- `beforeInput`
- `beforeRenderOutput`
- pane-scoped behavior
- desktop notification callbacks
- shader stage replacement per pane

Это хороший reminder:

если extension seam не заложить рано, потом product-команда начнёт вшивать hoc-логику прямо в renderer/runtime core.

### Что утащить как идею

- separate foundation resource session
- lazy WASM/GPU/font loading shared across panes
- keep runtime split into testable controllers
- output buffer controller as standalone primitive
- reserve explicit input/output interception seams

---

## 3. `zmx` - session runtime donor даже сильнее, чем кажется снаружи

### Какие исходники особенно полезны

- `src/main.zig`
- `src/ipc.zig`
- `src/util.zig`

### Что стало понятнее

#### A. One daemon per session is a deliberate architecture choice

`main.zig` прямо формулирует важную идею:

- на каждую session поднимается отдельный daemon
- IPC не надо multiplex-ить тегами session id внутри одного giant server
- crash blast radius меньше
- socket ownership и cleanup проще

🔥 Это очень сильный runtime pattern для durable sessions.

#### B. Attach restore is careful about resize timing

`handleInit()` у `zmx` особенно полезен:

- terminal state serializes **before** resize
- это делается только на re-attach
- причина - resize triggers reflow and can move cursor
- shell redraw after SIGWINCH приходит позже
- OSC `133;A` patch-ится через `rewritePromptRedraw(... redraw=0)`

Это один из лучших найденных practical patterns для correct restore semantics.

#### C. Attach/detach hygiene is treated as correctness work

`attach()` делает много аккуратных вещей:

- skips tty mode setup if stdin is not a TTY
- uses `TCSAFLUSH`
- disables `Ctrl+\\` SIGQUIT to reclaim it as detach key
- clears screen before attach
- on detach restores mouse, bracketed paste, focus, alt-screen and kitty keyboard modes
- intentionally does **not** clear screen on detach to avoid breaking outer restore semantics

Это хороший reminder:

attach/detach - это не просто "соединились с сокетом", а целый compatibility boundary with outer terminal.

#### D. Headless daemon still behaves like a terminal peer

В `daemonLoop()` очень сильный кусок:

- PTY output всегда скармливается в `ghostty_vt.Terminal`
- даже когда клиентов нет
- если клиентов нет, daemon сам отвечает на terminal queries через `respondToDeviceAttributes`

🔥 Это очень полезная идея.

Detached/headless runtime всё равно должен оставаться terminal-aware, иначе shell/TUI behavior начнёт деградировать между attach cycles.

#### E. Writes are buffered, not best-effort dropped

У `zmx` есть отдельный `pty_write_buf` и non-blocking poll-driven flush.

Это лучше, чем naïve direct writes с потерей на `EAGAIN`.

По сути это ещё одно подтверждение:

- interactive PTY runtime needs explicit write buffering
- write ordering and backpressure are not optional polish

#### F. IPC protocol is narrow but expressive

`ipc.zig` хорош тем, что протокол маленький, но достаточный:

- `Input`
- `Output`
- `Resize`
- `Detach`
- `Info`
- `Init`
- `History`
- `Run`
- `Ack`
- `Switch`
- `Write`
- `TaskComplete`

Это хороший пример того, что durable session runtime не требует гигантского RPC surface.

### Что утащить как идею

- one daemon per session as blast-radius limiter
- serialize restore snapshot before resize on reattach
- detached runtime should still answer terminal queries
- explicit PTY write buffer instead of direct best-effort writes
- keep session IPC narrow but task-aware

---

## 4. Practical synthesis

После этого deep dive видно, что terminal foundation и session runtime надо разводить ещё чётче:

### A. There are at least two kinds of "session"

- foundation resource session
- PTY/process session

Их нельзя автоматически склеивать в один object.

### B. Reattach correctness depends on ordering

Сначала:

- snapshot/serialize
- потом resize/reflow-sensitive actions
- потом attach replay

Иначе теряются cursor position, prompt lines или visible frame.

### C. Detached mode should still preserve terminal semantics

Если UI не attached, runtime всё равно может нуждаться в:

- query responses
- VT state tracking
- scrollback/snapshot upkeep
- task completion markers

### D. Small protocol beats giant abstraction

И `wterm`, и `zmx`, и даже куски `restty` подтверждают одно и то же:

- узкие explicit seams устойчивее, чем large magical API

## Sources

- [wterm](https://github.com/vercel-labs/wterm)
- [restty](https://github.com/wiedymi/restty)
- [zmx](https://github.com/neurosnap/zmx)
